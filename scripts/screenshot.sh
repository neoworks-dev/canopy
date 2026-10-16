#!/usr/bin/env bash
# Runs a Canopy bundle on a headless Hyprland output and saves a screenshot,
# with svelte-gpui's scripts/shot.sh. Config, data and the index daemon live in
# a scratch directory, so a run never touches your own settings or index.
#
#   scripts/screenshot.sh [BUNDLE] [OUT.png] [SECONDS] [host flags…]   e.g. --do press:Home
#
# OUTPUT names the headless output (default canopy), CARGO_TARGET_DIR where the
# canopy binary was built, and SANDBOX the scratch directory (default /tmp/canopy-sandbox).
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
bundle=${1:-$root/dist/canopy.js}
out=${2:-/tmp/canopy.png}
seconds=${3:-3}
shift $(( $# < 3 ? $# : 3 ))

target=${CARGO_TARGET_DIR:-$root/target}
sandbox=${SANDBOX:-/tmp/canopy-sandbox}
mkdir -p "$sandbox/config" "$sandbox/data" "$sandbox/runtime"

export XDG_CONFIG_HOME=$sandbox/config XDG_DATA_HOME=$sandbox/data
export NEOWORKS_DATA_DIR=$sandbox/data NEOWORKS_RUNTIME_DIR=$sandbox/runtime
export CANOPYD_BIN=${CANOPYD_BIN:-$target/release/canopyd}
export HOST=$target/release/canopy OUTPUT=${OUTPUT:-canopy}

exec "$root/node_modules/@neoworks/svelte-gpui/scripts/shot.sh" "$bundle" "$out" "$seconds" --dev "$@"
