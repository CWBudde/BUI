// Every benchmark number on the site, in one place. Copied from README.md "Performance"
// (and docs/javascript.md#performance), with the README's rounding. When the README
// changes, change it here; the landing page and the MDX pages read from this file.
//
// In MDX:   <BenchChart id="first-frame" />          a chart, plus an "Open bench" table
//           <BenchTable id="startup" />              just the exact table
// The ids are the keys of `charts` and `tables` below.

export const machine = "Ryzen 7 5800U laptop, Windows 11, on AC power"
export const benchCaveat =
  "Measured before the latest features (ImGui, the Linux GPU renderer, the Blender search and expression code) landed. Your numbers will differ; the tools ship with the repo."

export const sources = {
  readme: "README.md#performance",
  rust: "bindings/rust/bench/README.md",
  js: "docs/javascript.md#performance",
} as const

/** One row of a bar chart. `display` is the text shown (defaults to the value + unit). */
export interface Bar {
  label: string
  /** Smaller second line under the label, e.g. the renderer. */
  sub?: string
  value: number
  display?: string
  /** BUI's own bars are drawn in Blender blue; the rest in grey. */
  ours?: boolean
}

export interface Series {
  name: string
  ours?: boolean
}

/** A grouped chart: one group per category, one bar per series. `null` = not measured. */
export interface Group {
  label: string
  sub?: string
  values: (number | null)[]
  display?: (string | null)[]
}

export interface Table {
  caption: string
  columns: string[]
  /** Cells may contain <strong>. First cell is the row header. */
  rows: string[][]
  /** Column indexes to right-align (numbers). */
  numeric?: number[]
  note?: string
}

interface ChartBase {
  title: string
  /** What's measured, in one line under the title. */
  subtitle?: string
  unit: string
  better: "lower" | "higher"
  /** The one-glance takeaway, e.g. { value: "40×", text: "sooner first frame than gpui" }. */
  headline?: { value: string; text: string }
  /** Id of the exact table in `tables`, revealed by "Open bench". */
  table: TableId
  source: string
}

export interface BarSpec extends ChartBase {
  kind: "bars"
  bars: Bar[]
}
export interface GroupSpec extends ChartBase {
  kind: "groups"
  series: Series[]
  groups: Group[]
}
export interface LineSpec extends ChartBase {
  kind: "line"
  /** x positions, in order (e.g. cell counts). */
  x: number[]
  xLabel: string
  lines: { name: string; ours?: boolean; values: number[] }[]
}
export type ChartSpec = BarSpec | GroupSpec | LineSpec

