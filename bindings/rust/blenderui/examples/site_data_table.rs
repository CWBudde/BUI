//! A one-million-row data table in Blender's theme: virtualized, sortable,
//! resizable columns, a pinned column, row selection and a status line.
//!
//! ```text
//! cargo run --release --example site_data_table
//! ```

#[path = "site/common.rs"]
mod common;

use blenderui::prelude::*;
use blenderui::table::{Cell, CellSource, Column, ColumnSort, DataTable, TableDelegate, TableState};

const ROWS: usize = 1_000_000;
const NAMES: [&str; 8] = ["Cube", "Sphere", "Suzanne", "Torus", "Cone", "Plane", "Cylinder", "Icosphere"];
const MATERIALS: [&str; 6] = ["Principled BSDF", "Emission", "Glass BSDF", "Diffuse", "Glossy", "Toon"];
const STATUS: [&str; 4] = ["Ready", "Baking", "Modified", "Linked"];

/// Cells are computed from the row number: nothing is stored per row.
struct Meshes;

fn hash(r: usize, salt: u64) -> u64 {
    let mut x = (r as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ salt.wrapping_mul(0xD6E8_FEB8_6659_FD93);
    x ^= x >> 29;
    x.wrapping_mul(0xBF58_476D_1CE4_E5B9) >> 20
}

impl CellSource for Meshes {
    fn cell(&self, row: usize, col: usize) -> Cell<'_> {
        match col {
            0 => Cell::U64(row as u64),
            1 => Cell::Str(NAMES[hash(row, 1) as usize % NAMES.len()]),
            2 => Cell::Str(MATERIALS[hash(row, 2) as usize % MATERIALS.len()]),
            3 => Cell::U64(hash(row, 3) % 900_000 + 8),
            4 => Cell::U64(hash(row, 3) % 900_000 / 2 + 6),
            5 => Cell::F64((hash(row, 5) % 90_000) as f64 / 100.0, 2),
            _ => Cell::Str(STATUS[hash(row, 6) as usize % STATUS.len()]),
        }
    }
}

impl TableDelegate for Meshes {
    fn columns_count(&self, _: &App) -> usize {
        7
    }
    fn rows_count(&self, _: &App) -> usize {
        ROWS
    }
    fn column(&self, ix: usize, _: &App) -> Column {
        match ix {
            0 => Column::new("id", "ID").sortable().width(px(84.0)).fixed_left().text_right(),
            1 => Column::new("name", "Object").sortable().width(px(150.0)),
            2 => Column::new("material", "Material").sortable().width(px(140.0)),
            3 => Column::new("verts", "Vertices").sortable().width(px(100.0)).text_right(),
            4 => Column::new("faces", "Faces").sortable().width(px(90.0)).text_right(),
            5 => Column::new("size", "Size (MB)").sortable().width(px(100.0)).text_right(),
            _ => Column::new("status", "Status").sortable().width(px(110.0)),
        }
    }
}

struct Page {
    table: Entity<TableState<Meshes>>,
}

impl Render for Page {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (rows, selected, vis, sort) = {
            let s = self.table.read(cx);
            (s.rows(), s.selected_rows().count(), s.visible_range().rows().clone(), s.sorted_by())
        };
        let sorted = match sort {
            Some((c, ColumnSort::Descending)) => format!("sorted by column {c}, descending"),
            Some((c, _)) => format!("sorted by column {c}, ascending"),
            None => "unsorted".to_string(),
        };
        html().styled().child(
            body().m_0().child(
                div().flex().flex_col().w(px(760.0)).h(px(460.0))
                    .child(div().h(px(436.0)).child(DataTable::new(&self.table).stripe(true)))
                    .child(
                        div().flex().items_center().h(px(24.0)).px_2().gap_4().text_size(px(12.0))
                            .child(format!("{rows} rows"))
                            .child(format!("showing {}..{}", vis.start, vis.end))
                            .child(format!("{selected} selected"))
                            .child(sorted),
                    ),
            ),
        )
    }
}

fn main() {
    let shot = common::screenshot_arg();
    let app = if shot.is_some() { Application::headless() } else { Application::new() };
    app.run(move |cx| {
        let win = cx
            .open_window(common::options(760.0, 460.0, "Data table"), |window, cx| {
                let table = cx.new(|cx| TableState::new(Meshes, window, cx).row_header(true));
                if shot.is_some() {
                    // a state worth a picture: sorted, a few rows selected, scrolled a little
                    table.update(cx, |s, c| {
                        s.sort_by(3, ColumnSort::Descending, window, c);
                        let row = s.row_at(2);
                        s.set_selected_row(row, c);
                    });
                }
                cx.new(|_| Page { table })
            })
            .unwrap();
        if let Some(path) = shot {
            for _ in 0..3 {
                cx.draw(win.id);
            }
            // shift-click extends the selection like in Blender's lists
            cx.simulate_click(win.id, 300.0, 100.0);
            cx.simulate_click_with(win.id, 300.0, 140.0, false, true);
            cx.draw(win.id);
            let (w, h, px) = cx.draw(win.id);
            common::save(&path, w, h, &px);
        }
    });
}
