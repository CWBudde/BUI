//! The render-farm dashboard's data and stylesheet (site_dashboard and
//! site_dashboard_builder). Colours are translucent over the `styled`
//! background, so the page follows Blender's dark and light themes.

#![allow(dead_code)]

pub const STATS: [(&str, &str, &str, u32); 4] = [
    ("Active jobs", "3", "+1 since 09:00", 38),
    ("Frames today", "12,480", "+18% vs. yesterday", 72),
    ("GPU utilisation", "87%", "8 of 8 nodes busy", 87),
    ("Queue ETA", "1h 42m", "6 jobs waiting", 55),
];

pub const JOBS: [(&str, &str, &str, u32); 5] = [
    ("robot_arm.blend", "Cycles", "149 / 240", 62),
    ("forest_flyby.blend", "EEVEE", "880 / 1200", 73),
    ("ocean_sim.blend", "Cycles", "175 / 500", 35),
    ("product_shot.blend", "Cycles", "1 / 1", 100),
    ("title_card.blend", "EEVEE", "12 / 90", 13),
];

pub const NODES: [(&str, &str, u32); 5] = [
    ("gpu-01 · RTX 4090", "busy", 96),
    ("gpu-02 · RTX 4090", "busy", 91),
    ("gpu-03 · RTX 3080", "busy", 78),
    ("cpu-01 · 64 cores", "idle", 12),
    ("cpu-02 · 64 cores", "down", 0),
];

pub const CSS: &str = r#"
body { margin: 0; padding: 14px 16px; }
header { display: flex; align-items: center; gap: 8px; margin-bottom: 12px; }
header .title { flex: 1; display: flex; align-items: baseline; gap: 10px; }
h1 { font-size: 20px; margin: 0; }
h2 { font-size: 13px; margin: 0 0 8px 0; opacity: 0.8; }
.muted { opacity: 0.6; font-size: 12px; }
.stats { display: grid; grid-template-columns: repeat(4, 1fr); gap: 10px; margin-bottom: 12px; }
.card { display: flex; flex-direction: column; gap: 4px; padding: 10px 12px; border-radius: 8px;
        background: rgba(128, 128, 128, 0.16); border: 1px solid rgba(128, 128, 128, 0.22); }
.card strong { font-size: 22px; }
.delta { font-size: 11px; color: #3fa66b; }
.meter { height: 4px; border-radius: 2px; background: rgba(128, 128, 128, 0.3); }
.meter .fill { height: 4px; border-radius: 2px; background: linear-gradient(90deg, #4772b3, #6f9be0); }
.meter.small { width: 70px; }
.columns { display: flex; gap: 10px; }
.panel { padding: 10px 12px; border-radius: 8px; background: rgba(128, 128, 128, 0.12); }
.jobs { flex: 1; min-width: 0; }
.nodes { flex: none; width: 230px; }
.job { display: flex; align-items: center; gap: 8px; height: 26px; }
.job .name { width: 124px; white-space: nowrap; }
.job .frames { width: 70px; text-align: right; }
.job .bar { flex: 1; }
.job .pct { width: 36px; text-align: right; font-size: 12px; }
.badge { font-size: 10px; padding: 1px 6px; border-radius: 8px; color: white; width: 40px; text-align: center; }
.badge.cycles { background: #4772b3; }
.badge.eevee { background: #3a9b7a; }
.node { display: flex; align-items: center; gap: 8px; height: 24px; }
.node .name { flex: 1; font-size: 12px; white-space: nowrap; }
.dot { width: 8px; height: 8px; border-radius: 4px; }
.dot.busy { background: #e0a030; }
.dot.idle { background: #3fa66b; }
.dot.down { background: #d0504a; }
.row { display: flex; align-items: center; gap: 6px; margin-top: 8px; font-size: 12px; }
"#;
