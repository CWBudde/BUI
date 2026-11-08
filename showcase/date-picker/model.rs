//! The picker's state and what can happen to it. No drawing here.

use crate::parse;
use crate::selection::Selection;
use blenderui::prelude::Context;
use chrono::prelude::*;

/// A calendar month.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Month {
    pub year: i32,
    pub month: u32,
}

impl Month {
    pub fn of(day: NaiveDate) -> Month {
        Month { year: day.year(), month: day.month() }
    }

    pub fn shift(self, months: i32) -> Month {
        let total = self.year * 12 + self.month as i32 - 1 + months;
        Month { year: total.div_euclid(12), month: total.rem_euclid(12) as u32 + 1 }
    }

    pub fn first_day(self) -> NaiveDate {
        NaiveDate::from_ymd_opt(self.year, self.month, 1).expect("a valid month")
    }

    /// The days of the month, in order.
    pub fn days(self) -> impl Iterator<Item = NaiveDate> {
        let first = self.first_day();
        first.iter_days().take_while(move |d| d.month() == first.month())
    }

    pub fn title(self) -> String {
        const NAMES: [&str; 12] =
            ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
        format!("{} {}", NAMES[self.month as usize - 1], self.year)
    }
}

pub struct Picker {
    pub today: NaiveDate,
    pub query: String,
    pub selection: Selection,
    /// The first day of a range being clicked out.
    anchor: Option<NaiveDate>,
    /// The line of the last "Apply dates".
    pub applied: Option<String>,
    /// The months on show: the left one is where the selection starts, the right one where it ends.
    pub left: Month,
    pub right: Month,
}

impl Picker {
    pub fn new(today: NaiveDate) -> Picker {
        let this = Month::of(today);
        Picker { today, query: String::new(), selection: Selection::None, anchor: None, applied: None, left: this, right: this.shift(1) }
    }

    /// The query changed: re-read it and show the selection.
    pub fn set_query(&mut self, query: &str, cx: &mut Context<Self>) {
        self.query = query.to_owned();
        self.selection = parse::parse(query, self.today);
        self.anchor = None;
        self.show_selection();
        cx.notify();
    }

    /// A click on a day: the first one starts a range, the second ends it (and shows it).
    pub fn pick_day(&mut self, day: NaiveDate, cx: &mut Context<Self>) {
        self.query.clear();
        self.selection = match self.anchor.take() {
            Some(anchor) => Selection::range(anchor, day),
            None => {
                self.anchor = Some(day);
                Selection::Day(day)
            }
        };
        if self.anchor.is_none() {
            self.show_selection();
        }
        cx.notify();
    }

    /// The left month is the one the selection starts in, the right one the one it ends in (the next
    /// one when it is a single month).
    fn show_selection(&mut self) {
        if let Some((from, to)) = self.selection.bounds() {
            self.left = Month::of(from);
            self.right = if Month::of(to) == self.left { self.left.shift(1) } else { Month::of(to) };
        }
    }

    /// The left arrow pages the left month back, the right arrow the right month forward.
    pub fn page(&mut self, months: i32, cx: &mut Context<Self>) {
        if months < 0 {
            self.left = self.left.shift(months);
        } else {
            self.right = self.right.shift(months);
        }
        cx.notify();
    }

    pub fn apply(&mut self, cx: &mut Context<Self>) {
        if let Some(line) = self.selection.summary() {
            println!("Applied dates: {line}");
            self.applied = Some(line);
            cx.notify();
        }
    }
}
