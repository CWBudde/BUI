#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-2.0-or-later
#
# Records the engines' benchmarks for tracking over time: run it on a quiet machine
# and commit the new file. Compare two with benchstat from golang.org/x/perf.
#
#   scripts/bench.sh [count]   writes bench/<date>-<commit>.txt
#
# Native times move with the machine's load by up to 3x (docs/design.md, "Speed"), so
# only ratios from the same run mean much. The load goes in the file for that reason.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
count="${1:-5}"
out="$root/bench/$(date +%F)-$(git -C "$root" rev-parse --short HEAD).txt"

# macOS has no /proc: its sysctl prints "{ 1.23 2.34 3.45 }", or "{ 1,23 ... }" in a German
# locale, hence LC_ALL=C.
load() {
	if [[ -r /proc/loadavg ]]; then
		cut -d' ' -f1-3 /proc/loadavg
	else
		LC_ALL=C sysctl -n vm.loadavg | tr -d '{}' | xargs
	fi
}

# The native engine's benchmarks need the libraries of BUI's CMake build (README, "Engines").
tags=()
if [[ -f "$(git -C "$root" rev-parse --show-toplevel)/build/kit/libbui_kit.a" ]]; then
	tags=(-tags bui_cgo)
else
	echo "bench: no build/kit/libbui_kit.a at the repository's root, so the wasm engine only" >&2
fi

mkdir -p "$root/bench"
cd "$root"
{
	printf '# %s, load: %s\n' "$(git rev-parse HEAD)" "$(load)"
	go test ${tags[@]+"${tags[@]}"} -run '^$' -bench . -benchmem -benchtime 200x -count "$count" ./internal/enginetest/
} | tee "$out"
