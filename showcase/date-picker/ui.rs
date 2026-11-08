//! The picker's view: a few small components, styled with Tailwind utilities.
//! They hold a handle to the [`Picker`] (a `WeakEntity`, which is `Copy`) and tell it what happened.

use crate::icons;
use crate::model::{Month, Picker};
use crate::parse::PRESETS;
use crate::selection::Selection;
use blenderui::prelude::*;
use chrono::prelude::*;

/// A click listener that runs `f` on the picker.
fn click(picker: WeakEntity<Picker>, f: impl Fn(&mut Picker, &mut Context<Picker>) + 'static)
-> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
    move |_, _, cx| {
        picker.update(cx, |p, cx| f(p, cx));
    }
}

impl Render for Picker {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let me = cx.weak_entity();
        let summary = self.selection.summary().unwrap_or_default();
        rsx! {
            <body m-0 min-h-screen flex items-center justify-center bg-[#eceef1]>
                <div w-[800px] flex flex-col gap-4 rounded-[18px] bg-white px-[26px] pt-[22px] pb-[18px]
                     style="box-shadow: 0 16px 48px rgba(20,20,40,0.10), 0 2px 8px rgba(20,20,40,0.06)">
                    <SearchBox picker={me} query={self.query.clone()} />
                    <div h-px bg-[#f0f0f3] />
                    <div flex items-start gap-[18px]>
                        <Sidebar picker={me} active={self.query.trim().to_lowercase()} />
                        <Calendars picker={me} left={self.left} right={self.right} today={self.today} selection={self.selection.clone()} />
                    </div>
                    <Footer picker={me} summary={summary} />
                </div>
            </body>
        }
    }
}

/// The query field: a native text field that reports every keystroke (`live`), so the calendar
/// follows the typing. Selection, copy and paste, undo, the blinking caret and IME come with it.
#[component]
fn SearchBox(picker: WeakEntity<Picker>, query: String) {
    let typed = move |e: &ChangeEvent, _: &mut Window, cx: &mut App| {
        picker.update(cx, |p, cx| p.set_query(&e.value, cx));
    };
    let has_text = !query.is_empty();
    rsx! {
        <div flex items-center gap-[10px] h-9>
            {icons::calendar(0x4a4e57)}
            <input id="search" live type="text" placeholder="What dates are you looking for?" value={query}
                   flex-1 border-none text-[14px] onInput={typed} />
            {if has_text {
                <div id="clear" w-5 h-5 flex items-center justify-center cursor-pointer onClick={click(picker, |p, cx| p.set_query("", cx))}>
                    {icons::close(0x5b606a)}
                </div>
            }}
        </div>
    }
}

#[component]
fn Sidebar(picker: WeakEntity<Picker>, active: String) {
    rsx! {
        <div w-[140px] flex flex-col>
            {for label in PRESETS {
                <div id={format!("preset-{label}")} px-[10px] py-[6px] rounded-lg cursor-pointer text-[14px] text-[#33333a]
                     class={if active == label.to_lowercase() { "bg-[#e9eaf0]" } else { "hover:bg-[#f1f2f5]" }}
                     onClick={click(picker, move |p, cx| p.set_query(label, cx))}>
                    {label}
                </div>
            }}
        </div>
    }
}

/// Two months side by side, with the buttons that move them.
#[component]
fn Calendars(picker: WeakEntity<Picker>, left: Month, right: Month, today: NaiveDate, selection: Selection) {
    rsx! {
        <div flex-1 flex flex-col gap-2.5 pl-[18px] border-l border-[#f0f0f3]>
            <div flex items-center>
                <NavButton picker={picker} months={-1} />
                <div flex-1 flex justify-center><MonthPill month={left} /></div>
                <div flex-1 flex justify-center><MonthPill month={right} /></div>
                <NavButton picker={picker} months={1} />
            </div>
            <div flex gap-8>
                <MonthGrid picker={picker} month={left} today={today} selection={selection.clone()} />
                <MonthGrid picker={picker} month={right} today={today} selection={selection} />
            </div>
        </div>
    }
}

#[component]
fn NavButton(picker: WeakEntity<Picker>, months: i32) {
    rsx! {
        <div id={format!("nav-{months}")} w-7 h-7 flex items-center justify-center rounded-full cursor-pointer hover:bg-[#f1f2f5]
             onClick={click(picker, move |p, cx| p.page(months, cx))}>
            {icons::chevron_side(0x3d4048, months > 0)}
        </div>
    }
}

#[component]
fn MonthPill(month: Month) {
    rsx! {
        <div flex items-center gap-1.5 rounded-full border border-[#e6e6ea] px-3 py-1.5 text-[13px] font-medium text-[#33333a]>
            {month.title()}
            {icons::chevron_down(0x4a4e57)}
        </div>
    }
}

/// How a day is drawn.
#[derive(Clone, Copy, PartialEq)]
enum Tone {
    Plain,
    Future,
    Today,
    /// A selected day drawn as a circle.
    Endpoint,
    /// A selected day inside a range.
    Between,
}

impl Tone {
    fn of(day: NaiveDate, today: NaiveDate, selection: &Selection) -> Tone {
        match () {
            () if selection.is_endpoint(day) => Tone::Endpoint,
            () if selection.contains(day) => Tone::Between,
            () if day == today => Tone::Today,
            () if day > today => Tone::Future,
            () => Tone::Plain,
        }
    }

    fn classes(self) -> &'static str {
        match self {
            Tone::Plain => "rounded-full text-[#33333a] hover:bg-[#f1f2f5]",
            Tone::Future => "rounded-full text-[#a8abb3] hover:bg-[#f1f2f5]",
            Tone::Today => "rounded-full text-[#33333a] font-semibold shadow-[inset_0_0_0_1.5px_#b7bcc6]",
            Tone::Endpoint => "rounded-full bg-[#4f46e5] text-white font-medium",
            Tone::Between => "rounded-md bg-[#dfe3fc] text-[#33333a]",
        }
    }
}

#[component]
fn MonthGrid(picker: WeakEntity<Picker>, month: Month, today: NaiveDate, selection: Selection) {
    let blanks = month.first_day().weekday().num_days_from_monday();
    rsx! {
        <div style="display: grid; grid-template-columns: repeat(7, 34px); gap: 3px">
            {for name in ["M", "T", "W", "T", "F", "S", "S"] {
                <div w-[34px] h-6 flex items-center justify-center text-[11px] text-[#a8abb3]>{name}</div>
            }}
            {for _ in 0..blanks { <div w-[34px] h-[34px] /> }}
            {for day in month.days() {
                <DayCell picker={picker} day={day} tone={Tone::of(day, today, &selection)} />
            }}
        </div>
    }
}

#[component]
fn DayCell(picker: WeakEntity<Picker>, day: NaiveDate, tone: Tone) {
    rsx! {
        <div id={format!("day-{day}")} w-[34px] h-[34px] flex items-center justify-center text-sm cursor-pointer
             class={tone.classes()} onClick={click(picker, move |p, cx| p.pick_day(day, cx))}>
            {day.day().to_string()}
        </div>
    }
}

/// The selection in words, and the button that takes it.
#[component]
fn Footer(picker: WeakEntity<Picker>, summary: String) {
    let hint = if summary.is_empty() { "Type or select dates.".to_owned() } else { summary.clone() };
    let button = if summary.is_empty() { "bg-[#b9bbc1] text-[#f4f4f6]" } else { "bg-[#17171c] text-white hover:bg-[#3a3a42] cursor-pointer" };
    rsx! {
        <div flex items-center justify-between>
            <div text-sm text-[#8a8f98]>{hint}</div>
            <div id="apply" rounded-full px-[18px] py-[10px] text-[14px] class={button}
                 onClick={click(picker, |p, cx| p.apply(cx))}>
                "Apply dates"
            </div>
        </div>
    }
}