const ratio = (a: number, b: number) => {
  const r = a / b
  return `${r >= 10 ? Math.round(r) : Math.round(r * 10) / 10}×`
}

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
      [
        "<strong>Same gpui code:</strong> blenderui",
        "CPU",
        "<strong>40 ms</strong>",
        "<strong>27 ms</strong>",
        "<strong>15 MB</strong>",
        "<strong>0 ms</strong>",
      ],
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
      [
        "<strong>QuickJS executable</strong> (bui build)",
        "<strong>68 ms</strong>",
        "<strong>104 ms</strong>",
        "<strong>19 MB</strong>",
        "<strong>4.9 MB</strong>",
      ],
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
      [
        "QuickJS: plain JS / React with signals / React with state",
        "59.1 / 58.7 / 19.7",
        "16.3 / 14.9 / 51.0 ms",
        "<strong>56 / 61 / 67 MB</strong>",
      ],
      ["Bun 1.4: plain JS / React with signals / React with state", "58.8 / 58.4 / 55.8", "13.1 / 13.3 / 20.8 ms", "80 / 85 / 118 MB"],
      ["Node 22: plain JS / React with signals / React with state", "55.8 / 56.7 / 54.8", "14.4 / 14.0 / 18.9 ms", "88 / 93 / 121 MB"],
    ],
    note: "Plain JS uses @blender-ui/native; React with signals renders the grid once; React with state re-renders it every frame. React's production build throughout.",
  },
  jsFrameworksStartup: {
    caption:
      "React vs Svelte, startup: the same small counter app (bench/first-frame.js, bench/first-frame-svelte.js) quits after its first frame; median of 12 launches after a warm-up, one session (2026-11-06)",
    columns: ["First frame (React / Svelte)", "React", "Svelte", "RAM at first frame (React / Svelte)"],
    numeric: [1, 2, 3],
    rows: [
      ["<strong>QuickJS executable</strong> (bui build, 6.9 MB either way)", "94 ms", "<strong>89 ms</strong>", "20.5 / <strong>19.8 MB</strong>"],
      ["Bun executable (bui build --bun)", "111 ms", "107 ms", "37.8 / 38.3 MB"],
      ["Bun (bun --preload …/register app.js)", "<strong>130 ms</strong>", "243 ms", "<strong>36.4</strong> / 46.6 MB"],
      ["Node.js (node --import …/register app.js)", "<strong>147 ms</strong>", "440 ms", "<strong>46.8</strong> / 64.1 MB"],
    ],
    note: "On Node.js and Bun, Svelte's runtime loads from its source files (about 90 modules) and components come from the compile cache; bui build bundles both frameworks ahead of time.",
  },
  jsFrameworks3000: {
    caption:
      "React vs Svelte, 3,000 animated cells (bench/cells.js): one cell changes per frame, GPU renderer, medians of 3 interleaved runs, one session (2026-11-06)",
    columns: ["3,000 cells", "FPS", "CPU per frame", "RAM"],
    numeric: [1, 2, 3],
    rows: [
      ["Node 26: React with state / React with signals", "27.6 / 27.6", "42.5 / 36.9 ms", "150 / <strong>98 MB</strong>"],
      ["Node 26: Svelte (runes) / Svelte with signals", "24.8 / 26.6", "41.5 / 38.6 ms", "119 / 118 MB"],
      ["Bun 1.4: React with state / React with signals", "24.0 / 25.0", "55.8 / 40.8 ms", "111 / <strong>86 MB</strong>"],
      ["Bun 1.4: Svelte (runes) / Svelte with signals", "26.4 / 27.0", "39.3 / <strong>38.2 ms</strong>", "100 / 100 MB"],
      ["QuickJS: React with state / React with signals", "8.5 / 27.0", "117 / <strong>37.3 ms</strong>", "67 / <strong>62 MB</strong>"],
      ["QuickJS: Svelte (runes) / Svelte with signals", "26.9 / 25.9", "37.5 / 38.7 ms", "76 / 76 MB"],
    ],
    note: "This session ran at about 27 FPS for every variant, so the absolute numbers aren't comparable with the tables above; compare the rows with each other. Svelte with runes needs no signals: only the cell that changes updates. Svelte with signals passes BUI signals as stores ($style). The same session's React with state cost 45.8 ms and 190 MB on Node before this release's DOM changes.",
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
  dataTableGpui: {
    caption:
      "Data table on real gpui: this crate vs gpui-component 0.9.0, 1,000,000 rows x 20 columns, 1200x700 table in a 1280x740 window (bindings/rust/blenderui-table-gpui, examples/bench_gpui.rs, release build, Direct3D 11)",
    columns: ["", "blenderui-table-gpui", "gpui-component 0.9.0", "Empty window (no table)"],
    numeric: [1, 2, 3],
    rows: [
      ["Scroll frame, render to end of paint, median", "<strong>1.45 ms</strong>", "4.75 ms", "0.15 ms"],
      ["Scroll frame, 95th / 99th percentile", "<strong>1.77 / 2.11 ms</strong>", "8.99 / 9.58 ms", "0.26 / 0.30 ms"],
      ["Frames per second under continuous scrolling", "60.1", "60.0", "60.1"],
      ["Process CPU per frame (all threads)", "<strong>4.6 ms</strong>", "8.9 ms", "1.0 ms"],
      ["Heap allocations per scrolled frame", "<strong>259</strong>", "7,623", "102"],
      ["Heap allocations per repainted frame (no scroll)", "<strong>136</strong>", "7,416", "102"],
      ["Selection (click) to painted, CPU", "<strong>1.05 ms</strong>", "5.27 ms", "-"],
      ["Keyboard (Down) to painted, CPU", "<strong>1.63 ms</strong>", "8.85 ms", "-"],
      ["Sort 1,000,000 rows by a number", "<strong>37 ms</strong>", "140 ms", "-"],
      ["Sort 1,000,000 rows by text", "<strong>38 ms</strong>", "57 ms", "-"],
      ["First frame (window creation dominates)", "431 ms", "445 ms", "438 ms"],
      ["Memory above the data and the empty runtime", "42 MB", "42 MB", "-"],
      ["Peak working set", "231 MB", "192 MB", "-"],
      ["100,000 x 8: scroll frame median", "<strong>1.24 ms</strong>", "4.02 ms", "-"],
      ["100,000 x 8: sort by a number / by text", "<strong>4.2 / 6.8 ms</strong>", "8.6 / <strong>3.9 ms</strong>", "-"],
    ],
    note: "Ryzen 7 5800U laptop (8 cores), 15 GB, Windows 11; the GPU is whatever gpui picks on the default Direct3D 11 adapter. One process per library, median of 3 runs. Frame time is the main thread from the start of the root view's render to the end of the table's paint (elements, layout, prepaint, paint); GPU submission and the 60 Hz vsync wait are not in it, so both libraries show 60 FPS and the difference is CPU time. Scrolling is two rows per frame, driven the same way for both. Allocations come from a counting global allocator on the UI thread. Select and key inject real events through Window::dispatch_event and count the dispatch plus the next frame. Both tables hold the same data (columnar, generated once) and the same 110 px columns and 32 px rows; the component sorts its own row index with sort_by in perform_sort, as its examples sort their data, while this crate sorts a row mask with typed keys on every core. At 100,000 rows the component's plain single-threaded sort wins on text. This crate's peak memory is higher because sorting builds key pairs. Not measured: GPU time, a different GPU, other platforms.",
  },
  dataTable: {
    caption: "Rust data table, 1,000,000 rows x 20 columns in a 900x500 window (examples/bench_table.rs, release build, software raster)",
    columns: ["", "DataTable (painted)", "uniform_list rows", "One element per cell"],
    numeric: [1, 2, 3],
    rows: [
      ["Rows measured", "1,000,000", "1,000,000", "2,000"],
      ["Build", "<strong>0.17 ms</strong>", "-", "-"],
      ["First frame", "<strong>6-8 ms</strong>", "13 ms", "2,800 ms"],
      ["Scroll frame", "<strong>1.1 ms</strong>", "3.1 ms", "1,735 ms"],
      ["Heap allocations per frame (whole pipeline)", "<strong>48</strong>", "5,186", "780,467"],
      ["Allocations in the table's own drawing", "<strong>0</strong>", "-", "-"],
      ["Memory added (working set)", "<strong>17 MB</strong>", "19 MB", "138 MB"],
      ["Sort 1,000,000 rows by a column (header click, includes the repaint)", "<strong>22 ms</strong>", "-", "-"],
    ],
    note: "Frame times are the whole pipeline: render, layout, paint and the software raster. One element per cell was measured at 2,000 rows (a million would need about 70 GB and over ten minutes per frame; its cost grows linearly with rows). Allocation counts come from a counting global allocator; the table's drawing allocates nothing in steady state (a unit test asserts it). The same table on real gpui against gpui-component is in the next chart.",
  },
  shadcnDataTable: {
    caption:
      "The shadcn layer on the painted table, 1,000,000 rows x 20 columns in a 900x500 window (examples/bench_table.rs, release build, software raster, Windows 11)",
    columns: ["", "DataTable before this change", "DataTable after", "shadcn Table<T>, typed cells"],
    numeric: [1, 2, 3],
    rows: [
      ["Runs (median of)", "5", "5", "3"],
      ["Build", "0.10 ms", "0.10 ms", "1.6 ms"],
      ["First frame", "6.7 ms", "7.0 ms", "6.9 ms"],
      ["Scroll frame", "1.39 ms", "<strong>1.26 ms</strong>", "1.53 ms"],
      ["Heap allocations per frame (whole pipeline)", "48", "<strong>48</strong>", "<strong>48</strong>"],
      ["Allocations in the table's own drawing", "0", "<strong>0</strong>", "<strong>0</strong>"],
      ["Memory added (working set)", "17.2 MB", "17.3 MB", "24.1 MB"],
      ["Sort 1,000,000 rows by a column (header click, includes the repaint)", "25.5 ms", "24.5 ms", "27.6 ms"],
    ],
    note: "Both DataTable columns are the same benchmark (the painted DataTable of the delegate API): before is the commit that precedes this change, built in a separate worktree, run alternately with the current build on the same machine; the difference is noise. The last column is the same table written with shadcn's ColumnDef and Table: a checkbox column, a dollar column and a progress column among the numbers, rows held as 4-byte row numbers whose cells are computed. Its extra build time is creating the one million row values and the row model; its extra memory is the data and the row model (a list of row numbers). The allocation counts come from a counting global allocator; unit tests assert that the table's own drawing allocates nothing in steady state, for both. Not measured: other machines, the real GPU path.",
  },
  vlistIndex: {
    caption:
      "Height index against a flat Vec of running totals, medians of 5 runs of a batch (bindings/rust/blenderui-vlist, examples/bench_core.rs, release build)",
    columns: ["Items", "What", "HeightIndex", "Flat Vec"],
    numeric: [2, 3],
    rows: [
      ["100", "A frame's lookups: find an item, read 30 heights", "53 ns", "34 ns"],
      ["100", "Set one height / set the last item", "22 / 11 ns", "15 / 1.4 ns"],
      ["10,000", "Set one height, random item", "<strong>53 ns</strong>", "684 ns"],
      ["10,000", "Prepend a 500-item page", "<strong>0.3 us</strong>", "17 us"],
      ["1,000,000", "Item at a pixel offset (all measured, random)", "136 ns", "63 ns"],
      ["1,000,000", "Set one height, random item", "<strong>112 ns</strong>", "81,849 ns"],
      ["1,000,000", "Prepend a 500-item page", "<strong>0.1 us</strong>", "1,838 us"],
      ["1,000,000", "Build with nothing measured / heap per item", "<strong>0.19 ms / 0.13 B</strong>", "needs every height / 8 B"],
      ["1,000,000", "Heap after touching only the last 60 items", "<strong>189 KB</strong>", "8 MB"],
      ["10,000,000", "Item at a pixel offset (all measured, random)", "536 ns", "387 ns"],
      ["10,000,000", "Set one height, random item", "<strong>182 ns</strong>", "3,825,956 ns"],
      ["10,000,000", "Prepend a 500-item page", "<strong>0.1 us</strong>", "26,647 us"],
      ["10,000,000", "Build with nothing measured / heap per item", "<strong>0.86 ms / 0.12 B</strong>", "needs every height / 8 B"],
    ],
    note: "Single-threaded, inputs seeded. Heap is live bytes from a counting allocator. Reading is where the flat array wins (about 1.5 times faster to find an item by offset when everything is measured, and far faster at 100 items in absolute ns); changing anything is where it loses. Not measured: other machines.",
  },
  vlistFrames: {
    caption:
      "Core frame cost: layout and paint of a chat into a counting sink, medians of 3 runs, 600 px viewport, 57 px scrolled per frame (examples/bench_frames.rs, bench/run.mjs)",
    columns: [
      "Items (source)",
      "Index build",
      "First frame",
      "Scroll frame p50 / p99",
      "Settled frame p50",
      "Jump p50",
      "Allocations per frame (scroll / settled)",
      "Working set",
    ],
    numeric: [1, 2, 3, 4, 5, 6, 7],
    rows: [
      ["100 (memory)", "0.0 ms", "50 us", "4.0 / 4.9 us", "4.0 us", "3.9 us", "0.02 / 0", "2 MB"],
      ["10,000 (memory)", "0.3 ms", "48 us", "4.7 / 10.9 us", "3.5 us", "13 us", "2.2 / 0", "5 MB"],
      ["1,000,000 (memory)", "19 ms", "61 us", "4.7 / 11.0 us", "3.8 us", "27 us", "2.2 / 0", "205 MB (the messages)"],
      ["1,000,000 (file log)", "8 ms", "71 us", "4.9 / 15 us", "4.1 us", "37 us", "2.2 / 0", "<strong>2 MB</strong>"],
      ["10,000,000 (file log)", "79 ms", "78 us", "4.8 / 19 us", "2.7 us", "36 us", "2.3 / 0", "<strong>3.6 MB</strong>"],
    ],
    note: "The sink only counts calls and the text width is a length times a constant, so this is the list's own cost, not a text engine's or a rasterizer's. The scroll allocations are the two a newly measured message needs (its cache entry); a frame that shows messages already measured allocates nothing. The file log was just written, so its pages come from the operating system's cache. Streaming a token into the last message (edit, layout, paint) takes about 20 us.",
  },
  vlistGpui: {
    caption:
      "Variable-height list on real gpui 0.3.11: this crate against gpui's list (SumTree) and gpui-component 0.9.0's virtual_list, 700x600 chat, medians of 3 runs, one process per run (bindings/rust/blenderui-vlist-gpui, examples/bench_list_gpui.rs)",
    columns: [
      "Items",
      "Library",
      "Setup",
      "First frame",
      "Scroll frame p50 / p95 / p99",
      "Allocations per frame",
      "RAM above the empty window",
      "Jump p50",
    ],
    numeric: [2, 3, 4, 5, 6, 7],
    rows: [
      [
        "100",
        "<strong>blenderui-vlist</strong>",
        "0 ms",
        "508 ms",
        "<strong>0.54 / 0.84 / 0.92 ms</strong>",
        "<strong>134</strong>",
        "0 MB",
        "<strong>0.53 ms</strong>",
      ],
      ["", "gpui list", "0 ms", "515 ms", "1.00 / 1.39 / 1.47 ms", "793", "0 MB", "1.83 ms"],
      ["", "virtual_list", "0.1 ms", "496 ms", "1.04 / 1.46 / 1.56 ms", "923", "0 MB", "1.94 ms"],
      [
        "10,000",
        "<strong>blenderui-vlist</strong>",
        "0 ms",
        "501 ms",
        "<strong>0.69 / 1.12 / 1.46 ms</strong>",
        "<strong>152</strong>",
        "5 MB",
        "<strong>1.41 ms</strong>",
      ],
      ["", "gpui list", "0.6 ms", "506 ms", "1.30 / 1.70 / 1.86 ms", "939", "1 MB", "2.60 ms"],
      ["", "virtual_list", "8.4 ms", "507 ms", "1.26 / 1.72 / 1.85 ms", "1,035", "0 MB", "2.10 ms"],
      [
        "1,000,000",
        "<strong>blenderui-vlist</strong>",
        "0.3 ms",
        "<strong>224 ms</strong>",
        "<strong>0.71 / 1.16 / 1.39 ms</strong>",
        "<strong>152</strong>",
        "2 MB",
        "<strong>1.47 ms</strong>",
      ],
      ["", "gpui list", "58 ms", "371 ms", "1.32 / 1.78 / 1.89 ms", "982", "86 MB", "2.58 ms"],
      ["", "virtual_list", "628 ms", "836 ms", "1.30 / 1.76 / 1.91 ms", "1,091", "10 MB", "2.15 ms"],
      [
        "10,000,000 (file log)",
        "<strong>blenderui-vlist</strong>",
        "82 ms",
        "<strong>591 ms</strong>",
        "<strong>0.74 / 1.22 / 1.44 ms</strong>",
        "<strong>153</strong>",
        "<strong>6 MB</strong>",
        "<strong>1.54 ms</strong>",
      ],
      ["", "gpui list", "537 ms", "1,995 ms", "1.44 / 1.87 / 2.03 ms", "957", "910 MB", "2.82 ms"],
      ["", "virtual_list", "7,285 ms", "7,843 ms", "1.30 / 1.82 / 2.17 ms", "984", "151 MB", "2.23 ms"],
    ],
    note: "Ryzen 7 5800U laptop, 16 GB, Windows 11, Direct3D 11. Frame time is the main thread from the start of the root view's render to the end of paint; GPU submission and the 60 Hz wait are not in it (all three show 60 FPS). Scrolling is 57 px per frame, driven the same way for all three; the empty window with the same harness takes 0.13 ms and 102 allocations per frame. RAM is the working set after the scroll minus the same window with 100 items (about 29 MB) and minus the data. Nobody hit the 4 GB or 150 s limits, so there are no failures to report: gpui's list held ten million items in 910 MB and virtual_list took 7.3 s to build its size table. virtual_list's sizes come from a cheap synthetic text width, which favours it. Our first frame at 100 and 10,000 items is window creation. Not measured: GPU time, other machines, selection and keyboard latency, streaming updates on the competitors. The machine was in normal use during the runs.",
  },
} satisfies Record<string, Table>

