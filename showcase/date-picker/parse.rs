//! Natural-language dates: what someone types in the search box becomes a [`Selection`].
//!
//! A query is lower-cased and tidied ([`tidy`]), then tried as
//!
//! 1. a count of weekdays: "last 3 fridays", "next 2 saturdays";
//! 2. weekdays within a span: "weekends since october", "mondays in march 2026", "fridays last month";
//! 3. a span: "from sep 10 to oct 20", "between 1 and 5 may", "last week", "q1 2026", "ytd";
//! 4. a single day: "tomorrow", "in 2 weeks", "christmas", "end of next month", "the 3rd", "5 may 2024".
//!
//! The pieces that no rule above covers ("next friday", "2026-03-04") are left to `chrono-english`.
//!
//! "This week / month / year / quarter" mean from its start up to today; "last" and "next" mean the
//! whole previous or next one; "last N weeks / months" are the N calendar weeks or months that end
//! with the current one, up to today.

use crate::selection::{Selection, WeekdaySet, YEARS};
use chrono::prelude::*;
use chrono::{Duration, Months};
use chrono_english::{Dialect, parse_date_string};

/// The sidebar presets; each is also something one can type.
pub const PRESETS: [&str; 7] =
    ["Today", "Yesterday", "This week", "Last week", "This month", "Last month", "Last 3 months"];

pub fn parse(query: &str, today: NaiveDate) -> Selection {
    let q = tidy(query);
    let words: Vec<&str> = q.split(' ').filter(|w| !w.is_empty()).collect();
    if words.is_empty() {
        return Selection::None;
    }
    counted_weekdays(&words, today)
        .or_else(|| weekdays_within(&words, today))
        .or_else(|| bounds(&words, today).map(|(a, b)| Selection::range(a, b)))
        .unwrap_or(Selection::None)
}

/// Lower case, no commas, apostrophes or ordinal suffixes ("May 5th," -> "may 5"), and no filler.
fn tidy(query: &str) -> String {
    let lower = query.to_lowercase().replace([',', '\''], " ").replace(['–', '—'], " – ");
    let words = lower.split_whitespace().filter(|w| !matches!(*w, "the" | "please" | "show" | "me"));
    let mut tidy: Vec<&str> = Vec::new();
    for word in words.map(strip_ordinal) {
        // "5th of may": the day, then the month
        let after_day = tidy.last().is_some_and(|p| p.bytes().all(|b| b.is_ascii_digit()));
        if !(word == "of" && after_day) {
            tidy.push(word);
        }
    }
    tidy.join(" ")
}

fn strip_ordinal(word: &str) -> &str {
    for suffix in ["st", "nd", "rd", "th"] {
        if let Some(n) = word.strip_suffix(suffix)
            && !n.is_empty()
            && n.bytes().all(|b| b.is_ascii_digit())
        {
            return n;
        }
    }
    word
}

// ---- units of time ----

#[derive(Clone, Copy, PartialEq)]
enum Unit {
    Day,
    Week,
    Month,
    Quarter,
    Year,
}

impl Unit {
    fn parse(word: &str) -> Option<Unit> {
        Some(match word.trim_end_matches('s') {
            "day" => Unit::Day,
            "week" => Unit::Week,
            "month" => Unit::Month,
            "quarter" => Unit::Quarter,
            "year" => Unit::Year,
            _ => return None,
        })
    }

    /// The first day of the unit that holds `d` (weeks start on Monday).
    fn start(self, d: NaiveDate) -> NaiveDate {
        let first = |year, month| NaiveDate::from_ymd_opt(year, month, 1).unwrap_or(d);
        match self {
            Unit::Day => d,
            Unit::Week => d - Duration::days(i64::from(d.weekday().num_days_from_monday())),
            Unit::Month => first(d.year(), d.month()),
            Unit::Quarter => first(d.year(), d.month0() / 3 * 3 + 1),
            Unit::Year => first(d.year(), 1),
        }
    }

    /// The last day of the unit that holds `d`.
    fn end(self, d: NaiveDate) -> Option<NaiveDate> {
        self.shift(self.start(d), 1)?.pred_opt()
    }

    /// `d` moved by `n` units (the day of the month is kept, or clamped to the month's end).
    fn shift(self, d: NaiveDate, n: i32) -> Option<NaiveDate> {
        let months = |n: i32| {
            if n >= 0 { d.checked_add_months(Months::new(n.unsigned_abs())) } else { d.checked_sub_months(Months::new(n.unsigned_abs())) }
        };
        match self {
            Unit::Day => d.checked_add_signed(Duration::days(i64::from(n))),
            Unit::Week => d.checked_add_signed(Duration::days(7 * i64::from(n))),
            Unit::Month => months(n),
            Unit::Quarter => months(n.checked_mul(3)?),
            Unit::Year => months(n.checked_mul(12)?),
        }
    }

