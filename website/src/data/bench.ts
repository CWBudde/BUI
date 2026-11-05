// Every benchmark number on the site, in one place. Copied from README.md "Performance"
// (and docs/javascript.md#performance), with the README's rounding. When the README
// changes, change it here; the landing page and the MDX pages read from this file.
//
// In MDX:   <BenchChart id="first-frame" />          a chart, plus an "Open bench" table
//           <BenchTable id="startup" />              just the exact table
// The ids are the keys of `charts` and `tables` below.

export const machine = "Ryzen 7 5800U laptop, Windows 11, on AC power";
export const benchCaveat =
  "Measured before the latest features (ImGui, the Linux GPU renderer, the Blender search and expression code) landed. Your numbers will differ; the tools ship with the repo.";

export const sources = {
  readme: "README.md#performance",
  rust: "bindings/rust/bench/README.md",
  js: "docs/javascript.md#performance",
} as const;

/** One row of a bar chart. `display` is the text shown (defaults to the value + unit). */
export interface Bar {
  label: string;
  /** Smaller second line under the label, e.g. the renderer. */
  sub?: string;
  value: number;
  display?: string;
  /** BUI's own bars are drawn in Blender blue; the rest in grey. */
  ours?: boolean;
}

export interface Series {
  name: string;
  ours?: boolean;
}

/** A grouped chart: one group per category, one bar per series. `null` = not measured. */
export interface Group {
  label: string;
  sub?: string;
  values: (number | null)[];
  display?: (string | null)[];
}

export interface Table {
  caption: string;
  columns: string[];
  /** Cells may contain <strong>. First cell is the row header. */
  rows: string[][];
  /** Column indexes to right-align (numbers). */
  numeric?: number[];
  note?: string;
}

interface ChartBase {
  title: string;
  /** What's measured, in one line under the title. */
  subtitle?: string;
  unit: string;
  better: "lower" | "higher";
  /** The one-glance takeaway, e.g. { value: "40×", text: "sooner first frame than gpui" }. */
  headline?: { value: string; text: string };
  /** Id of the exact table in `tables`, revealed by "Open bench". */
  table: TableId;
  source: string;
}

export interface BarSpec extends ChartBase {
  kind: "bars";
  bars: Bar[];
}
export interface GroupSpec extends ChartBase {
  kind: "groups";
  series: Series[];
  groups: Group[];
}
export interface LineSpec extends ChartBase {
  kind: "line";
  /** x positions, in order (e.g. cell counts). */
  x: number[];
  xLabel: string;
  lines: { name: string; ours?: boolean; values: number[] }[];
}
export type ChartSpec = BarSpec | GroupSpec | LineSpec;

const ratio = (a: number, b: number) => {
  const r = a / b;
  return `${r >= 10 ? Math.round(r) : Math.round(r * 10) / 10}×`;
};

// ---------------------------------------------------------------------------------------
// The exact tables
// ---------------------------------------------------------------------------------------

