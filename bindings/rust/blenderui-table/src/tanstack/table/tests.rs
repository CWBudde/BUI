use std::sync::Arc;

use super::*;
use crate::tanstack::{actions_column, select_column, ColumnMeta, Header, Record};

struct Person {
    name: String,
    age: u64,
    city: String,
}

impl Record for Person {
    const FIELDS: &'static [&'static str] = &["name", "age", "city"];
    fn field(&self, field: usize) -> Cell<'_> {
        match field {
            0 => Cell::Str(&self.name),
            1 => Cell::U64(self.age),
            _ => Cell::Str(&self.city),
        }
    }
}

fn people(n: usize) -> Arc<[Person]> {
    const CITIES: [&str; 4] = ["Paris", "Oslo", "Lima", "Kyoto"];
    (0..n).map(|i| Person { name: format!("user{:03}", (i * 37) % 1000), age: 20 + (i as u64 * 7) % 50, city: CITIES[i % 4].to_owned() }).collect()
}

fn columns() -> Arc<[ColumnDef<Person>]> {
    vec![
        select_column(),
        ColumnDef::accessor_key("name").header(Header::menu("Name")),
        ColumnDef::accessor_key("age").meta(ColumnMeta::right()),
        ColumnDef::accessor_key("city"),
        actions_column(),
    ]
    .into()
}

fn table(n: usize) -> Table<Person> {
    Table::new(TableOptions::new(columns(), people(n)).paginate(true)).unwrap()
}

fn page_ages(t: &Table<Person>) -> Vec<u64> {
    t.row_model().iter().map(|&r| t.data()[r as usize].age).collect()
}

#[test]
fn options_are_validated_at_the_boundary() {
    let bad: Arc<[ColumnDef<Person>]> = vec![ColumnDef::<Person>::accessor_key("nope")].into();
    assert_eq!(Table::new(TableOptions::new(bad, people(1))).err(), Some(TableError::UnknownKey("nope".into())));
    let dup: Arc<[ColumnDef<Person>]> = vec![ColumnDef::<Person>::accessor_key("age"), ColumnDef::accessor_key("age")].into();
    assert_eq!(Table::new(TableOptions::new(dup, people(1))).err(), Some(TableError::DuplicateId("age".into())));
    let mut o = TableOptions::new(columns(), people(1));
    o.initial_state.pagination.page_size = 0;
    assert_eq!(Table::new(o).err(), Some(TableError::ZeroPageSize));
}

#[test]
fn row_models_follow_tanstack() {
    let mut t = table(95);
    assert_eq!((t.core_row_count(), t.filtered_row_count(), t.row_count()), (95, 95, 95));
    assert_eq!(t.row_model().len(), 10);
    assert_eq!(t.page_count(), 10);
    t.set_column_filter_value("city", "KYO");
    assert_eq!(t.filtered_row_count(), 23); // Kyoto: 23 of 95
    assert_eq!(t.page_count(), 3);
    assert!(t.row_model().iter().all(|&r| t.data()[r as usize].city == "Kyoto"));
    t.set_column_filter_value("city", "");
    assert_eq!(t.filtered_row_count(), 95);
    t.set_global_filter("PARIS");
    assert_eq!(t.row_count(), 24);
    t.set_global_filter("");
    t.set_global_filter("user0");
    assert!(t.row_count() > 0 && t.row_count() < 95);
    assert!(matches!(t.core_row_model(), RowMask::Identity));
}

#[test]
fn sorting_single_multi_and_cycling() {
    let mut t = table(40);
    t.toggle_column_sorting("age", None);
    assert_eq!(t.column_sort("age"), ColumnSort::Ascending);
    assert!(page_ages(&t).windows(2).all(|w| w[0] <= w[1]));
    t.toggle_column_sorting("age", None);
    assert_eq!(t.column_sort("age"), ColumnSort::Descending);
    assert!(page_ages(&t).windows(2).all(|w| w[0] >= w[1]));
    t.toggle_column_sorting("age", None);
    assert_eq!(t.column_sort("age"), ColumnSort::Default);
    assert!(t.sorting().is_empty());
    // city, then age descending inside each city
    t.set_sorting(vec![SortingRule::new("city", false), SortingRule::new("age", true)]);
    t.set_page_size(40);
    let rows: Vec<(&str, u64)> = t.row_model().iter().map(|&r| (t.data()[r as usize].city.as_str(), t.data()[r as usize].age)).collect();
    assert!(rows.windows(2).all(|w| w[0].0 < w[1].0 || (w[0].0 == w[1].0 && w[0].1 >= w[1].1)), "{rows:?}");
    // a column with sorting disabled ignores the toggle
    t.toggle_column_sorting("select", None);
    assert_eq!(t.sorting().len(), 2);
}

#[test]
fn pagination_clamps_and_resets() {
    let mut t = table(95);
    t.next_page();
    t.last_page();
    assert_eq!(t.pagination().page_index, 9);
    assert_eq!(t.row_model().len(), 5);
    assert!(!t.can_next_page() && t.can_previous_page());
    t.next_page();
    assert_eq!(t.pagination().page_index, 9, "clamped");
    t.set_page_size(50);
    assert_eq!((t.pagination().page_index, t.page_count()), (1, 2), "the first row of the page stays in view");
    t.set_column_filter_value("city", "oslo");
    assert_eq!(t.pagination().page_index, 0, "autoResetPageIndex");
    t.first_page();
    t.previous_page();
    assert_eq!(t.pagination().page_index, 0);
    let o = TableOptions::new(columns(), people(0)).paginate(true);
    assert_eq!(Table::new(o).unwrap().page_count(), 0, "no rows, no pages");
}