    /// The unit that holds `d`, whole.
    fn whole(self, d: NaiveDate) -> Option<(NaiveDate, NaiveDate)> {
        Some((self.start(d), self.end(d)?))
    }
}

/// "3", "three", "a": how many (1 to 9999).
fn count(word: &str) -> Option<i32> {
    const WORDS: [&str; 12] = ["one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve"];
    let n = match word {
        "a" | "an" => 1,
        _ => match WORDS.iter().position(|w| *w == word) {
            Some(i) => i as i32 + 1,
            None => word.parse().ok()?,
        },
    };
    (1..10_000).contains(&n).then_some(n)
}

fn month_of(word: &str) -> Option<u32> {
    const NAMES: [&str; 12] =
        ["january", "february", "march", "april", "may", "june", "july", "august", "september", "october", "november", "december"];
    let word = if word == "sept" { "sep" } else { word };
    NAMES.iter().position(|m| word.len() >= 3 && m.starts_with(word)).map(|i| i as u32 + 1)
}

fn year_of(word: &str) -> Option<i32> {
    let year: i32 = word.parse().ok()?;
    (word.len() == 4 && YEARS.contains(&year)).then_some(year)
}

// ---- weekdays ----

/// "friday", "fridays", "fri", "weekends": the days a word names.
fn weekday_set(word: &str) -> Option<WeekdaySet> {
    match word {
        "weekend" | "weekends" => return Some(WeekdaySet::WEEKEND),
        "weekday" | "weekdays" => return Some(WeekdaySet::WEEKDAYS),
        _ => {}
    }
    let name = match word {
        "tues" | "thurs" | "weds" => word,
        _ => word.trim_end_matches('s'),
    };
    let day = match name {
        "mon" | "monday" => Weekday::Mon,
        "tue" | "tues" | "tuesday" => Weekday::Tue,
        "wed" | "weds" | "wednesday" => Weekday::Wed,
        "thu" | "thur" | "thurs" | "thursday" => Weekday::Thu,
        "fri" | "friday" => Weekday::Fri,
        "sat" | "saturday" => Weekday::Sat,
        "sun" | "sunday" => Weekday::Sun,
        _ => return None,
    };
    Some(WeekdaySet::one(day))
}

/// "last 3 fridays": the N closest such days before today, or after it ("next").
fn counted_weekdays(words: &[&str], today: NaiveDate) -> Option<Selection> {
    let [way @ ("last" | "past" | "previous" | "next"), n, name] = words else { return None };
    let (n, set) = (count(n)?, weekday_set(name)?);
    let step = if *way == "next" { 1 } else { -1 };
    let days = (1..).map(|i| today + Duration::days(i64::from(step) * i)).filter(|d| set.contains(d.weekday()));
    Some(Selection::dates(days.take(n as usize).collect(), set.label()))
}

/// "weekends since october", "mondays in march 2026", "fridays last month", "weekends" (nothing yet).
fn weekdays_within(words: &[&str], today: NaiveDate) -> Option<Selection> {
    let (head, rest) = match words {
        ["every" | "all", head, rest @ ..] | [head, rest @ ..] => (head, rest),
        [] => return None,
    };
    let set = weekday_set(head)?;
    let rest = match rest {
        ["in" | "during" | "of" | "over" | "within", more @ ..] => more,
        _ => rest,
    };
    if rest.is_empty() {
        return Some(Selection::None); // a kind of day, not yet a selection
    }
    let (from, to) = match rest {
        ["since" | "from", date @ ..] => (bounds(date, today)?.0, today),
        _ => bounds(rest, today)?,
    };
    Some(Selection::weekdays(set, from, to))
}

// ---- spans and days ----

/// The first and last day a phrase names: a range, a span or one day.
fn bounds(words: &[&str], today: NaiveDate) -> Option<(NaiveDate, NaiveDate)> {
    range(words, today).or_else(|| span_or_day(words, today))
}

fn span_or_day(words: &[&str], today: NaiveDate) -> Option<(NaiveDate, NaiveDate)> {
    span(words, today).or_else(|| day(words, today).map(|d| (d, d)))
}

