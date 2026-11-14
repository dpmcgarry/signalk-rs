//! Test helpers: build a ready-to-use app and make requests against it.
//!
//! Each test file includes this module and uses only the subset it needs.
//! `dead_code` is suppressed because the full helper surface is intentionally
//! shared across multiple test compilation units.
#![allow(dead_code)]

use axum::{Router, body::Body, http::Request, response::Response};
use signalk_server::{ServerState, build_router, config::ServerConfig};
use signalk_store::store::SignalKStore;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceExt;

/// Build a test app with default config and empty store.
pub fn test_app() -> Router {
    let config = ServerConfig::default();
    let state = ServerState::new(config);
    build_router(state, &[])
}

/// Build a test app AND return the `Arc<RwLock<SignalKStore>>` for data injection.
///
/// Tests inject deltas with `store.write().await.apply_delta(...)` before making
/// HTTP requests, verifying the full store → REST pipeline.
#[allow(dead_code)]
pub fn test_app_with_store() -> (Router, Arc<RwLock<SignalKStore>>) {
    let config = ServerConfig::default();
    let state = ServerState::new(config);
    let store = state.store.clone();
    (build_router(state, &[]), store)
}

/// Build a test app AND return the self vessel URI for assertions.
#[allow(dead_code)]
pub fn test_app_with_uri() -> (Router, String) {
    let config = ServerConfig::default();
    let state = ServerState::new(config);
    let self_uri = state.config_store.vessel_uuid();
    (build_router(state, &[]), self_uri)
}

/// Make a GET request and return the parsed JSON body + status code.
pub async fn get(app: Router, uri: &str) -> (u16, serde_json::Value) {
    let response = app
        .oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    response_json(response).await
}