const withoutRust = (t: Table, caption: string): Table => ({ ...t, caption, rows: t.rows.filter((r) => !r[0].startsWith("Rust")) })

/** The base tables plus narrower views of them, for pages about one language. */
export const tables = {
  ...tablesBase,
  /** The startup table's last rows: one gpui-syntax file compiled against each library. */
  startupGpui: {
    caption:
      "Start time, the same gpui-syntax file (bindings/rust/bench/common/settings.rs) compiled against each library: median of 10 launches, process start to first frame",
    columns: ["Same gpui code", ...tablesBase.startup.columns.slice(1)],
    numeric: tablesBase.startup.numeric,
    rows: tablesBase.startup.rows.slice(-4).map((r, i) => (i === 0 ? ["<strong>blenderui</strong>", ...r.slice(1)] : r)),
    note: "gpui has no form widgets, so there the controls are styled divs. Memory doesn't count GPU memory.",
  },
  jsStartupJs: withoutRust(tablesBase.jsStartup, tablesBase.jsStartup.caption),
  jsAnim3000Js: withoutRust(tablesBase.jsAnim3000, tablesBase.jsAnim3000.caption),
  jsAnim1000Js: withoutRust(tablesBase.jsAnim1000, tablesBase.jsAnim1000.caption),
} satisfies Record<string, Table>