/// "from A to B", "between A and B", "A - B": from where A starts to where B ends.
fn range(words: &[&str], today: NaiveDate) -> Option<(NaiveDate, NaiveDate)> {
    let (between, words) = match words {
        ["between", rest @ ..] => (true, rest),
        ["from", rest @ ..] => (false, rest),
        _ => (false, words),
    };
    let at = words.iter().position(|w| matches!(*w, "to" | "until" | "till" | "through" | "thru" | "–" | "-") || (between && *w == "and"))?;
    let (a, b) = (&words[..at], &words[at + 1..]);
    let from = span_or_day(a, today)?.0;
    let to = span_or_day(b, today)?.1;
    Some((from.min(to), from.max(to)))
}

/// A named stretch of time: "last week", "october", "2025", "q2", "last 3 months", "week of may 4".
fn span(words: &[&str], today: NaiveDate) -> Option<(NaiveDate, NaiveDate)> {
    let unit = |w: &str| Unit::parse(w);
    match words {
        ["ytd"] | ["year", "to", "date"] => Some((Unit::Year.start(today), today)),
        ["mtd"] | ["month", "to", "date"] => Some((Unit::Month.start(today), today)),
        ["wtd"] | ["week", "to", "date"] => Some((Unit::Week.start(today), today)),
        [when @ ("this" | "current" | "last" | "previous" | "next"), "weekend"] => {
            let weeks = match *when {
                "last" | "previous" => -1,
                "next" => 1,
                _ => 0,
            };
            let saturday = Unit::Week.shift(today, weeks)?.checked_add_signed(Duration::days(5 - i64::from(today.weekday().num_days_from_monday())))?;
            Some((saturday, saturday.succ_opt()?))
        }
        ["this" | "current", u] => Some((unit(u)?.start(today), today)),
        ["last" | "previous", u] => unit(u)?.shift(today, -1).and_then(|d| unit(u)?.whole(d)),
        ["next", u] => unit(u)?.shift(today, 1).and_then(|d| unit(u)?.whole(d)),
        ["last" | "past" | "previous", n, u] => {
            let (n, u) = (count(n)?, unit(u)?);
            Some((u.start(u.shift(today, 1 - n)?), today))
        }
        ["next", n, u] => Some((today.succ_opt()?, unit(u)?.shift(today, count(n)?)?)),
        ["week", "of", date @ ..] => Unit::Week.whole(day(date, today)?),
        [which @ ("first" | "last"), "week", "of", month @ ..] => {
            let (first, last) = month_span(month, today)?;
            Some(if *which == "first" { (first, first + Duration::days(6)) } else { (last - Duration::days(6), last) })
        }
        ["in" | "during", rest @ ..] => span(rest, today),
        [year] if year_of(year).is_some() => {
            let first = NaiveDate::from_ymd_opt(year_of(year)?, 1, 1)?;
            Unit::Year.whole(first)
        }
        [quarter, year @ ..] if quarter.len() == 2 && quarter.starts_with('q') => {
            let q: u32 = quarter[1..].parse().ok().filter(|q| (1..=4).contains(q))?;
            let year = match year {
                [] => today.year(),
                [y] => year_of(y)?,
                _ => return None,
            };
            Unit::Quarter.whole(NaiveDate::from_ymd_opt(year, q * 3 - 2, 1)?)
        }
        _ => month_span(words, today),
    }
}

/// "october" or "october 2025": that whole month (this year when none is given).
fn month_span(words: &[&str], today: NaiveDate) -> Option<(NaiveDate, NaiveDate)> {
    let (month, year) = match words {
        [m] => (month_of(m)?, today.year()),
        [m, y] => (month_of(m)?, year_of(y)?),
        _ => return None,
    };
    Unit::Month.whole(NaiveDate::from_ymd_opt(year, month, 1)?)
}

