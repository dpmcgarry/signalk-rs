#!/usr/bin/env bash
# Update the pinned SignalK JSON schemas used by conformance tests.
#
# Fetches the full schema tree (top-level, groups/, messages/, external/) so
# that every cross-file `$ref` resolves locally. Tests never hit the network.
#
# Usage:
#   ./scripts/update-schemas.sh              # use version from .schema-version
#   ./scripts/update-schemas.sh 1.8.6        # explicit version
#
# After running, review diffs and commit:
#   git diff tests/schemas/
#   git add tests/schemas/ .schema-version && git commit -m "chore: update schemas to vX.Y.Z"

set -euo pipefail

VERSION_FILE="$(dirname "$0")/../.schema-version"
SCHEMA_DIR="$(dirname "$0")/../tests/schemas"
BASE_URL="https://raw.githubusercontent.com/SignalK/specification"

# Determine target version
if [[ $# -ge 1 ]]; then
    VERSION="$1"
else
    VERSION="$(cat "$VERSION_FILE" 2>/dev/null || echo "")"
    if [[ -z "$VERSION" ]]; then
        echo "Error: no version specified and .schema-version not found."
        echo "Usage: $0 <version>  (e.g. $0 1.8.6)"
        exit 1
    fi
fi

# Every file under schemas/ in the SignalK specification repo.
SCHEMAS=(
    aircraft
    aton
    definitions
    delta
    discovery
    hello
    sar
    signalk
    vessel
    external/geojson/bbox
    external/geojson/crs
    external/geojson/geojson
    external/geojson/geometry
    groups/communication
    groups/design
    groups/electrical
    groups/environment
    groups/navigation
    groups/notifications
    groups/performance
    groups/propulsion
    groups/resources
    groups/sails
    groups/sensors
    groups/sources
    groups/steering
    groups/tanks
    messages/auth
    messages/get
    messages/put
    messages/subscribe
    messages/unsubscribe
)

echo "Fetching SignalK schemas v${VERSION} ..."

for schema in "${SCHEMAS[@]}"; do
    url="${BASE_URL}/v${VERSION}/schemas/${schema}.json"
    dest="${SCHEMA_DIR}/${schema}.json"
    mkdir -p "$(dirname "$dest")"
    echo "  ${schema}.json  ← ${url}"
    if ! curl -fsSL "$url" -o "$dest"; then
        echo "Error: failed to fetch ${url}" >&2
        exit 1
    fi
done

echo "$VERSION" > "$VERSION_FILE"
echo ""
echo "Done. Schemas updated to v${VERSION}."
echo "Review changes with: git diff tests/schemas/"
