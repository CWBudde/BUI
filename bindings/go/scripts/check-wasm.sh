#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-2.0-or-later
#
# Requires the committed module to be what the sources build, from an empty build
# directory, and within its budget.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
module="$root/internal/engine/wasmengine/bui_kit.wasm"
budget=$((8 * 1024 * 1024)) # docs/design.md, "Feasibility criteria"

# git diff sees no change in a file it does not track.
if ! git -C "$root" ls-files --error-unmatch internal/engine/wasmengine/bui_kit.wasm >/dev/null 2>&1; then
	echo "check-wasm: bui_kit.wasm is not tracked by git, so there is nothing to compare with" >&2
	exit 1
fi

build="$(mktemp -d)"
trap 'rm -rf "$build"' EXIT
BUI_WASM_BUILD_DIR="$build" "$root/scripts/build-wasm.sh"

if ! git -C "$root" diff --exit-code --stat -- internal/engine/wasmengine/bui_kit.wasm; then
	echo "check-wasm: the committed module is not what the sources build" >&2
	exit 1
fi

size="$(wc -c <"$module")"
if [ "$size" -gt "$budget" ]; then
	echo "check-wasm: bui_kit.wasm is $size bytes, over the budget of $budget" >&2
	exit 1
fi
echo "check-wasm: identical, and $size of $budget bytes"
