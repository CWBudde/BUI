//! What the picker has selected, and how to ask about it without ever listing the days.
//!
//! A selection can span years ("2000 to today", "weekends since 2010"), so nothing here builds a
//! `Vec` of its days: membership, the first and last day and the count are all arithmetic.

use chrono::prelude::*;

/// Years the picker accepts. Half-typed input like "since 1 jan 20" reads as the year 20, which is
/// never what someone means, so such dates are not a selection.
pub const YEARS: std::ops::RangeInclusive<i32> = 1900..=2200;

/// A set of weekdays, as a bit mask (bit 0 is Monday).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WeekdaySet(u8);

impl WeekdaySet {
    pub const WEEKDAYS: WeekdaySet = WeekdaySet(0b001_1111);
    pub const WEEKEND: WeekdaySet = WeekdaySet(0b110_0000);

    pub fn one(day: Weekday) -> WeekdaySet {
        WeekdaySet(1 << day.num_days_from_monday())
    }

    pub fn contains(self, day: Weekday) -> bool {
        self.0 & (1 << day.num_days_from_monday()) != 0
    }

    fn days(self) -> impl Iterator<Item = Weekday> {
        (0..7u8).filter_map(|i| Weekday::try_from(i).ok()).filter(move |d| self.contains(*d))
    }