const tablesBase = {
  startup: {
    caption: "Start time and battery: the same small settings window, median of 10 launches, process start to first frame",
    columns: ["App", "Renderer", "Start time", "CPU used to start", "Memory (idle)", "Idle CPU per minute"],
    numeric: [2, 3, 4, 5],
    rows: [
      ["Rust (HTML + CSS)", "CPU", "<strong>38 ms</strong>", "<strong>27 ms</strong>", "<strong>13 MB</strong>", "<strong>0 ms</strong>"],
      ["", "GPU", "42 ms", "61 ms", "31 MB", "1.8 ms"],
      ["JavaScript (React, Node.js)", "CPU", "94 ms", "100 ms", "48 MB", "68 ms"],
      ["", "GPU", "99 ms", "131 ms", "60 MB", "71 ms"],
      ["Blender components (Python)", "CPU", "112 ms", "86 ms", "23 MB", "0 ms"],
      ["", "GPU", "116 ms", "125 ms", "32 MB", "1.7 ms"],
      ["tkinter syntax (Python)", "CPU", "139 ms", "112 ms", "25 MB", "0.2 ms"],
      ["", "GPU", "159 ms", "157 ms", "43 MB", "2.4 ms"],
      ["Qt syntax (Python)", "CPU", "156 ms", "123 ms", "27 MB", "0 ms"],
      ["", "GPU", "161 ms", "160 ms", "36 MB", "1.2 ms"],
      ["<strong>Same gpui code:</strong> blenderui", "CPU", "<strong>40 ms</strong>", "<strong>27 ms</strong>", "<strong>15 MB</strong>", "<strong>0 ms</strong>"],
      ["blenderui", "GPU", "43 ms", "60 ms", "31 MB", "2.1 ms"],
      ["gpui", "GPU", "231 ms", "226 ms", "29 MB", "743 ms"],
      ["gpui-fast", "GPU", "226 ms", "217 ms", "31 MB", "2.5 ms"],
    ],
    note: "Python 3.14 and Node 22. The last rows are one gpui-syntax source file (bindings/rust/bench/common/settings.rs) compiled unchanged against blenderui, gpui and gpui-fast; gpui has no form widgets, so there the controls are styled divs. Battery cost is the CPU time an app causes; memory doesn't count GPU memory.",
  },
  cells3000: {
    caption: "Rust, 3000 animated cells, all apps in the same session",
    columns: ["", "blenderui (GPU)", "blenderui (CPU)", "gpui-fast", "gpui"],
    numeric: [1, 2, 3, 4],
    rows: [
      ["FPS", "<strong>59.9</strong>", "57.6", "31.5", "19.9"],
      ["CPU per frame", "<strong>11.6 ms</strong>", "22.0 ms", "31.9 ms + GPU", "50.5 ms + GPU"],
      ["RAM", "40 MB", "<strong>30 MB</strong>", "65 MB", "77 MB"],
      ["First frame", "<strong>~5 ms</strong>", "<strong>~5 ms</strong>", "~200 ms", "~220 ms"],
      ["Idle CPU", "<strong>0 %</strong>", "0–0.9 %", "0–0.4 %", "0.9–1.4 %"],
    ],
    note: "gpui-fast's heavy-scene frame rate varies between sessions on this laptop (30–60 fps), so compare within one session.",
  },
  cellsFull: {
    caption: "Rust, full results: 100, 1000 and 3000 animated cells",
    columns: ["", "blenderui (GPU)", "blenderui (CPU)", "gpui-fast", "gpui"],
    numeric: [1, 2, 3, 4],
    rows: [
      ["FPS, 1000 animated cells", "<strong>60</strong>", "<strong>60</strong>", "49.2", "36.4"],
      ["CPU per frame, 1000 / 100 cells", "<strong>4.7 / 1.2 ms</strong>", "10.2 / 4.6 ms", "20.5 / 2.7 ms + GPU", "27.7 / 4.7 ms + GPU"],
      ["Uncapped FPS, 100 / 1000 / 3000 cells", "<strong>1491 / 309 / 94</strong>", "649 / 202 / 76", "-", "-"],
      ["RAM, 1000 / 100 cells", "35 / 28 MB", "<strong>24 / 21 MB</strong>", "45 / 47 MB", "47 / 40 MB"],
      ["GPU memory", "19–22 MB", "<strong>none</strong>", "19–24 MB", "16–21 MB"],
    ],
    note: "The GPU renderer is off by default: Application::new().gpu(true), tk.Tk(gpu=True), bui.set_gpu(True) or BUI_GPU=1. Its shaders evaluate the software raster's formulas: 10 of 172,800 pixels differ by more than 3/255.",
  },
  frameTimes: {
    caption: "Python and C, the full component demo (520×900)",
    columns: ["Frame", "Time"],
    numeric: [1],
    rows: [
      ["Complete frame (everything redrawn)", "~0.7 ms"],
      ["Hover or number-scrub frame (damage only)", "~0.05 ms"],
      ["Native window: input event → on screen", "~0.06 ms"],
    ],
    note: "Dear ImGui's demo window with every section open (27k triangles, 1280×800) is a full software redraw of about 5–8 ms.",
  },
  jsStartup: {
    caption: "JavaScript startup: a small React app that quits after its first frame, median of 12 launches after a warm-up",
    columns: ["", "First frame", "Launch to exit", "RAM at first frame", "What you ship"],
    numeric: [1, 2, 3, 4],
    rows: [
      ["<strong>QuickJS executable</strong> (bui build)", "<strong>68 ms</strong>", "<strong>104 ms</strong>", "<strong>19 MB</strong>", "<strong>4.9 MB</strong>"],
      ["Bun executable (bui build --bun)", "79 ms", "147 ms", "35 MB", "114 MB"],
      ["Bun (bun app.js)", "80 ms", "152 ms", "35 MB", "Bun + your app"],
      ["Node.js (node app.js)", "108 ms", "187 ms", "45 MB", "Node + your app"],
      ["Rust (for reference: the benchmark app)", "5 ms", "~92 ms", "", "3.5 MB"],
    ],
    note: "First frame is measured from the runtime's start. Launch to exit is wall time measured from outside, including process creation and teardown.",
  },
  jsAnim3000: {
    caption: "JavaScript animation, 3,000 cells: one cell changes per frame, GPU renderer, medians of 3 runs",
    columns: ["3,000 cells", "FPS", "CPU per frame", "RAM"],
    numeric: [1, 2, 3],
    rows: [
      ["Rust", "59.6", "<strong>12.0 ms</strong>", "<strong>43 MB</strong>"],
      ["QuickJS: plain JS / React with signals / React with state", "59.1 / 58.7 / 19.7", "16.3 / 14.9 / 51.0 ms", "<strong>56 / 61 / 67 MB</strong>"],
      ["Bun 1.4: plain JS / React with signals / React with state", "58.8 / 58.4 / 55.8", "13.1 / 13.3 / 20.8 ms", "80 / 85 / 118 MB"],
      ["Node 22: plain JS / React with signals / React with state", "55.8 / 56.7 / 54.8", "14.4 / 14.0 / 18.9 ms", "88 / 93 / 121 MB"],
    ],
    note: "Plain JS uses @blender-ui/native; React with signals renders the grid once; React with state re-renders it every frame. React's production build throughout.",
  },
  jsAnim1000: {
    caption: "JavaScript animation, 1,000 cells",
    columns: ["1,000 cells", "FPS", "CPU per frame", "RAM"],
    numeric: [1, 2, 3],
    rows: [
      ["Rust", "60", "<strong>4.8 ms</strong>", "<strong>32 MB</strong>"],
      ["QuickJS: plain JS / React with signals / React with state", "60 / 60 / 56.8", "6.0 / 5.6 / 17.8 ms", "<strong>38 / 40 / 43 MB</strong>"],
      ["Bun: plain JS / React with signals / React with state", "60 / 60 / 60", "5.2 / 5.2 / 7.5 ms", "58 / 63 / 81 MB"],
      ["Node: plain JS / React with signals / React with state", "60 / 60 / 60", "5.6 / 5.6 / 6.9 ms", "68 / 71 / 97 MB"],
    ],
  },
} satisfies Record<string, Table>;

