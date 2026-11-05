// Runs the core's whole-frame benchmark (examples/bench_frames.rs) three
// times per size and writes the medians to results.json and results.md.
//
//   cargo build --release -p blenderui-vlist --example bench_frames -j 1
//   node bindings/rust/blenderui-vlist/bench/run.mjs [--reps 3] [--dir <scratch dir for the 10M log>]
//
// 100, 10,000 and 1,000,000 messages come from memory, 10,000,000 from the
// file-backed log (about 1.3 GB on disk, written on first use, kept).

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const exe = join(here, "..", "..", "target", "release", "examples", process.platform === "win32" ? "bench_frames.exe" : "bench_frames");
if (!existsSync(exe)) throw new Error("build the example first");
const arg = (k, d) => { const i = process.argv.indexOf(k); return i > 0 ? process.argv[i + 1] : d; };
const reps = Number(arg("--reps", 3));
const dir = arg("--dir", join(here, "..", "..", "target", "bench-logs"));
mkdirSync(dir, { recursive: true });

const configs = [[100, "vec"], [10_000, "vec"], [1_000_000, "vec"], [1_000_000, "log"], [10_000_000, "log"]];
const median = (xs) => { const s = [...xs].sort((a, b) => a - b); return s[Math.floor((s.length - 1) / 2)]; };

const results = [];
for (const [n, source] of configs) {
  const runs = [];
  for (let i = 0; i < reps; i++) {
    const r = spawnSync(exe, ["--n", String(n), "--source", source, "--dir", dir], { encoding: "utf8", timeout: 600_000 });
    const line = (r.stdout || "").split("\n").find((l) => l.startsWith("{"));
    if (!line) { console.error(`run failed: ${n} ${source}`, (r.stderr || "").slice(-300)); continue; }
    runs.push(JSON.parse(line));
  }
  if (!runs.length) continue;
  const med = { n, source, runs: runs.length };
  for (const k of Object.keys(runs[0])) if (typeof runs[0][k] === "number" && !(k in med)) med[k] = median(runs.map((r) => r[k]));
  results.push(med);
}
writeFileSync(join(here, "results.json"), JSON.stringify(results, null, 2));

const cols = [["build_ms", "Index build ms"], ["first_us", "First frame us"], ["scroll_p50_us", "Scroll p50 us"], ["scroll_p95_us", "p95"], ["scroll_p99_us", "p99"], ["settled_p50_us", "Settled frame p50 us"], ["settled_allocs_per_frame", "Settled allocs"], ["scroll_allocs_per_frame", "Scroll allocs/frame"], ["jump_p50_us", "Jump p50 us"], ["stream_p50_us", "Stream p50 us"], ["working_set_mb", "Working set MB"]];
let md = `| Items | Source | ${cols.map((c) => c[1]).join(" | ")} |\n|---:|---|${cols.map(() => "---:").join("|")}|\n`;
for (const r of results) md += `| ${r.n.toLocaleString("en-US")} | ${r.source} | ${cols.map(([k]) => (r[k] === undefined ? "-" : r[k].toFixed(k.includes("allocs") ? 2 : 1))).join(" | ")} |\n`;
writeFileSync(join(here, "results.md"), md);
console.log(md);