/// A date that is one day: "tomorrow", "in 2 weeks", "end of next month", "christmas", "the 3rd", ...
fn day(words: &[&str], today: NaiveDate) -> Option<NaiveDate> {
    const HOLIDAYS: [(&str, u32, u32); 11] = [
        ("new years day", 1, 1),
        ("new years", 1, 1),
        ("new year", 1, 1),
        ("valentines day", 2, 14),
        ("independence day", 7, 4),
        ("halloween", 10, 31),
        ("christmas eve", 12, 24),
        ("christmas day", 12, 25),
        ("christmas", 12, 25),
        ("boxing day", 12, 26),
        ("new years eve", 12, 31),
    ];
    let named = words.join(" ");
    let date = match words {
        ["today" | "now"] => Some(today),
        ["yesterday"] => today.pred_opt(),
        ["tomorrow"] => today.succ_opt(),
        ["in", n, u] => Unit::parse(u)?.shift(today, count(n)?),
        [n, u, "from", "now"] => Unit::parse(u)?.shift(today, count(n)?),
        [n, u, "ago"] => Unit::parse(u)?.shift(today, -count(n)?),
        [edge @ ("end" | "start" | "beginning"), "of", unit @ ..] => {
            let (unit, within) = unit_of(unit, today)?;
            if *edge == "end" { unit.end(within) } else { Some(unit.start(within)) }
        }
        [d] if d.parse::<u32>().is_ok_and(|d| (1..=31).contains(&d)) => today.with_day(d.parse().ok()?),
        _ => HOLIDAYS
            .iter()
            .find(|(name, ..)| *name == named)
            .and_then(|&(_, month, day)| NaiveDate::from_ymd_opt(today.year(), month, day)),
    };
    date.or_else(|| chrono_english_day(&named, today)).filter(|d| YEARS.contains(&d.year()))
}

/// "month", "this year", "next week": the unit, and a day in it.
fn unit_of(words: &[&str], today: NaiveDate) -> Option<(Unit, NaiveDate)> {
    match words {
        [u] | ["this", u] => Some((Unit::parse(u)?, today)),
        ["next", u] => Some((Unit::parse(u)?, Unit::parse(u)?.shift(today, 1)?)),
        ["last" | "previous", u] => Some((Unit::parse(u)?, Unit::parse(u)?.shift(today, -1)?)),
        _ => None,
    }
}