const withoutRust = (t: Table, caption: string): Table => ({ ...t, caption, rows: t.rows.filter((r) => !r[0].startsWith("Rust")) });

/** The base tables plus narrower views of them, for pages about one language. */
export const tables = {
  ...tablesBase,
  /** The startup table's last rows: one gpui-syntax file compiled against each library. */
  startupGpui: {
    caption: "Start time, the same gpui-syntax file (bindings/rust/bench/common/settings.rs) compiled against each library: median of 10 launches, process start to first frame",
    columns: ["Same gpui code", ...tablesBase.startup.columns.slice(1)],
    numeric: tablesBase.startup.numeric,
    rows: tablesBase.startup.rows.slice(-4).map((r, i) => (i === 0 ? ["<strong>blenderui</strong>", ...r.slice(1)] : r)),
    note: "gpui has no form widgets, so there the controls are styled divs. Memory doesn't count GPU memory.",
  },
  jsStartupJs: withoutRust(tablesBase.jsStartup, tablesBase.jsStartup.caption),
  jsAnim3000Js: withoutRust(tablesBase.jsAnim3000, tablesBase.jsAnim3000.caption),
  jsAnim1000Js: withoutRust(tablesBase.jsAnim1000, tablesBase.jsAnim1000.caption),
} satisfies Record<string, Table>;