export type TableId = keyof typeof tables

// ---------------------------------------------------------------------------------------
// Charts
// ---------------------------------------------------------------------------------------

const gpuiSame = (v: [number, number, number, number], d?: [string, string, string, string]): Bar[] => [
  { label: "blenderui", sub: "CPU renderer", value: v[0], display: d?.[0], ours: true },
  { label: "blenderui", sub: "GPU renderer", value: v[1], display: d?.[1], ours: true },
  { label: "gpui-fast", sub: "GPU", value: v[3], display: d?.[3] },
  { label: "gpui", sub: "Zed's, GPU", value: v[2], display: d?.[2] },
]

const cells = (v: [number, number, number, number], d?: [string, string, string, string]): Bar[] => [
  { label: "blenderui", sub: "GPU renderer", value: v[0], display: d?.[0], ours: true },
  { label: "blenderui", sub: "CPU renderer", value: v[1], display: d?.[1], ours: true },
  { label: "gpui-fast", value: v[2], display: d?.[2] },
  { label: "gpui", value: v[3], display: d?.[3] },
]

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
    series: [{ name: "CPU renderer", ours: true }, { name: "GPU renderer" }],
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
    series: [{ name: "blenderui GPU", ours: true }, { name: "blenderui CPU", ours: true }, { name: "gpui-fast" }, { name: "gpui" }],
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
  "data-table-frame": {
    kind: "bars",
    title: "Rust data table: scroll frame, 1,000,000 rows x 20 columns",
    subtitle: "Whole frame, 900x500 window, software raster. One element per cell: 2,000 rows",
    unit: "ms",
    better: "lower",
    headline: { value: "1.1 ms", text: "per scrolled frame at a million rows" },
    bars: [
      { label: "DataTable", sub: "painted like Blender's spreadsheet", value: 1.1, display: "1.1 ms", ours: true },
      { label: "uniform_list", sub: "20 cell elements per visible row", value: 3.1, display: "3.1 ms", ours: true },
      { label: "One element per cell", sub: "2,000 rows only", value: 1735, display: "1,735 ms" },
    ],
    table: "dataTable",
    source: "bindings/rust/blenderui/examples/bench_table.rs",
  },
  "shadcn-data-table-frame": {
    kind: "bars",
    title: "shadcn Data Table: scroll frame, 1,000,000 rows x 20 columns",
    subtitle: "Whole frame, 900x500 window, software raster, median of 3 to 5 runs",
    unit: "ms",
    better: "lower",
    headline: { value: "1.5 ms", text: "per scrolled frame through ColumnDef and Table" },
    bars: [
      { label: "DataTable", sub: "the delegate API", value: 1.26, display: "1.26 ms", ours: true },
      { label: "shadcn Table<T>", sub: "ColumnDef, typed cells", value: 1.53, display: "1.53 ms", ours: true },
    ],
    table: "shadcnDataTable",
    source: "bindings/rust/blenderui/examples/bench_table.rs",
  },
  "data-table-gpui": {
    kind: "bars",
    title: "gpui: scroll frame, 1,000,000 rows x 20 columns",
    subtitle: "Main thread, render to end of paint, median of 3 runs, real window",
    unit: "ms",
    better: "lower",
    headline: { value: "3.3x", text: "less CPU per scrolled frame than gpui-component" },
    bars: [
      { label: "blenderui-table-gpui", sub: "paints visible cells", value: 1.45, display: "1.45 ms", ours: true },
      { label: "gpui-component 0.9.0", sub: "an element per cell", value: 4.75, display: "4.75 ms" },
      { label: "Empty window", sub: "the framework alone", value: 0.15, display: "0.15 ms" },
    ],
    table: "dataTableGpui",
    source: "bindings/rust/blenderui-table-gpui/bench/results.md",
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
  "js-frameworks-cpu-3000": {
    kind: "groups",
    title: "React vs Svelte: CPU per frame, 3,000 cells",
    subtitle: "One cell changes per frame; GPU renderer; one session, interleaved runs",
    unit: "ms",
    better: "lower",
    series: [{ name: "React + signals", ours: true }, { name: "React + state" }, { name: "Svelte (runes)", ours: true }, { name: "Svelte + signals", ours: true }],
    groups: [
      { label: "QuickJS", values: [37.3, 117, 37.5, 38.7] },
      { label: "Bun 1.4", values: [40.8, 55.8, 39.3, 38.2] },
      { label: "Node 26", values: [36.9, 42.5, 41.5, 38.6] },
    ],
    table: "jsFrameworks3000",
    source: sources.js,
  },
  "js-cpu-3000": {
    kind: "groups",
    title: "JavaScript: CPU per frame, 3,000 cells",
    subtitle: "One cell changes per frame; GPU renderer",
    unit: "ms",
    better: "lower",
    series: [{ name: "Plain JS", ours: true }, { name: "React + signals", ours: true }, { name: "React + state" }],
    groups: [
      { label: "QuickJS", values: [16.3, 14.9, 51.0] },
      { label: "Bun 1.4", values: [13.1, 13.3, 20.8] },
      { label: "Node 22", values: [14.4, 14.0, 18.9] },
    ],
    table: "jsAnim3000",
    source: sources.readme,
  },
  "vlist-scroll": {
    kind: "groups",
    title: "Variable-height list: scroll frame (median)",
    subtitle: "Real gpui window, chat messages, main thread from render to end of paint, median of 3 runs",
    unit: "ms",
    better: "lower",
    headline: { value: "about 2x", text: "faster per scrolled frame at 100, 1M and 10M messages" },
    series: [{ name: "blenderui-vlist", ours: true }, { name: "gpui list" }, { name: "gpui-component virtual_list" }],
    groups: [
      { label: "100", values: [0.54, 1.0, 1.04] },
      { label: "1,000,000", values: [0.71, 1.32, 1.3] },
      { label: "10,000,000", sub: "file log", values: [0.74, 1.44, 1.3] },
    ],
    table: "vlistGpui",
    source: "bindings/rust/blenderui-vlist-gpui/bench/results.md",
  },
  "vlist-memory": {
    kind: "groups",
    title: "Variable-height list: memory above an empty window",
    subtitle: "Real gpui window: how much the process grows for the list, not counting the messages (at 10M they are a file on disk)",
    unit: "MB",
    better: "lower",
    headline: { value: "6 MB", text: "for 10 million messages (gpui list: 910 MB)" },
    series: [{ name: "blenderui-vlist", ours: true }, { name: "gpui list" }, { name: "gpui-component virtual_list" }],
    groups: [
      { label: "100", values: [0, 0, 0] },
      { label: "1,000,000", values: [2, 86, 10] },
      { label: "10,000,000", sub: "file log", values: [6, 910, 151] },
    ],
    table: "vlistGpui",
    source: "bindings/rust/blenderui-vlist-gpui/bench/results.md",
  },
  "vlist-setup": {
    kind: "groups",
    title: "Variable-height list: setup time",
    subtitle: "From handing the list its messages to being ready to draw, real gpui, median of 3 runs",
    unit: "ms",
    better: "lower",
    headline: { value: "7.3 s to 82 ms", text: "building the 10M list (virtual_list vs ours)" },
    series: [{ name: "blenderui-vlist", ours: true }, { name: "gpui list" }, { name: "gpui-component virtual_list" }],
    groups: [
      { label: "100", values: [0, 0, 0.1] },
      { label: "1,000,000", values: [0.3, 58, 628] },
      { label: "10,000,000", sub: "file log", values: [82, 537, 7285] },
    ],
    table: "vlistGpui",
    source: "bindings/rust/blenderui-vlist-gpui/bench/results.md",
  },
  "vlist-index": {
    kind: "groups",
    title: "Changing one item's height",
    subtitle: "The height index against a flat Vec of running totals, release build, median of 5",
    unit: "ns",
    better: "lower",
    headline: { value: "20,000x", text: "cheaper than a flat array at 10M items" },
    series: [{ name: "HeightIndex", ours: true }, { name: "Flat Vec" }],
    groups: [
      { label: "10,000", values: [53, 684] },
      { label: "1,000,000", values: [112, 81849] },
      { label: "10,000,000", values: [182, 3825956] },
    ],
    table: "vlistIndex",
    source: "bindings/rust/blenderui-vlist/examples/bench_core.rs",
  },
} satisfies Record<string, ChartSpec>

export type ChartId = keyof typeof charts

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
  value: string
  unit: string
  label: string
  note: string
  ours: number
  theirs: { name: string; value: number; display: string }
  claim: string
  chart: ChartId
}[]
