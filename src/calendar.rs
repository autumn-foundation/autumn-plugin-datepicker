//! The calendar markup: one month as an APG date grid.
//!
//! The pickers put it in a popover, and the month route returns it. Each
//! cell is a `<td data-date>`. The keyboard core moves the roving tab stop.

use chrono::{Datelike, NaiveDate};
use maud::{Markup, html};

use crate::grid::MonthGrid;
use crate::locale::CalendarLocale;
use crate::rules::{DateRules, weekday_to_iso};
use crate::value::DateRange;

/// The path of the month route.
pub const MONTH_PATH: &str = "/_datepicker/month";

/// The prefix of each month route key. A form field whose name starts with
/// it cannot use a picker: `hx-include` sends the field under its name.
pub const QUERY_PREFIX: &str = "dp-";

/// What the calendar marks as selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Selection {
    /// Nothing.
    #[default]
    None,
    /// One day.
    Day(NaiveDate),
    /// A range. The end is `None` while the user picks it.
    Range(NaiveDate, Option<NaiveDate>),
}

impl Selection {
    /// The selection of a range input pair. A lone end acts as the start.
    #[must_use]
    pub const fn range(start: Option<NaiveDate>, end: Option<NaiveDate>) -> Self {
        match (start, end) {
            (Some(a), b) => Self::Range(a, b),
            (None, Some(b)) => Self::Range(b, None),
            (None, None) => Self::None,
        }
    }
}

/// What a click on a day does. The pack script reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Set a `date` input.
    #[default]
    Date,
    /// Set the date part of a `datetime-local` input.
    DateTime,
    /// Set the start, then the end, of a range.
    Range,
}

impl Mode {
    /// The query and `data-*` name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Date => "date",
            Self::DateTime => "datetime",
            Self::Range => "range",
        }
    }

    /// Parse [`as_str`](Self::as_str).
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        [Self::Date, Self::DateTime, Self::Range]
            .into_iter()
            .find(|mode| mode.as_str() == name)
    }
}

/// One month of a picker, ready to render.
#[derive(Debug, Clone)]
pub struct Calendar<'a> {
    id: &'a str,
    month: MonthGrid,
    rules: &'a DateRules,
    locale: &'a CalendarLocale,
    selection: Selection,
    today: Option<NaiveDate>,
    mode: Mode,
}

impl<'a> Calendar<'a> {
    /// The calendar of the field `id`, showing `month`.
    #[must_use]
    pub const fn new(
        id: &'a str,
        month: MonthGrid,
        rules: &'a DateRules,
        locale: &'a CalendarLocale,
    ) -> Self {
        Self {
            id,
            month,
            rules,
            locale,
            selection: Selection::None,
            today: None,
            mode: Mode::Date,
        }
    }

    /// Mark the selection.
    #[must_use]
    pub const fn selection(mut self, selection: Selection) -> Self {
        self.selection = selection;
        self
    }

    /// Mark today with `aria-current="date"`.
    #[must_use]
    pub const fn today(mut self, today: NaiveDate) -> Self {
        self.today = Some(today);
        self
    }