    /// "Weekends", "Fridays", "Weekdays": the heading of a summary.
    pub fn label(self) -> &'static str {
        const SINGLE: [&str; 7] = ["Mondays", "Tuesdays", "Wednesdays", "Thursdays", "Fridays", "Saturdays", "Sundays"];
        match self {
            Self::WEEKDAYS => "Weekdays",
            Self::WEEKEND => "Weekends",
            _ => self.days().find(|d| self == Self::one(*d)).map_or("Days", |d| SINGLE[d.num_days_from_monday() as usize]),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Selection {
    None,
    Day(NaiveDate),
    /// Every day from `from` to `to`, inclusive (`from <= to`).
    Range { from: NaiveDate, to: NaiveDate },
    /// The days of the week in `days` between `from` and `to`, which are both such days.
    Weekdays { days: WeekdaySet, from: NaiveDate, to: NaiveDate },
    /// A few separate days, sorted ("last 3 fridays").
    Dates { days: Vec<NaiveDate>, label: &'static str },
}

impl Selection {
    /// The days from `a` to `b` (in either order), or `None` outside [`YEARS`].
    pub fn range(a: NaiveDate, b: NaiveDate) -> Selection {
        if !YEARS.contains(&a.year()) || !YEARS.contains(&b.year()) {
            return Selection::None;
        }
        let (from, to) = (a.min(b), a.max(b));
        if from == to { Selection::Day(from) } else { Selection::Range { from, to } }
    }

    /// The days of the week in `days` between `a` and `b`, trimmed to the first and last of them;
    /// `None` when there are none.
    pub fn weekdays(days: WeekdaySet, a: NaiveDate, b: NaiveDate) -> Selection {
        if !YEARS.contains(&a.year()) || !YEARS.contains(&b.year()) {
            return Selection::None;
        }
        let (lo, hi) = (a.min(b), a.max(b));
        let first = (0..7).map(|i| lo + chrono::Duration::days(i)).find(|d| days.contains(d.weekday()));
        let last = (0..7).map(|i| hi - chrono::Duration::days(i)).find(|d| days.contains(d.weekday()));
        match (first, last) {
            (Some(from), Some(to)) if from <= to => Selection::Weekdays { days, from, to },
            _ => Selection::None,
        }
    }

    pub fn dates(mut days: Vec<NaiveDate>, label: &'static str) -> Selection {
        days.retain(|d| YEARS.contains(&d.year()));
        days.sort();
        days.dedup();
        if days.is_empty() { Selection::None } else { Selection::Dates { days, label } }
    }

    #[cfg(test)]
    pub fn is_none(&self) -> bool {
        matches!(self, Selection::None)
    }

    pub fn contains(&self, day: NaiveDate) -> bool {
        match self {
            Selection::None => false,
            Selection::Day(d) => *d == day,
            Selection::Range { from, to } => (*from..=*to).contains(&day),
            Selection::Weekdays { days, from, to } => {
                (*from..=*to).contains(&day) && days.contains(day.weekday())
            }
            Selection::Dates { days, .. } => days.binary_search(&day).is_ok(),
        }
    }

    /// Whether `day` is drawn as a circle: every selected day, except the inside of a plain range,
    /// where only the two ends are.
    pub fn is_endpoint(&self, day: NaiveDate) -> bool {
        match self {
            Selection::Range { from, to } => day == *from || day == *to,
            _ => self.contains(day),
        }
    }

    /// The first and last selected day, to bring the calendar to the selection.
    pub fn bounds(&self) -> Option<(NaiveDate, NaiveDate)> {
        match self {
            Selection::None => None,
            Selection::Day(d) => Some((*d, *d)),
            Selection::Range { from, to } | Selection::Weekdays { from, to, .. } => Some((*from, *to)),
            Selection::Dates { days, .. } => days.first().copied().zip(days.last().copied()),
        }
    }

    /// How many days are selected.
    pub fn len(&self) -> usize {
        match self {
            Selection::None => 0,
            Selection::Day(_) => 1,
            Selection::Range { from, to } => (*to - *from).num_days() as usize + 1,
            Selection::Weekdays { days, from, to } => days
                .days()
                .map(|wd| {
                    // the first `wd` on or after `from`, then one every seven days
                    let skip = (wd.num_days_from_monday() as i64 - from.weekday().num_days_from_monday() as i64)
                        .rem_euclid(7);
                    let first = *from + chrono::Duration::days(skip);
                    if first > *to { 0 } else { ((*to - first).num_days() / 7) as usize + 1 }
                })
                .sum(),
            Selection::Dates { days, .. } => days.len(),
        }
    }

    /// The line under the calendar: "Sep 10 – Oct 20, 2026 · 41 days".
    pub fn summary(&self) -> Option<String> {
        let (a, b) = self.bounds()?;
        let n = self.len();
        let days = if n == 1 { "1 day".to_string() } else { format!("{n} days") };
        let span = if a == b {
            fmt_day_year(a)
        } else if a.year() == b.year() {
            format!("{} – {}", fmt_day(a), fmt_day_year(b))
        } else {
            format!("{} – {}", fmt_day_year(a), fmt_day_year(b))
        };
        Some(match self {
            Selection::Weekdays { days: set, .. } => format!("{} · {span} · {days}", set.label()),
            Selection::Dates { label, .. } => format!("{label} · {span} · {days}"),
            _ => format!("{span} · {days}"),
        })
    }
}

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

fn fmt_day(d: NaiveDate) -> String {
    format!("{} {}", MONTHS[d.month0() as usize], d.day())
}

fn fmt_day_year(d: NaiveDate) -> String {
    format!("{}, {}", fmt_day(d), d.year())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn a_range_is_arithmetic() {
        let all_time = Selection::range(d(1900, 1, 1), d(2200, 12, 31));
        assert!(all_time.contains(d(2026, 11, 15)));
        assert_eq!(all_time.len(), 109_938);
        assert_eq!(Selection::range(d(2026, 9, 10), d(2026, 9, 10)), Selection::Day(d(2026, 9, 10)));
        assert_eq!(Selection::range(d(20, 1, 1), d(2026, 1, 1)), Selection::None);
    }

    #[test]
    fn weekdays_are_trimmed_and_counted() {
        // Oct 1 to Nov 15, 2026: Thursday to Sunday; the weekends in it
        let sel = Selection::weekdays(WeekdaySet::WEEKEND, d(2026, 10, 1), d(2026, 11, 15));
        assert_eq!(sel.bounds(), Some((d(2026, 10, 3), d(2026, 11, 15))));
        assert_eq!(sel.len(), 14);
        assert!(sel.contains(d(2026, 10, 4)) && !sel.contains(d(2026, 10, 5)));
        // a span with none of them in it
        assert_eq!(Selection::weekdays(WeekdaySet::one(Weekday::Fri), d(2026, 11, 2), d(2026, 11, 4)), Selection::None);
    }

    #[test]
    fn summaries() {
        assert_eq!(
            Selection::range(d(2026, 9, 10), d(2026, 10, 20)).summary().unwrap(),
            "Sep 10 – Oct 20, 2026 · 41 days"
        );
        let fridays = Selection::dates(vec![d(2026, 10, 30), d(2026, 11, 6), d(2026, 11, 13)], "Fridays");
        assert_eq!(fridays.summary().unwrap(), "Fridays · Oct 30 – Nov 13, 2026 · 3 days");
        let weekends = Selection::weekdays(WeekdaySet::WEEKEND, d(2026, 10, 1), d(2026, 11, 15));
        assert_eq!(weekends.summary().unwrap(), "Weekends · Oct 3 – Nov 15, 2026 · 14 days");
        assert_eq!(Selection::Day(d(2026, 11, 15)).summary().unwrap(), "Nov 15, 2026 · 1 day");
        assert_eq!(Selection::None.summary(), None);
    }
}