/// What the other rules leave: "next friday", "2026-03-04", "5 may 2024", "9/10".
fn chrono_english_day(text: &str, today: NaiveDate) -> Option<NaiveDate> {
    let noon = Local.from_local_datetime(&today.and_hms_opt(12, 0, 0)?).earliest()?;
    parse_date_string(text, noon, Dialect::Us).ok().map(|d| d.naive_local().date())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Sunday: the week is Nov 9 to 15.
    fn today() -> NaiveDate {
        d(2026, 11, 15)
    }

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn bounds_of(query: &str) -> Option<(NaiveDate, NaiveDate)> {
        parse(query, today()).bounds()
    }

    fn assert_span(query: &str, from: NaiveDate, to: NaiveDate) {
        assert_eq!(bounds_of(query), Some((from, to)), "{query}");
    }

    #[test]
    fn ranges() {
        assert_eq!(parse("September 10 to October 20", today()), Selection::range(d(2026, 9, 10), d(2026, 10, 20)));
        assert_span("oct 5 until oct 9", d(2026, 10, 5), d(2026, 10, 9));
        assert_span("october 20 to september 10", d(2026, 9, 10), d(2026, 10, 20)); // either order
        assert_span("between sep 1 and sep 5", d(2026, 9, 1), d(2026, 9, 5));
        assert_span("from october to december", d(2026, 10, 1), d(2026, 12, 31));
        assert_span("sep 10–oct 20", d(2026, 9, 10), d(2026, 10, 20));
        assert_span("from 5th of may to the 9th of may", d(2026, 5, 5), d(2026, 5, 9));
    }

    #[test]
    fn counted_weekdays() {
        assert_eq!(
            parse("Last 3 Fridays", today()),
            Selection::dates(vec![d(2026, 10, 30), d(2026, 11, 6), d(2026, 11, 13)], "Fridays")
        );
        assert_eq!(parse("next 2 saturdays", today()), Selection::dates(vec![d(2026, 11, 21), d(2026, 11, 28)], "Saturdays"));
        assert_eq!(parse("past three sundays", today()).len(), 3);
    }

    #[test]
    fn weekdays_within_a_span() {
        let weekends = parse("Weekends since October", today());
        assert_eq!((weekends.bounds(), weekends.len()), (Some((d(2026, 10, 3), d(2026, 11, 15))), 14));
        let fridays = parse("fridays since sep 1", today());
        assert_eq!((fridays.bounds(), fridays.len()), (Some((d(2026, 9, 4), d(2026, 11, 13))), 11));
        assert_eq!(parse("weekends this month", today()).len(), 5);
        assert_eq!(parse("mondays in october", today()).len(), 4);
        assert_eq!(parse("every friday last month", today()).len(), 5);
        assert_eq!(parse("weekdays last week", today()).len(), 5);
        assert_eq!(parse("weekends", today()), Selection::None);
        assert_eq!(parse("weekends in 1990", today()).len(), 104);
    }

    #[test]
    fn spans() {
        assert_span("today", today(), today());
        assert_span("this week", d(2026, 11, 9), today());
        assert_span("last week", d(2026, 11, 2), d(2026, 11, 8));
        assert_span("next week", d(2026, 11, 16), d(2026, 11, 22));
        assert_span("this month", d(2026, 11, 1), today());
        assert_span("last month", d(2026, 10, 1), d(2026, 10, 31));
        assert_span("next month", d(2026, 12, 1), d(2026, 12, 31));
        assert_span("this year", d(2026, 1, 1), today());
        assert_span("last year", d(2025, 1, 1), d(2025, 12, 31));
        assert_span("this quarter", d(2026, 10, 1), today());
        assert_span("last quarter", d(2026, 7, 1), d(2026, 9, 30));
        assert_span("this weekend", d(2026, 11, 14), today());
        assert_span("next weekend", d(2026, 11, 21), d(2026, 11, 22));
        assert_span("last weekend", d(2026, 11, 7), d(2026, 11, 8));
        assert_span("last 7 days", d(2026, 11, 9), today());
        assert_span("past 2 weeks", d(2026, 11, 2), today());
        assert_span("last 3 months", d(2026, 9, 1), today());
        assert_span("last 2 years", d(2025, 1, 1), today());
        assert_span("next 3 days", d(2026, 11, 16), d(2026, 11, 18));
        assert_span("next 2 weeks", d(2026, 11, 16), d(2026, 11, 29));
        assert_span("week of nov 4", d(2026, 11, 2), d(2026, 11, 8));
        assert_span("ytd", d(2026, 1, 1), today());
        assert_span("mtd", d(2026, 11, 1), today());
    }

    #[test]
    fn named_periods() {
        assert_span("october", d(2026, 10, 1), d(2026, 10, 31));
        assert_span("in feb 2024", d(2024, 2, 1), d(2024, 2, 29));
        assert_span("2025", d(2025, 1, 1), d(2025, 12, 31));
        assert_span("q1 2026", d(2026, 1, 1), d(2026, 3, 31));
        assert_span("q4", d(2026, 10, 1), d(2026, 12, 31));
        assert_span("first week of october", d(2026, 10, 1), d(2026, 10, 7));
        assert_span("last week of october", d(2026, 10, 25), d(2026, 10, 31));
    }

    #[test]
    fn single_days() {
        assert_eq!(parse("tomorrow", today()), Selection::Day(d(2026, 11, 16)));
        assert_eq!(parse("yesterday", today()), Selection::Day(d(2026, 11, 14)));
        assert_eq!(parse("3 days ago", today()), Selection::Day(d(2026, 11, 12)));
        assert_eq!(parse("in 2 weeks", today()), Selection::Day(d(2026, 11, 29)));
        assert_eq!(parse("in a month", today()), Selection::Day(d(2026, 12, 15)));
        assert_eq!(parse("christmas", today()), Selection::Day(d(2026, 12, 25)));
        assert_eq!(parse("end of month", today()), Selection::Day(d(2026, 11, 30)));
        assert_eq!(parse("start of next month", today()), Selection::Day(d(2026, 12, 1)));
        assert_eq!(parse("end of last year", today()), Selection::Day(d(2025, 12, 31)));
        assert_eq!(parse("the 3rd", today()), Selection::Day(d(2026, 11, 3)));
        assert_eq!(parse("may 5th", today()), Selection::Day(d(2026, 5, 5)));
        assert_eq!(parse("september 10", today()), Selection::Day(d(2026, 9, 10)));
        assert_eq!(parse("5 may 2024", today()), Selection::Day(d(2024, 5, 5)));
        assert_eq!(parse("2026-03-04", today()), Selection::Day(d(2026, 3, 4)));
        assert_eq!(parse("next friday", today()), Selection::Day(d(2026, 11, 20)));
    }

    #[test]
    fn nonsense_and_half_typed_input_select_nothing() {
        for q in ["", "  ", "gibberish", "last", "last 3", "sep 10 to", "to oct 9", "weekends since", "q9", "1 jan 20"] {
            assert!(parse(q, today()).is_none(), "{q:?} -> {:?}", parse(q, today()));
        }
    }

    #[test]
    fn presets_parse() {
        for preset in PRESETS {
            assert!(!parse(preset, today()).is_none(), "{preset}");
        }
        assert_eq!(parse("Today", today()), Selection::Day(today()));
    }
}