    /// Set what a click on a day does.
    #[must_use]
    pub const fn mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self
    }

    /// The month route URL without the `month` key.
    #[must_use]
    pub fn source_url(&self) -> String {
        let mut url = format!(
            "{MONTH_PATH}?dp-id={}&dp-mode={}&dp-ws={}",
            query_escape(self.id),
            self.mode.as_str(),
            weekday_to_iso(self.month.week_start())
        );
        if let Some(today) = self.today {
            url.push_str("&dp-today=");
            url.push_str(&today.to_string());
        }
        for (key, value) in self.rules.to_query() {
            url.push_str("&dp-");
            url.push_str(key);
            url.push('=');
            url.push_str(&query_escape(&value));
        }
        url
    }

    /// `true` when `day` is in the selection.
    fn is_selected(&self, day: NaiveDate) -> bool {
        match self.selection {
            Selection::None => false,
            Selection::Day(d) => d == day,
            Selection::Range(start, None) => start == day,
            Selection::Range(start, Some(end)) => DateRange::new(start, end).contains(day),
        }
    }

    /// The day that holds the tab stop: the first selected day, else today,
    /// else the first enabled day. `None` when no day in the month is
    /// enabled.
    fn tab_stop(&self) -> Option<NaiveDate> {
        let days = || {
            (1..=self.month.days())
                .filter_map(|d| self.month.first().with_day(d))
                .filter(|d| self.rules.is_enabled(*d))
        };
        days()
            .find(|d| self.is_selected(*d))
            .or_else(|| self.today.filter(|t| days().any(|d| d == *t)))
            .or_else(|| days().next())
    }

    /// `true` when today gets `aria-current`: no day of this month is selected.
    fn marks_today(&self) -> bool {
        !(1..=self.month.days())
            .filter_map(|d| self.month.first().with_day(d))
            .any(|d| self.is_selected(d))
    }

    /// Render the calendar.
    #[must_use]
    pub fn render(&self) -> Markup {
        let id = self.id;
        let heading = format!("{id}-calendar-heading");
        let src = self.source_url();
        let stop = self.tab_stop();
        let marks_today = self.marks_today();
        let nav = |label: &str, target: Option<MonthGrid>, name: &str, glyph: &str| {
            let target = target.filter(|m| self.rules.allows_month(m));
            html! {
                button type="button" class="autumn-datepicker-nav" data-nav=(name)
                    aria-label=(label)
                    data-month=[target.map(|m| m.key())]
                    hx-get=[target.map(|m| format!("{src}&dp-month={}", m.key()))]
                    hx-target="closest .autumn-datepicker-calendar"
                    hx-swap="outerHTML"
                    hx-include="closest .autumn-datepicker-field"
                    disabled[target.is_none()] {
                    span aria-hidden="true" { (glyph) }
                }
            }
        };
        html! {
            div class="autumn-datepicker-calendar" data-autumn-datepicker-calendar
                data-mode=(self.mode.as_str()) data-month=(self.month.key()) data-src=(src)
                data-today=[self.today.map(|d| d.to_string())]
                data-min=[self.rules.min_date().map(|d| d.to_string())]
                data-max=[self.rules.max_date().map(|d| d.to_string())] {
                div class="autumn-datepicker-header" {
                    (nav(self.locale.previous_month(), self.month.previous(), "previous", "‹"))
                    div class="autumn-datepicker-heading" id=(heading) aria-live="polite" {
                        (self.locale.title(&self.month))
                    }
                    (nav(self.locale.next_month(), self.month.next(), "next", "›"))
                }
                table class="autumn-datepicker-grid" role="grid" aria-labelledby=(heading)
                    aria-multiselectable=[(self.mode == Mode::Range).then_some("true")]
                    data-autumn-roving="[data-date]" data-autumn-orientation="horizontal"
                    data-autumn-wrap="false" {
                    thead {
                        tr {
                            @for weekday in self.month.weekdays() {
                                th scope="col" abbr=(self.locale.weekday_name(weekday))
                                    class="autumn-datepicker-weekday" {
                                    (self.locale.weekday_short(weekday))
                                }
                            }
                        }
                    }
                    tbody {
                        @for week in self.month.weeks() {
                            tr {
                                @for cell in week {
                                    @match cell {
                                        Some(day) => (self.day(day, stop, marks_today)),
                                        None => td class="autumn-datepicker-blank" {},
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// One day cell.
    fn day(&self, day: NaiveDate, stop: Option<NaiveDate>, marks_today: bool) -> Markup {
        let enabled = self.rules.is_enabled(day);
        let selected = self.is_selected(day);
        let is_today = self.today == Some(day);
        let mut class = String::from("autumn-datepicker-day");
        if is_today {
            class.push_str(" autumn-datepicker-day--today");
        }
        if let Selection::Range(start, end) = self.selection {
            let end = end.unwrap_or(start);
            let range = DateRange::new(start, end);
            if day == range.start() {
                class.push_str(" autumn-datepicker-day--range-start");
            }
            if day == range.end() {
                class.push_str(" autumn-datepicker-day--range-end");
            }
            if range.contains(day) && day != range.start() && day != range.end() {
                class.push_str(" autumn-datepicker-day--in-range");
            }
        }
        html! {
            td class=(class) data-date=(day.to_string())
                tabindex=(if stop == Some(day) { "0" } else { "-1" })
                aria-selected=[selected.then_some("true")]
                aria-current=[(is_today && marks_today).then_some("date")]
                aria-disabled=[(!enabled).then_some("true")] {
                (day.day())
            }
        }
    }
}

/// `true` when `id` can go into an element id and a query string:
/// 1 to 64 bytes of `[A-Za-z0-9_.:-]`, and `[`/`]` for nested forms. It
/// does not start with [`QUERY_PREFIX`].
#[must_use]
pub fn is_valid_id(id: &str) -> bool {
    (1..=64).contains(&id.len())
        && !id.starts_with(QUERY_PREFIX)
        && id.bytes().all(|b| {
            b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b':' | b'-' | b'[' | b']')
        })
}

/// Percent-encode a query value. Ids and rule values are ASCII, so this
/// encodes only the bytes that have a meaning in a query.
fn query_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for b in value.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':' | b',') {
            out.push(char::from(b));
        } else {
            const HEX: &[u8; 16] = b"0123456789ABCDEF";
            out.push('%');
            out.push(char::from(HEX[usize::from(b >> 4)]));
            out.push(char::from(HEX[usize::from(b & 0xF)]));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Weekday;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("a valid test date")
    }

    fn render(c: &Calendar<'_>) -> String {
        c.render().into_string()
    }

    fn nov() -> MonthGrid {
        MonthGrid::new(2026, 11, Weekday::Sun).expect("month")
    }

    #[test]
    fn mode_names_round_trip() {
        for mode in [Mode::Date, Mode::DateTime, Mode::Range] {
            assert_eq!(Mode::parse(mode.as_str()), Some(mode));
        }
        assert_eq!(Mode::parse("week"), None);
    }

    #[test]
    fn ids_are_checked() {
        for ok in ["due", "start_at", "items[0].due", "a-b:c"] {
            assert!(is_valid_id(ok), "{ok}");
        }
        for bad in [
            "",
            "a b",
            "a\"b",
            "<x>",
            "a&b",
            "a=b",
            "dp-month",
            &"x".repeat(65),
        ] {
            assert!(!is_valid_id(bad), "{bad:?}");
        }
    }

    #[test]
    fn the_grid_has_the_apg_roles_and_the_core_attributes() {
        let (rules, locale) = (DateRules::new(), CalendarLocale::english());
        let html = render(&Calendar::new("due", nov(), &rules, &locale));
        assert!(html.contains(r#"role="grid""#), "{html}");
        assert!(
            html.contains(r#"aria-labelledby="due-calendar-heading""#),
            "{html}"
        );
        assert!(html.contains(r#"id="due-calendar-heading""#), "{html}");
        assert!(html.contains(r#"aria-live="polite""#), "{html}");
        assert!(html.contains(">November 2026<"), "{html}");
        assert!(
            html.contains(r#"data-autumn-roving="[data-date]""#),
            "{html}"
        );
        assert!(
            html.contains(r#"data-autumn-orientation="horizontal""#),
            "{html}"
        );
        assert!(html.contains(r#"data-autumn-wrap="false""#), "{html}");
        assert!(html.contains(r#"<th scope="col" abbr="Sunday""#), "{html}");
        assert_eq!(html.matches("data-date=").count(), 30, "{html}");
        assert!(html.contains(r#"data-date="2026-11-30""#), "{html}");
        assert!(!html.contains("style="), "{html}");
    }

    #[test]
    fn the_week_start_orders_the_headers() {
        let (rules, locale) = (DateRules::new(), CalendarLocale::english());
        let monday = MonthGrid::new(2026, 11, Weekday::Mon).expect("month");
        let html = render(&Calendar::new("due", monday, &rules, &locale));
        let mo = html.find(r#"abbr="Monday""#).expect("Monday");
        let su = html.find(r#"abbr="Sunday""#).expect("Sunday");
        assert!(mo < su, "{html}");
    }

    #[test]
    fn one_cell_is_the_tab_stop() {
        let (rules, locale) = (DateRules::new(), CalendarLocale::english());
        let c = Calendar::new("due", nov(), &rules, &locale)
            .today(date(2026, 11, 3))
            .selection(Selection::Day(date(2026, 11, 20)));
        let html = render(&c);
        assert_eq!(html.matches(r#"tabindex="0""#).count(), 1, "{html}");
        assert!(
            html.contains(r#"data-date="2026-11-20" tabindex="0" aria-selected="true""#),
            "the selection wins: {html}"
        );
        assert!(html.contains("autumn-datepicker-day--today"), "{html}");
        assert!(
            !html.contains("aria-current"),
            "only one marked day: {html}"
        );
    }

    #[test]
    fn today_is_current_when_nothing_is_selected() {
        let (rules, locale) = (DateRules::new(), CalendarLocale::english());
        let c = Calendar::new("due", nov(), &rules, &locale).today(date(2026, 11, 3));
        let html = render(&c);
        assert!(
            html.contains(r#"data-date="2026-11-03" tabindex="0" aria-current="date""#),
            "{html}"
        );
    }

    #[test]
    fn disabled_days_get_aria_disabled() {
        let rules = DateRules::new().min(date(2026, 11, 10)).disable_weekends();
        let locale = CalendarLocale::english();
        let html = render(&Calendar::new("due", nov(), &rules, &locale));
        assert!(
            html.contains(r#"data-date="2026-11-09" tabindex="-1" aria-disabled="true""#),
            "{html}"
        );
        assert!(
            html.contains(r#"data-date="2026-11-14" tabindex="-1" aria-disabled="true""#),
            "Saturday: {html}"
        );
        assert!(
            html.contains(r#"data-date="2026-11-10" tabindex="0""#),
            "the first enabled day is the stop: {html}"
        );
    }

    #[test]
    fn navigation_buttons_are_htmx_requests() {
        let (rules, locale) = (DateRules::new(), CalendarLocale::english());
        let c = Calendar::new("due", nov(), &rules, &locale).today(date(2026, 11, 5));
        let html = render(&c);
        assert!(html.contains(r#"aria-label="Previous month""#), "{html}");
        assert!(html.contains(r#"aria-label="Next month""#), "{html}");
        assert!(
            html.contains(r#"hx-get="/_datepicker/month?dp-id=due&amp;dp-mode=date&amp;dp-ws=7&amp;dp-today=2026-11-05&amp;dp-month=2026-10""#),
            "{html}"
        );
        assert!(
            html.contains(r#"hx-target="closest .autumn-datepicker-calendar""#),
            "{html}"
        );
        assert!(html.contains(r#"hx-swap="outerHTML""#), "{html}");
        assert!(
            html.contains(r#"hx-include="closest .autumn-datepicker-field""#),
            "{html}"
        );
    }

    #[test]
    fn a_month_outside_the_rules_has_a_disabled_button() {
        let rules = DateRules::new()
            .min(date(2026, 11, 1))
            .max(date(2026, 11, 30));
        let locale = CalendarLocale::english();
        let html = render(&Calendar::new("due", nov(), &rules, &locale));
        assert_eq!(html.matches(" disabled").count(), 2, "{html}");
        assert!(
            html.contains("dp-min=2026-11-01"),
            "rules are in the URL: {html}"
        );
    }

    #[test]
    fn a_range_marks_each_day_in_it() {
        let (rules, locale) = (DateRules::new(), CalendarLocale::english());
        let c = Calendar::new("stay", nov(), &rules, &locale)
            .mode(Mode::Range)
            .selection(Selection::Range(date(2026, 11, 5), Some(date(2026, 11, 7))));
        let html = render(&c);
        assert!(html.contains(r#"aria-multiselectable="true""#), "{html}");
        assert_eq!(html.matches(r#"aria-selected="true""#).count(), 3, "{html}");
        assert!(
            html.contains("autumn-datepicker-day--range-start"),
            "{html}"
        );
        assert!(html.contains("autumn-datepicker-day--range-end"), "{html}");
        assert!(html.contains("autumn-datepicker-day--in-range"), "{html}");
        assert!(html.contains(r#"data-mode="range""#), "{html}");
    }

    #[test]
    fn text_is_escaped() {
        let rules = DateRules::new();
        let locale = CalendarLocale::english().title_template("<b>{month}</b>");
        let html = render(&Calendar::new("due", nov(), &rules, &locale));
        assert!(html.contains("&lt;b&gt;November&lt;/b&gt;"), "{html}");
    }
}
