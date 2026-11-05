// Runs the head-to-head (examples/bench_list_gpui.rs) several times per
// library and size, and writes the medians to results.json and results.md.
//
//   cargo build --release --features bench --example bench_list_gpui -j 1
//   node bench/run.mjs [--reps 3] [--seconds 5] [--sizes 100,10000,1000000,10000000]
//
// One process per run, per library and per size; nothing else should be
// running. A library that runs out of memory (the process itself caps its
// working set above the data at --cap-mb) or time (--limit-seconds) is
// recorded as a failure with how far it got, and the median is taken over
// the runs that finished.

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const exeName = process.platform === "win32" ? "bench_list_gpui.exe" : "bench_list_gpui";
const candidates = [join(here, "..", "target", "release", "examples", exeName), join(here, "..", "..", "blenderui-table-gpui", "target", "release", "examples", exeName)];
const exe = candidates.find(existsSync);
if (!exe) throw new Error("build the example first");
const arg = (k, d) => { const i = process.argv.indexOf(k); return i > 0 ? process.argv[i + 1] : d; };
const reps = Number(arg("--reps", 3));
const seconds = arg("--seconds", "5");
const sizes = arg("--sizes", "100,10000,1000000,10000000").split(",").map(Number);
const capMb = arg("--cap-mb", "4096");
const limit = arg("--limit-seconds", "150");
const dir = join(here, "..", "target", "bench-logs");
mkdirSync(dir, { recursive: true });

const libs = ["ours", "list", "virtual"];
const median = (xs) => { const s = [...xs].sort((a, b) => a - b); return s[Math.floor((s.length - 1) / 2)]; };

const results = [];
const run = (lib, n) => {
  const runs = [], fails = [];
  for (let i = 0; i < reps; i++) {
    const r = spawnSync(exe, ["--lib", lib, "--n", String(n), "--seconds", seconds, "--cap-mb", capMb, "--limit-seconds", limit, "--dir", dir], { encoding: "utf8", timeout: (Number(limit) + 120) * 1000 });
    const line = (r.stdout || "").split("\n").find((l) => l.startsWith("{"));
    if (!line) { fails.push({ failed: `no output (exit ${r.status})`, stderr: (r.stderr || "").slice(-300) }); console.error(`${lib} ${n} run ${i + 1}: no output`); continue; }
    const j = JSON.parse(line);
    if (j.failed) { fails.push(j); console.error(`${lib} ${n} run ${i + 1}: FAILED ${j.failed} in ${j.stage} after ${j.elapsed_s}s, ${j.ram_above_data_mb} MB`); continue; }
    runs.push(j);
    console.error(`${lib} ${n} run ${i + 1}: p50 ${j.frame_ms_p50} ms, first ${j.first_frame_ms} ms`);
  }
  const out = { lib: runs[0]?.lib ?? fails[0]?.lib ?? lib, n, finished: runs.length, failed: fails.length, failures: fails };
  if (runs.length) for (const k of Object.keys(runs[0])) if (typeof runs[0][k] === "number" && !(k in out)) out[k] = median(runs.map((r) => r[k]));
  results.push(out);
  // a library that fails at one size will fail at the next: do not burn the time limit again
  return runs.length > 0;
};

for (const lib of libs) {
  let alive = true;
  for (const n of sizes) { if (!alive) { results.push({ lib, n, finished: 0, failed: 0, skipped: "failed at a smaller size" }); continue; } alive = run(lib, n); }
}
run("blank", sizes.includes(1000000) ? 1000000 : sizes[0]);
writeFileSync(join(here, "results.json"), JSON.stringify(results, null, 2));

const cols = [["setup_ms", "Setup ms"], ["first_frame_ms", "First frame ms"], ["frame_ms_p50", "Scroll p50 ms"], ["frame_ms_p95", "p95"], ["frame_ms_p99", "p99"], ["cpu_ms_per_frame", "CPU ms/frame"], ["main_allocs_per_frame", "Allocs/frame"], ["still_frame_ms_p50", "Repaint p50 ms"], ["jump_ms_p50", "Jump p50 ms"], ["ram_above_data_mb", "RAM above data MB"], ["peak_above_data_mb", "Peak MB"]];
let md = `| Library | Items | Finished | ${cols.map((c) => c[1]).join(" | ")} |\n|---|---:|---:|${cols.map(() => "---:").join("|")}|\n`;
for (const r of results) {
  if (r.skipped) { md += `| ${r.lib} | ${r.n.toLocaleString("en-US")} | skipped (${r.skipped}) |${cols.map(() => " |").join("")}\n`; continue; }
  if (!r.finished) { const f = r.failures[0] ?? {}; md += `| ${r.lib} | ${r.n.toLocaleString("en-US")} | 0/${reps}: ${f.failed ?? "failed"} in ${f.stage ?? "?"} after ${f.elapsed_s ?? "?"} s, ${f.ram_above_data_mb ?? "?"} MB |${cols.map(() => " |").join("")}\n`; continue; }
  md += `| ${r.lib} | ${r.n.toLocaleString("en-US")} | ${r.finished}/${reps} | ${cols.map(([k]) => (r[k] ?? 0).toFixed(k.includes("allocs") ? 0 : k.includes("mb") ? 0 : 2)).join(" | ")} |\n`;
}
writeFileSync(join(here, "results.md"), md);
console.log(md);