#[test]
fn selection_across_pages_and_filters() {
    let mut t = table(30);
    assert!(!t.is_all_page_rows_selected() && !t.is_some_page_rows_selected());
    t.toggle_row_selected(3, None);
    assert!(t.is_some_page_rows_selected());
    t.toggle_all_page_rows_selected(true);
    assert!(t.is_all_page_rows_selected());
    assert_eq!(t.row_selection().count(), 10);
    t.next_page();
    assert!(!t.is_some_page_rows_selected(), "selection is by row, not by position");
    t.toggle_all_rows_selected(true);
    assert_eq!((t.filtered_selected_row_count(), t.selected_rows().len()), (30, 30));
    t.set_column_filter_value("city", "paris");
    assert_eq!(t.filtered_selected_row_count(), t.filtered_row_count());
    assert_eq!(t.row_selection().count(), 30, "the selection outlives the filter");
    // the checkbox column reads the selection
    t.set_column_filter_value("city", "");
    t.toggle_all_rows_selected(false);
    t.toggle_row_selected(t.data_row(2).unwrap(), Some(true));
    assert_eq!(t.cell(2, 0), Cell::U64(1));
    assert_eq!(t.cell(1, 0), Cell::U64(0));
    assert_eq!(t.header_check(0), Check::Indeterminate);
    t.toggle_all_page_rows_selected(true);
    assert_eq!(t.header_check(0), Check::Checked);
    assert_eq!(t.header_check(1), Check::Unchecked);
}

#[test]
fn visibility_flags_the_core_columns() {
    let mut t = table(5);
    t.toggle_column_visibility("city", Some(false));
    t.toggle_column_visibility("select", Some(false)); // enable_hiding is off
    let cols = t.core_columns();
    assert_eq!(cols.iter().map(|c| c.visible).collect::<Vec<_>>(), vec![true, true, true, false, true]);
    t.toggle_column_visibility("city", None);
    assert!(t.column_is_visible("city"));
    t.toggle_all_columns_visible(false);
    assert_eq!(t.core_columns().iter().filter(|c| c.visible).count(), 2, "select and actions stay");
    assert!(t.column_infos().iter().any(|c| c.id == "name" && !c.visible && c.can_sort));
}

#[test]
fn controlled_and_uncontrolled_state() {
    // uncontrolled: applied and reported
    let mut t = table(40);
    t.set_page_index(2);
    assert_eq!(t.pagination().page_index, 2);
    assert_eq!(t.take_changes(), vec![Change::Pagination(PaginationState { page_index: 2, page_size: 10 })]);
    // controlled: reported, not applied, until the parent passes it back
    let (cols, data) = (columns(), people(40));
    let controlled = |sorting: SortingState| PartialState {
        sorting: Some(sorting),
        pagination: Some(PaginationState { page_index: 1, page_size: 10 }),
        ..PartialState::default()
    };
    let mut t = Table::new(TableOptions::new(Arc::clone(&cols), Arc::clone(&data)).paginate(true).state(controlled(Vec::new()))).unwrap();
    assert_eq!(t.pagination().page_index, 1);
    t.toggle_column_sorting("age", None);
    assert!(t.sorting().is_empty(), "the parent owns the sorting");
    assert_eq!(t.take_changes(), vec![Change::Sorting(vec![SortingRule::new("age", false)])]);
    let sorted = controlled(vec![SortingRule::new("age", false)]);
    assert!(t.set_options(TableOptions::new(Arc::clone(&cols), Arc::clone(&data)).paginate(true).state(sorted.clone())).unwrap(), "the new controlled value is shown");
    assert_eq!(t.column_sort("age"), ColumnSort::Ascending);
    assert!(page_ages(&t).windows(2).all(|w| w[0] <= w[1]));
    // identical options change nothing
    assert!(!t.set_options(TableOptions::new(cols, data).paginate(true).state(sorted)).unwrap());
}

#[test]
fn page_cells_use_the_core_kinds() {
    let t = table(12);
    let cols = t.core_columns();
    assert_eq!(cols[0].render, CellRender::Checkbox);
    assert!(cols[1].header_menu && !cols[2].header_menu);
    assert_eq!(cols[2].align, crate::column::TextAlign::Right);
    assert_eq!(t.cell(0, 2), Cell::U64(t.data()[0].age));
    assert_eq!(t.cell(0, 4), Cell::Empty);
    assert_eq!(t.cell(99, 1), Cell::Empty);
}

#[test]
fn a_million_rows_sort_and_page_without_copying_rows() {
    let n = 1_000_000;
    let mut t = Table::new(TableOptions::new(columns(), people(n)).paginate(true)).unwrap();
    t.toggle_column_sorting("age", Some(true));
    assert_eq!(t.row_count(), n);
    assert_eq!(t.row_model().len(), 10, "only a page is materialised");
    assert!(page_ages(&t).iter().all(|&a| a == 69));
    t.last_page();
    assert_eq!(t.pagination().page_index, n / 10 - 1);
}