/// Make a POST request with JSON body.
pub async fn post_json(
    app: Router,
    uri: &str,
    body: serde_json::Value,
) -> (u16, serde_json::Value) {
    let response = app
        .oneshot(
            Request::post(uri)
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    response_json(response).await
}

/// Make a PUT request with JSON body.
#[allow(dead_code)]
pub async fn put_json(app: Router, uri: &str, body: serde_json::Value) -> (u16, serde_json::Value) {
    let response = app
        .oneshot(
            Request::put(uri)
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    response_json(response).await
}

/// Build a test app using a specific data directory (for applicationData tests).
#[allow(dead_code)]
pub fn test_app_with_data_dir(data_dir: &std::path::Path) -> Router {
    let config = ServerConfig {
        data_dir: data_dir.to_string_lossy().to_string(),
        ..ServerConfig::default()
    };
    let state = ServerState::new(config);
    build_router(state, &[])
}

/// Build a test app with a pre-registered PUT handler pointing to a given plugin.
///
/// `handler_path` is a dot-notation path pattern, e.g. `"steering.autopilot.target.*"`.
/// `plugin_id` is the plugin identifier, e.g. `"test-plugin"`.
/// `bridge_socket` is the UDS socket path the server will try to forward to.
#[allow(dead_code)]
pub fn test_app_with_handler(handler_path: &str, plugin_id: &str, bridge_socket: &str) -> Router {
    use signalk_server::config::InternalSettings;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    let config = ServerConfig {
        internal: InternalSettings {
            transport: "uds".to_string(),
            uds_rs_socket: "/tmp/test-rs.sock".to_string(),
            uds_bridge_socket: bridge_socket.to_string(),
            http_rs_port: 3001,
            http_bridge_port: 3002,
            bridge_token: String::new(),
        },
        ..ServerConfig::default()
    };
    let test_db = signalk_sqlite::Database::open_in_memory().unwrap();
    let test_conn = Arc::new(std::sync::Mutex::new(test_db.into_conn()));
    let config_store = Arc::new(signalk_server::config_store::ConfigStore::new(
        test_conn,
        &signalk_server::config::SeedConfig::default(),
    ));
    let (store, _rx) = SignalKStore::new(config_store.vessel_uuid());
    let put_handlers = Arc::new(RwLock::new(HashMap::from([(
        handler_path.to_string(),
        plugin_id.to_string(),
    )])));
    let plugin_routes: Arc<RwLock<HashMap<String, String>>> = Arc::new(RwLock::new(HashMap::new()));
    let route_table = Arc::new(signalk_server::plugins::routes::PluginRouteTable::new());
    let put_handler_registry = Arc::new(signalk_server::plugins::host::PutHandlerRegistry::new());
    let plugin_manager = signalk_server::plugins::manager::PluginManager::new(
        store.clone(),
        route_table.clone(),
        put_handler_registry.clone(),
        put_handlers.clone(),
        plugin_routes.clone(),
        Arc::new(signalk_server::plugins::delta_filter::DeltaFilterChain::new()),
        Arc::new(RwLock::new(signalk_server::webapps::WebappRegistry::new())),
        std::path::PathBuf::from("/tmp/signalk-test/data"),
    );
    let resource_providers = Arc::new(signalk_server::resources::ResourceProviderRegistry::new(
        Arc::new(signalk_server::resources::FileResourceProvider::new(
            std::path::PathBuf::from("/tmp/signalk-test/resources"),
        )),
    ));
    let history_manager = signalk_server::history::HistoryManager::new_in_memory(
        signalk_server::history::HistoryConfig::default(),
    );
    let state = signalk_server::ServerState::new_shared(
        config,
        store,
        put_handlers,
        plugin_routes,
        put_handler_registry,
        route_table,
        Arc::new(tokio::sync::Mutex::new(plugin_manager)),
        Arc::new(RwLock::new(
            signalk_server::plugins::registry::PluginRegistry::new(),
        )),
        Arc::new(RwLock::new(signalk_server::webapps::WebappRegistry::new())),
        resource_providers,
        signalk_server::autopilot::AutopilotManager::new(),
        history_manager,
        Arc::new(signalk_server::notifications::NotificationManager::new()),
        None,
        config_store,
    );
    build_router(state, &[])
}

/// Build a test app AND return the Arc<ServerState> for direct state access.
///
/// Useful for admin API tests that need to register plugins in the PluginManager
/// or manipulate the PluginRegistry.
#[allow(dead_code)]
pub fn test_app_with_state() -> (Router, Arc<ServerState>) {
    let config = ServerConfig::default();
    let state = ServerState::new(config);
    let router = build_router(state.clone(), &[]);
    (router, state)
}

/// Make a POST request with an empty body.
#[allow(dead_code)]
pub async fn post_empty(app: Router, uri: &str) -> (u16, serde_json::Value) {
    let response = app
        .oneshot(
            Request::post(uri)
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    response_json(response).await
}

/// Make a GET request with a Bearer token.
#[allow(dead_code)]
pub async fn get_auth(app: Router, uri: &str, token: &str) -> (u16, serde_json::Value) {
    let response = app
        .oneshot(
            Request::get(uri)
                .header("authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    response_json(response).await
}

async fn response_json(response: Response) -> (u16, serde_json::Value) {
    let status = response.status().as_u16();
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let json: serde_json::Value = if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or_else(|_| {
            serde_json::Value::String(String::from_utf8_lossy(&bytes).to_string())
        })
    };
    (status, json)
}

/// Root of the vendored SignalK schema tree (`<workspace>/tests/schemas/`).
///
/// Populated by `scripts/update-schemas.sh`; the version is in `.schema-version`.
pub fn schema_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/schemas")
        .canonicalize()
        .expect("tests/schemas/ not found — run scripts/update-schemas.sh")
}

/// Load a SignalK JSON schema by its path relative to `tests/schemas/`,
/// without the `.json` suffix (e.g. `"delta"`, `"messages/put"`).
///
/// Panics if the file is missing or is not valid JSON.
pub fn load_schema(name: &str) -> serde_json::Value {
    let path = schema_root().join(format!("{name}.json"));
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("schema '{name}' not found at {}: {e}", path.display()));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("schema '{name}' is not valid JSON: {e}"))
}

/// Resolves every `$ref` from the vendored schema tree. Never touches the
/// network: an unknown URI is an error, so a missing file fails the test.
struct LocalSchemaRetriever {
    root: std::path::PathBuf,
}

impl jsonschema::Retrieve for LocalSchemaRetriever {
    fn retrieve(
        &self,
        uri: &jsonschema::Uri<String>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let uri = uri.as_str();
        let uri = uri.split('#').next().unwrap_or(uri);
        // SignalK ids look like https://signalk.org/specification/<ver>/schemas/<rel>.
        // The version in the id lags the release, so match on `/schemas/` only.
        // GeoJSON schemas keep their upstream json-schema.org ids.
        let rel = if let Some(i) = uri.rfind("/schemas/") {
            uri[i + "/schemas/".len()..].to_string()
        } else if let Some(r) = uri.strip_prefix("http://json-schema.org/geojson/") {
            format!("external/geojson/{r}")
        } else {
            return Err(format!("refusing to fetch non-local schema: {uri}").into());
        };
        let path = self.root.join(&rel);
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("schema {uri} -> {}: {e}", path.display()))?;
        Ok(serde_json::from_str(&content)?)
    }
}

/// Compile a named SignalK schema with all references resolved locally.
/// Panics if it does not compile.
pub fn compile_schema(name: &str) -> jsonschema::Validator {
    let schema = load_schema(name);
    jsonschema::options()
        .with_retriever(LocalSchemaRetriever {
            root: schema_root(),
        })
        .build(&schema)
        .unwrap_or_else(|e| panic!("schema '{name}' failed to compile: {e}"))
}

/// Validate a value against a named SignalK schema and return one line per
/// error (empty when valid).
pub fn schema_errors(name: &str, value: &serde_json::Value) -> Vec<String> {
    compile_schema(name)
        .iter_errors(value)
        .map(|e| format!("{e} (at {})", e.instance_path))
        .collect()
}

/// Assert that a JSON value conforms to a named SignalK schema.
/// Panics with every validation error if it does not, and also if the schema
/// is missing or does not compile — validation is never silently skipped.
pub fn assert_valid_schema(name: &str, value: &serde_json::Value) {
    let errors = schema_errors(name, value);
    if !errors.is_empty() {
        panic!(
            "JSON Schema '{name}' validation failed:\n  - {}\n\nValue:\n{}",
            errors.join("\n  - "),
            serde_json::to_string_pretty(value).unwrap()
        );
    }
}
