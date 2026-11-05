// Runs the head-to-head (examples/bench_gpui.rs) several times per
// configuration and writes the medians to results.json and results.md.
//
//   cargo build --release --features bench --example bench_gpui -j 1
//   node bench/run.mjs [--reps 3] [--seconds 8]
//
// One process per run and per library; nothing else should be running.

import { spawnSync } from "node:child_process";
import { writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const exe = join(here, "..", "target", "release", "examples", process.platform === "win32" ? "bench_gpui.exe" : "bench_gpui");
const arg = (k, d) => { const i = process.argv.indexOf(k); return i > 0 ? process.argv[i + 1] : d; };
const reps = Number(arg("--reps", 3));
const seconds = arg("--seconds", "8");

const configs = [{ rows: 1_000_000, cols: 20 }, { rows: 100_000, cols: 8 }];
const only = (cfg, lib) => lib !== "blank" || cfg.rows === 1_000_000; // the framework baseline once
const libs = ["ours", "component", "blank"];
const median = (xs) => { const s = [...xs].sort((a, b) => a - b); return s[Math.floor((s.length - 1) / 2)]; };

const results = [];
for (const cfg of configs) {
  for (const lib of libs.filter((l) => only(cfg, l))) {
    const runs = [];
    for (let i = 0; i < reps; i++) {
      const r = spawnSync(exe, ["--lib", lib, "--rows", String(cfg.rows), "--cols", String(cfg.cols), "--seconds", seconds], { encoding: "utf8", timeout: 300_000 });
      const line = (r.stdout || "").split("\n").find((l) => l.startsWith("{"));
      if (!line) { console.error(`run failed: ${lib} ${cfg.rows}x${cfg.cols}`, r.stderr?.slice(-400)); continue; }
      runs.push(JSON.parse(line));
      console.error(`${lib} ${cfg.rows}x${cfg.cols} run ${i + 1}: p50 ${runs.at(-1).frame_ms_p50} ms`);
    }
    if (!runs.length) continue;
    const med = { lib: runs[0].lib, rows: cfg.rows, cols: cfg.cols, runs: runs.length };
    for (const k of Object.keys(runs[0])) if (typeof runs[0][k] === "number" && !(k in med)) med[k] = median(runs.map((r) => r[k]));
    med.key_works = runs.every((r) => r.key_works);
    results.push(med);
  }
}
writeFileSync(join(here, "results.json"), JSON.stringify(results, null, 2));

const cols = [
  ["first_frame_ms", "First frame ms"], ["frame_ms_p50", "Frame ms p50"], ["frame_ms_p95", "p95"], ["frame_ms_p99", "p99"],
  ["fps", "FPS"], ["cpu_ms_per_frame", "CPU ms/frame"], ["main_allocs_per_frame", "Allocs/frame (UI thread)"],
  ["proc_allocs_per_frame", "Allocs/frame (process)"], ["still_frame_ms_p50", "Repaint ms p50"], ["still_main_allocs_per_frame", "Repaint allocs/frame"], ["select_ms", "Select ms"], ["key_ms", "Key ms"],
  ["sort_numeric_ms", "Sort numeric ms"], ["sort_string_ms", "Sort string ms"], ["ws_delta_mb", "RAM above data MB"], ["peak_ws_mb", "Peak RAM MB"],
];
let md = `| Library | Rows x cols | ${cols.map((c) => c[1]).join(" | ")} |\n|---|---|${cols.map(() => "---:").join("|")}|\n`;
for (const r of results) md += `| ${r.lib} | ${r.rows.toLocaleString("en-US")} x ${r.cols} | ${cols.map(([k]) => (r[k] ?? 0).toFixed(k.includes("allocs") || k.includes("ms") ? 2 : 1)).join(" | ")} |\n`;
writeFileSync(join(here, "results.md"), md);
console.log(md);
