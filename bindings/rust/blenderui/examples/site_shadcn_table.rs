//! shadcn/ui's "payments" Data Table in Blender's theme: a select column, a
//! status badge, a sortable email header with a menu, an amount formatted as
//! dollars and right-aligned, a row-actions column, a filter input, the view
//! options and the pagination footer. Written in `rsx!` / `#[component]` syntax.
//!
//! ```text
//! cargo run --release --example site_shadcn_table
//! ```

#[path = "site/common.rs"]
mod common;

use std::cell::Cell as StdCell;
use std::sync::Arc;

use blenderui::prelude::*;
use blenderui::shadcn::*;

const STATUS: [&str; 4] = ["pending", "processing", "success", "failed"];
const NAMES: [&str; 12] = ["ken99", "abe45", "monserrat44", "silas22", "carmella", "m", "example", "jason78", "nina", "omar", "lena", "tomas"];
const HOSTS: [&str; 4] = ["yahoo.com", "gmail.com", "example.com", "hotmail.com"];

/// shadcn's sample row: `{ id, amount, status, email }`.
struct Payment {
    id: String,
    status: &'static str,
    email: String,
    amount: f64,
}

impl Record for Payment {
    const FIELDS: &'static [&'static str] = &["id", "status", "email", "amount"];

    fn field(&self, field: usize) -> Cell<'_> {
        match field {
            0 => Cell::Str(&self.id),
            1 => Cell::Str(self.status),
            2 => Cell::Str(&self.email),
            _ => Cell::F64(self.amount, 2),
        }
    }
}

fn payments() -> Arc<[Payment]> {
    (0..84)
        .map(|i| {
            let h = (i as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 20;
            Payment {
                id: format!("{:08x}", (h as u32) ^ 0x728e_d52f),
                status: STATUS[(h >> 3) as usize % 4],
                email: format!("{}@{}", NAMES[(h >> 5) as usize % NAMES.len()], HOSTS[(h >> 9) as usize % HOSTS.len()]),
                amount: 50.0 + (h % 90_000) as f64 / 100.0,
            }
        })
        .collect()
}

/// `columns.tsx` of the shadcn recipe.
fn columns() -> Arc<[ColumnDef<Payment>]> {
    vec![
        select_column(),
        ColumnDef::accessor_key("status").header("Status").size(150.0).badge(|s| match s {
            "success" => BadgeVariant::Default,
            "failed" => BadgeVariant::Destructive,
            "processing" => BadgeVariant::Outline,
            _ => BadgeVariant::Secondary,
        }),
        ColumnDef::accessor_key("email").header(Header::menu("Email")).size(400.0),
        ColumnDef::accessor_key("amount").header("Amount").size(150.0).meta(ColumnMeta::right().usd()),
        actions_column(),
    ]
    .into()
}

struct Page {
    handle: StdCell<Option<TableHandle>>,
}

impl Render for Page {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let table = use_table();
        self.handle.set(Some(table));
        let (columns, data) = (use_memo(columns, ()), use_memo(payments, ()));
        html().styled().child(body().m_0().p_4().child(rsx! {
            <DataTable
                table={table}
                columns={columns}
                data={data}
                filter_column="email"
                filter_placeholder="Filter emails..."
                on_row_action={|row, _, _| println!("actions for row {row}")}
                w={px(820.0)}
                h={px(340.0)}
            />
        }))
    }
}

fn main() {
    let shot = common::screenshot_arg();
    let app = if shot.is_some() { Application::headless() } else { Application::new() };
    app.run(move |cx| {
        let mut page = None;
        let win = cx
            .open_window(common::options(852.0, 372.0, "Payments"), |_, cx| {
                let p = cx.new(|_| Page { handle: StdCell::new(None) });
                page = Some(p);
                p
            })
            .unwrap();
        if let Some(path) = shot {
            for _ in 0..4 {
                cx.draw(win.id);
            }
            // a state worth a picture: two rows checked and the email menu open
            let table = page.unwrap().read(cx).handle.get().expect("rendered");
            for row in [1, 3] {
                table.dispatch(Command::TogglePageRow(row), cx);
            }
            for _ in 0..3 {
                cx.draw(win.id);
            }
            cx.simulate_click(win.id, 16.0 + 40.0 + 150.0 + 100.0, 55.0);
            for _ in 0..3 {
                cx.draw(win.id);
            }
            let (w, h, px) = cx.draw(win.id);
            common::save(&path, w, h, &px);
        }
    });
}