export type TableId = keyof typeof tables;

// ---------------------------------------------------------------------------------------
// Charts
// ---------------------------------------------------------------------------------------

const gpuiSame = (v: [number, number, number, number], d?: [string, string, string, string]): Bar[] => [
  { label: "blenderui", sub: "CPU renderer", value: v[0], display: d?.[0], ours: true },
  { label: "blenderui", sub: "GPU renderer", value: v[1], display: d?.[1], ours: true },
  { label: "gpui-fast", sub: "GPU", value: v[3], display: d?.[3] },
  { label: "gpui", sub: "Zed's, GPU", value: v[2], display: d?.[2] },
];

const cells = (v: [number, number, number, number], d?: [string, string, string, string]): Bar[] => [
  { label: "blenderui", sub: "GPU renderer", value: v[0], display: d?.[0], ours: true },
  { label: "blenderui", sub: "CPU renderer", value: v[1], display: d?.[1], ours: true },
  { label: "gpui-fast", value: v[2], display: d?.[2] },
  { label: "gpui", value: v[3], display: d?.[3] },
];

export const charts = {
  "first-frame": {
    kind: "bars",
    title: "First frame",
    subtitle: "Rust, 3000 animated cells: from launch to the first frame on screen",
    unit: "ms",
    better: "lower",
    headline: { value: ratio(200, 5), text: "sooner on screen than gpui-fast" },
    bars: cells([5, 5, 200, 220], ["~5 ms", "~5 ms", "~200 ms", "~220 ms"]),
    table: "cells3000",
    source: sources.readme,
  },
  "start-time": {
    kind: "bars",
    title: "Start time, same gpui code",
    subtitle: "One gpui-syntax file compiled against each library; median of 10 launches",
    unit: "ms",
    better: "lower",
    headline: { value: ratio(226, 40), text: "faster start than gpui-fast" },
    bars: gpuiSame([40, 43, 231, 226]),
    table: "startupGpui",
    source: sources.readme,
  },
  "start-cpu": {
    kind: "bars",
    title: "CPU used to start, same gpui code",
    subtitle: "CPU time spent from launch to first frame: what a launch costs the battery",
    unit: "ms",
    better: "lower",
    headline: { value: ratio(217, 27), text: "less CPU to start than gpui-fast" },
    bars: gpuiSame([27, 60, 226, 217]),
    table: "startupGpui",
    source: sources.readme,
  },
  "idle-cpu": {
    kind: "bars",
    title: "Idle CPU, same gpui code",
    subtitle: "CPU time per minute while the window sits idle",
    unit: "ms",
    better: "lower",
    headline: { value: "0 ms", text: "idle CPU on the CPU renderer: it sleeps" },
    bars: gpuiSame([0, 2.1, 743, 2.5]),
    table: "startupGpui",
    source: sources.readme,
  },
  "memory-start": {
    kind: "bars",
    title: "Memory when idle, same gpui code",
    subtitle: "Resident memory, not counting GPU memory",
    unit: "MB",
    better: "lower",
    headline: { value: "Half", text: "the memory of gpui on the CPU renderer" },
    bars: gpuiSame([15, 31, 29, 31]),
    table: "startupGpui",
    source: sources.readme,
  },
  "start-all": {
    kind: "groups",
    title: "Start time, every way in",
    subtitle: "The same small settings window written five ways; median of 10 launches",
    unit: "ms",
    better: "lower",
    series: [
      { name: "CPU renderer", ours: true },
      { name: "GPU renderer" },
    ],
    groups: [
      { label: "Rust", sub: "HTML + CSS", values: [38, 42] },
      { label: "JavaScript", sub: "React, Node.js", values: [94, 99] },
      { label: "Blender components", sub: "Python", values: [112, 116] },
      { label: "tkinter syntax", sub: "Python", values: [139, 159] },
      { label: "Qt syntax", sub: "Python", values: [156, 161] },
    ],
    table: "startup",
    source: sources.readme,
  },
  "idle-all": {
    kind: "bars",
    title: "Idle CPU, every way in",
    subtitle: "CPU time per minute while the settings window sits idle, CPU renderer",
    unit: "ms",
    better: "lower",
    headline: { value: "0 ms", text: "while nothing changes" },
    bars: [
      { label: "Rust", sub: "HTML + CSS", value: 0, ours: true },
      { label: "Blender components", sub: "Python", value: 0, ours: true },
      { label: "Qt syntax", sub: "Python", value: 0, ours: true },
      { label: "tkinter syntax", sub: "Python", value: 0.2, ours: true },
      { label: "JavaScript", sub: "React, Node.js event loop", value: 68, ours: true },
    ],
    table: "startup",
    source: sources.readme,
  },
  fps: {
    kind: "bars",
    title: "Frames per second",
    subtitle: "Rust, 3000 animated cells, same session",
    unit: "fps",
    better: "higher",
    headline: { value: ratio(59.9, 19.9), text: "the frame rate of gpui" },
    bars: cells([59.9, 57.6, 31.5, 19.9]),
    table: "cells3000",
    source: sources.readme,
  },
  "cpu-per-frame": {
    kind: "bars",
    title: "CPU per frame",
    subtitle: "Rust, 3000 animated cells; gpui's numbers don't include its GPU time",
    unit: "ms",
    better: "lower",
    headline: { value: ratio(31.9, 11.6), text: "less CPU per frame than gpui-fast" },
    bars: cells([11.6, 22.0, 31.9, 50.5], ["11.6 ms", "22.0 ms", "31.9 ms + GPU", "50.5 ms + GPU"]),
    table: "cells3000",
    source: sources.readme,
  },
  ram: {
    kind: "bars",
    title: "RAM",
    subtitle: "Rust, 3000 animated cells",
    unit: "MB",
    better: "lower",
    headline: { value: ratio(77, 30), text: "less RAM than gpui (CPU renderer)" },
    bars: cells([40, 30, 65, 77]),
    table: "cells3000",
    source: sources.readme,
  },
  "cpu-per-frame-cells": {
    kind: "groups",
    title: "CPU per frame, 100 and 1000 cells",
    subtitle: "gpui's numbers don't include its GPU time",
    unit: "ms",
    better: "lower",
    series: [
      { name: "blenderui GPU", ours: true },
      { name: "blenderui CPU", ours: true },
      { name: "gpui-fast" },
      { name: "gpui" },
    ],
    groups: [
      { label: "100 cells", values: [1.2, 4.6, 2.7, 4.7] },
      { label: "1000 cells", values: [4.7, 10.2, 20.5, 27.7] },
    ],
    table: "cellsFull",
    source: sources.readme,
  },
  "uncapped-fps": {
    kind: "line",
    title: "Uncapped frame rate",
    subtitle: "Frames per second with vsync off, as the scene grows",
    unit: "fps",
    better: "higher",
    x: [100, 1000, 3000],
    xLabel: "animated cells",
    lines: [
      { name: "blenderui GPU", ours: true, values: [1491, 309, 94] },
      { name: "blenderui CPU", values: [649, 202, 76] },
    ],
    table: "cellsFull",
    source: sources.readme,
  },
  "frame-times": {
    kind: "bars",
    title: "Frame cost",
    subtitle: "Python and C, the full component demo (520×900)",
    unit: "ms",
    better: "lower",
    headline: { value: "0.05 ms", text: "for a hover or scrub frame" },
    bars: [
      { label: "Complete frame", sub: "everything redrawn", value: 0.7, display: "~0.7 ms", ours: true },
      { label: "Input to screen", sub: "native window", value: 0.06, display: "~0.06 ms", ours: true },
      { label: "Hover or scrub", sub: "damage only", value: 0.05, display: "~0.05 ms", ours: true },
    ],
    table: "frameTimes",
    source: sources.readme,
  },
  "js-first-frame": {
    kind: "bars",
    title: "JavaScript: first frame",
    subtitle: "A small React app, median of 12 launches",
    unit: "ms",
    better: "lower",
    headline: { value: "4.9 MB", text: "single executable with QuickJS" },
    bars: [
      { label: "QuickJS executable", sub: "bui build", value: 68, ours: true },
      { label: "Bun executable", sub: "bui build --bun", value: 79, ours: true },
      { label: "Bun", sub: "bun app.js", value: 80, ours: true },
      { label: "Node.js", sub: "node app.js", value: 108, ours: true },
    ],
    table: "jsStartup",
    source: sources.readme,
  },
  "js-cpu-3000": {
    kind: "groups",
    title: "JavaScript: CPU per frame, 3,000 cells",
    subtitle: "One cell changes per frame; GPU renderer",
    unit: "ms",
    better: "lower",
    series: [
      { name: "Plain JS", ours: true },
      { name: "React + signals", ours: true },
      { name: "React + state" },
    ],
    groups: [
      { label: "QuickJS", values: [16.3, 14.9, 51.0] },
      { label: "Bun 1.4", values: [13.1, 13.3, 20.8] },
      { label: "Node 22", values: [14.4, 14.0, 18.9] },
    ],
    table: "jsAnim3000",
    source: sources.readme,
  },
} satisfies Record<string, ChartSpec>;

