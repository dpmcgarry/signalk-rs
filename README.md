# signalk-rs

[![CI (dev)](https://github.com/dpmcgarry/signalk-rs/actions/workflows/ci.yml/badge.svg?branch=dev)](https://github.com/dpmcgarry/signalk-rs/actions/workflows/ci.yml?query=branch%3Adev)

A [Signal K](https://signalk.org/) server written in Rust.

This is a fork of [herostrat/signalk-rs](https://github.com/herostrat/signalk-rs). The `dev` branch collects work that is headed back upstream as pull requests.

## Features

- REST API (v1 + v2) and WebSocket streaming with subscriptions
- NMEA 0183 (TCP / serial) and NMEA 2000 input, optional output
- AIS target tracking, anchor alarm, derived data, tracks, history (SQLite)
- Plugin system: Rust in-process, Node.js bridge, standalone binaries
- Serves Signal K webapps (Admin UI, Instrument Panel, KIP) as static files

Targets Signal K spec 1.7.0.

## Build

System packages (Debian / Ubuntu):

```sh
sudo apt install pkg-config libudev-dev libssl-dev
```

`libssl-dev` is only needed for `cargo test`.

```sh
cargo build -p signalk-server
```

Optional cargo features on `signalk-server`:

| Feature | Adds |
|---|---|
| `simulator` | Fake sensor data for development |
| `nmea2000` | NMEA 2000 receive + send |
| `nmea0183-output` | NMEA 0183 output over TCP |
| `autopilot-experimental` | Autopilot control loop |

## Run

Config is TOML. The server looks in this order:

1. `$SIGNALK_CONFIG_FILE`
2. `/etc/signalk-rs/config.toml`
3. `signalk-rs.toml` in the working directory

Start from `signalk-rs.example.toml`. For local use, set `data_dir` and the `[internal]` socket paths to directories you can write to.

```sh
SIGNALK_CONFIG_FILE=path/to/config.toml RUST_LOG=info \
  cargo run -p signalk-server --features simulator
```

Then try:

```sh
curl -s localhost:3000/signalk
curl -s localhost:3000/signalk/v1/api/vessels/self
websocat ws://localhost:3000/signalk/v1/stream
```

### Web UIs

Webapps are npm packages served as static files by the Rust server. No Node.js process runs.

```sh
cd webapps && npm install --omit=dev
```

Restart the server, then open `/admin/`, `/@signalk/instrumentpanel/` or `/@mxtommy/kip/`.

## Test

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Layout

| Path | Contents |
|---|---|
| `crates/signalk-types` | Data model, serde, paths |
| `crates/signalk-store` | In-memory state, sources, subscriptions |
| `crates/signalk-server` | HTTP / WebSocket server, auth, APIs |
| `crates/signalk-plugin-api` | Plugin traits |
| `crates/signalk-internal` | Unix-socket bridge for external plugins |
| `crates/signalk-sqlite` | Shared SQLite storage |
| `crates/plugins/*` | Built-in plugins |
| `bridge/` | Node.js bridge for JS plugins (optional) |
| `docs/` | Plugin architecture and guides |

## License

Apache-2.0