export type ChartId = keyof typeof charts;

/**
 * The landing page's headline comparison: blenderui against gpui and gpui-fast.
 * `ours` / `theirs` drive the little two-bar comparison under each number.
 */
export const highlights = [
  {
    value: "~5",
    unit: "ms",
    label: "First frame",
    note: "3000 animated cells",
    ours: 5,
    theirs: { name: "gpui-fast", value: 200, display: "~200 ms" },
    claim: `${ratio(200, 5)} sooner`,
    chart: "first-frame",
  },
  {
    value: "40",
    unit: "ms",
    label: "Start, same code",
    note: "the same gpui code, compiled three ways",
    ours: 40,
    theirs: { name: "gpui", value: 231, display: "231 ms" },
    claim: `${ratio(231, 40)} faster start`,
    chart: "start-time",
  },
  {
    value: "11.6",
    unit: "ms",
    label: "CPU per frame",
    note: "3000 cells, GPU renderer",
    ours: 11.6,
    theirs: { name: "gpui-fast", value: 31.9, display: "31.9 ms + GPU" },
    claim: `${ratio(31.9, 11.6)} less CPU`,
    chart: "cpu-per-frame",
  },
  {
    value: "0",
    unit: "ms",
    label: "Idle CPU per minute",
    note: "it sleeps until something changes",
    ours: 0,
    theirs: { name: "gpui", value: 743, display: "743 ms" },
    claim: "Nothing while idle",
    chart: "idle-cpu",
  },
] as const satisfies readonly {
  value: string;
  unit: string;
  label: string;
  note: string;
  ours: number;
  theirs: { name: string; value: number; display: string };
  claim: string;
  chart: ChartId;
}[];
