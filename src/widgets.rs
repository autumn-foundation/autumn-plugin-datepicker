//! The pickers. Each one is a [`FieldRenderer`] on a native input.
//!
//! With JavaScript off, the native input works alone. The pack script shows
//! the toggle button, which opens a calendar or a time list in a native
//! popover.

use chrono::format::{Item, StrftimeItems};
use chrono::{DateTime, NaiveDate, NaiveTime, Timelike, Utc};
use chrono_tz::Tz;
use maud::{Markup, html};
use serde::Serialize;

use autumn_web::form::{Changeset, FieldContext, FieldOptions, FieldRenderer, custom_input_with};

use crate::calendar::{Calendar, Mode, Selection, is_valid_id};
use crate::grid::MonthGrid;
use crate::locale::CalendarLocale;
use crate::rules::DateRules;
use crate::value::{format_datetime, format_time, parse_date, parse_datetime, parse_time};

/// The class of each native input. `FieldContext::control` adds
/// `autumn-field__input` in front of it.
const INPUT_CLASS: &str = "autumn-datepicker-input";

/// Borrow owned attribute values for `FieldContext::control`.
fn borrowed<'a>(attrs: &'a [(&'static str, String)]) -> Vec<(&'static str, &'a str)> {
    attrs.iter().map(|(k, v)| (*k, v.as_str())).collect()
}

/// The button that opens a popover. It has `hidden` until the pack script
/// runs, so a page without JavaScript shows no dead button.
fn toggle(target: &str, label: &str, popup: &str, icon: &str) -> Markup {
    html! {
        button type="button" class="autumn-popover__trigger autumn-datepicker-toggle"
            command="toggle-popover" commandfor=(target) aria-controls=(target)
            aria-haspopup=(popup) data-popover-toggle=(target) aria-label=(label) hidden {
            span class={ "autumn-datepicker-icon autumn-datepicker-icon--" (icon) } aria-hidden="true" {}
        }
    }
}

/// A date picker: `<input type="date">` and a calendar.
#[derive(Debug, Clone, Default)]
pub struct DatePicker {
    pub(crate) rules: DateRules,
    pub(crate) locale: CalendarLocale,
    pub(crate) today: Option<NaiveDate>,
    pub(crate) zone: Option<Tz>,
    pub(crate) inline: bool,
}

impl DatePicker {
    /// A picker with no limits and English text. Today comes from the
    /// ambient clock, in UTC.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the rules: `min`, `max` and the disabled days.
    #[must_use]
    pub fn rules(mut self, rules: DateRules) -> Self {
        self.rules = rules;
        self
    }

    /// The first day a user can pick. The input gets `min` too.
    #[must_use]
    pub fn min(mut self, date: NaiveDate) -> Self {
        self.rules = self.rules.min(date);
        self
    }

    /// The last day a user can pick. The input gets `max` too.
    #[must_use]
    pub fn max(mut self, date: NaiveDate) -> Self {
        self.rules = self.rules.max(date);
        self
    }

    /// Set the names, the labels and the week start.
    #[must_use]
    pub fn locale(mut self, locale: CalendarLocale) -> Self {
        self.locale = locale;
        self
    }

    /// Set today. Pass the date in the user zone, for example from the
    /// `Clock` and `TimeZone` extractors.
    #[must_use]
    pub const fn today(mut self, today: NaiveDate) -> Self {
        self.today = Some(today);
        self
    }

    /// Use `zone` for today when [`today`](Self::today) is not set.
    #[must_use]
    pub const fn time_zone(mut self, zone: Tz) -> Self {
        self.zone = Some(zone);
        self
    }

    /// Show the calendar below the input, not in a popover.
    #[must_use]
    pub const fn inline(mut self) -> Self {
        self.inline = true;
        self
    }
}

/// The date of `now` in `zone`, or in UTC.
pub(crate) fn today_from(now: DateTime<Utc>, zone: Option<Tz>) -> NaiveDate {
    zone.map_or_else(
        || now.date_naive(),
        |tz| now.with_timezone(&tz).date_naive(),
    )
}

impl DatePicker {
    /// Today: the set date, else the ambient clock in the zone, else in UTC.
    fn today_in(&self) -> NaiveDate {
        self.today
            .unwrap_or_else(|| today_from(autumn_web::time::ambient_now(), self.zone))
    }

    /// The first month to show: the one of `anchor`, else of today, moved
    /// inside `min..=max`.
    fn first_month(&self, anchor: Option<NaiveDate>, today: NaiveDate) -> MonthGrid {
        let mut day = anchor.unwrap_or(today);
        if let Some(min) = self.rules.min_date() {
            day = day.max(min);
        }
        if let Some(max) = self.rules.max_date() {
            day = day.min(max);
        }
        MonthGrid::containing(day, self.locale.first_weekday())
    }

    /// The calendar, in a popover or inline. An id that the month route
    /// does not accept gives no calendar: the native input stays alone.
    fn calendar_markup(
        &self,
        id: &str,
        selection: Selection,
        anchor: Option<NaiveDate>,
        mode: Mode,
    ) -> Markup {
        if !is_valid_id(id) {
            return html! {};
        }
        let today = self.today_in();
        let calendar = Calendar::new(
            id,
            self.first_month(anchor, today),
            &self.rules,
            &self.locale,
        )
        .selection(selection)
        .today(today)
        .mode(mode)
        .render();
        // One live region. It is outside the calendar that htmx replaces.
        let status = html! {
            div class="autumn-datepicker-status" role="status" aria-live="polite" {}
        };
        if self.inline {
            return html! {
                div class="autumn-datepicker-inline" hidden { (status) (calendar) }
            };
        }
        let dialog = format!("{id}-dialog");
        html! {
            (toggle(&dialog, self.locale.choose_date(), "dialog", "calendar"))
            div id=(dialog) class="autumn-popover__panel autumn-datepicker-dialog" popover
                role="dialog" aria-label=(self.locale.calendar())
                data-side="bottom" data-align="start" data-autumn-popover {
                (status)
                (calendar)
            }
        }
    }

    /// `min` and `max` input attributes, with `suffix` after each date.
    fn limits(&self, min_suffix: &str, max_suffix: &str) -> Vec<(&'static str, String)> {
        let mut attrs = Vec::new();
        if let Some(min) = self.rules.min_date() {
            attrs.push(("min", format!("{min}{min_suffix}")));
        }
        if let Some(max) = self.rules.max_date() {
            attrs.push(("max", format!("{max}{max_suffix}")));
        }
        attrs
    }
}

impl FieldRenderer for DatePicker {
    fn name(&self) -> &'static str {
        "datepicker"
    }

    fn render(&self, field: &FieldContext<'_>) -> Markup {
        let selected = field.value().and_then(parse_date);
        let value = field
            .value()
            .map(|raw| selected.map_or_else(|| raw.to_owned(), |d| d.to_string()));
        let mut attrs = vec![
            ("type", "date".to_owned()),
            ("class", INPUT_CLASS.to_owned()),
        ];
        attrs.extend(value.map(|v| ("value", v)));
        attrs.extend(self.limits("", ""));
        let selection = selected.map_or(Selection::Empty, Selection::Day);
        html! {
            div class="autumn-datepicker-field" data-autumn-datepicker="date"
                data-autumn-datepicker-inline[self.inline] {
                (field.control("input", &borrowed(&attrs), html! {}))
                (self.calendar_markup(field.id(), selection, selected, Mode::Date))
            }
        }
    }
}

/// Render a [`DatePicker`] bound to `changeset[field]`.
#[must_use]
pub fn date_picker<T: Serialize>(
    changeset: &Changeset<T>,
    field: &str,
    label: &str,
    picker: &DatePicker,
    options: &FieldOptions<'_>,
) -> Markup {
    custom_input_with(changeset, field, label, picker, options)
}

/// A time picker: `<input type="time">` and a list of time slots.
#[derive(Debug, Clone)]
pub struct TimePicker {
    pub(crate) first: NaiveTime,
    pub(crate) last: NaiveTime,
    pub(crate) step_minutes: u32,
    pub(crate) label_format: &'static str,
    pub(crate) locale: CalendarLocale,
    /// `true` after [`between`](TimePicker::between). Only then does the
    /// input get `min` and `max`.
    pub(crate) limited: bool,
}

impl Default for TimePicker {
    fn default() -> Self {
        Self {
            first: NaiveTime::MIN,
            last: NaiveTime::from_hms_opt(23, 59, 0).unwrap_or(NaiveTime::MIN),
            step_minutes: 30,
            label_format: "%H:%M",
            locale: CalendarLocale::english(),
            limited: false,
        }
    }
}

impl TimePicker {
    /// Slots every 30 minutes from 00:00 to 23:30, shown as `HH:MM`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The first and the last slot. The input gets `min` and `max` too.
    ///
    /// # Panics
    ///
    /// Panics when `first` is after `last`. A list cannot go past midnight.
    #[must_use]
    pub fn between(mut self, first: NaiveTime, last: NaiveTime) -> Self {
        assert!(
            first <= last,
            "TimePicker::between: {first} is after {last}; a list cannot go past midnight"
        );
        self.first = first;
        self.last = last;
        self.limited = true;
        self
    }

    /// The minutes between two slots.
    ///
    /// # Panics
    ///
    /// Panics when `minutes` is 0 or more than 720.
    #[must_use]
    pub fn step_minutes(mut self, minutes: u32) -> Self {
        assert!(
            (1..=720).contains(&minutes),
            "step_minutes must be 1..=720, got {minutes}"
        );
        self.step_minutes = minutes;
        self
    }

    /// The `chrono` format of a slot label, for example `"%-I:%M %p"`. The
    /// input value stays `HH:MM`.
    ///
    /// # Panics
    ///
    /// Panics when `format` is not a valid `chrono` format.
    #[must_use]
    pub fn label_format(mut self, format: &'static str) -> Self {
        assert!(
            !StrftimeItems::new(format).any(|item| matches!(item, Item::Error)),
            "TimePicker::label_format: {format:?} is not a valid chrono format"
        );
        self.label_format = format;
        self
    }

    /// Set the labels.
    #[must_use]
    pub fn locale(mut self, locale: CalendarLocale) -> Self {
        self.locale = locale;
        self
    }

    /// The slot times, first to last.
    #[must_use]
    pub fn slots(&self) -> Vec<NaiveTime> {
        let step = self.step_minutes * 60;
        let first = self.first.num_seconds_from_midnight();
        let last = self.last.num_seconds_from_midnight();
        (first..=last)
            .step_by(step as usize)
            .filter_map(|secs| NaiveTime::from_num_seconds_from_midnight_opt(secs, 0))
            .collect()
    }

    /// The toggle and the slot list. `value` is the current time.
    fn list(&self, id: &str, value: Option<NaiveTime>) -> Markup {
        let list = format!("{id}-times-dialog");
        let slots = self.slots();
        let stop = value
            .and_then(|v| slots.iter().position(|s| *s >= v))
            .unwrap_or(0);
        html! {
            (toggle(&list, self.locale.choose_time(), "listbox", "clock"))
            div id=(list) class="autumn-popover__panel autumn-datepicker-dialog autumn-datepicker-times"
                popover role="listbox" aria-label=(self.locale.times())
                data-side="bottom" data-align="start" data-autumn-popover
                data-autumn-roving data-autumn-orientation="vertical" data-autumn-typeahead {
                @for (i, slot) in slots.iter().enumerate() {
                    button type="button" role="option" class="autumn-datepicker-time"
                        data-time=(format_time(*slot))
                        aria-selected=(if Some(*slot) == value { "true" } else { "false" })
                        tabindex=(if i == stop { "0" } else { "-1" }) {
                        (slot.format(self.label_format))
                    }
                }
            }
        }
    }
}

impl FieldRenderer for TimePicker {
    fn name(&self) -> &'static str {
        "timepicker"
    }

    fn render(&self, field: &FieldContext<'_>) -> Markup {
        let current = field.value().and_then(parse_time);
        let value = field
            .value()
            .map(|raw| current.map_or_else(|| raw.to_owned(), format_time));
        let mut attrs = vec![
            ("type", "time".to_owned()),
            ("class", INPUT_CLASS.to_owned()),
        ];
        attrs.extend(value.map(|v| ("value", v)));
        if self.limited {
            attrs.push(("min", format_time(self.first)));
            attrs.push(("max", format_time(self.last)));
        }
        attrs.push(("step", "any".to_owned()));
        html! {
            div class="autumn-datepicker-field" data-autumn-datepicker="time" {
                (field.control("input", &borrowed(&attrs), html! {}))
                (self.list(field.id(), current))
            }
        }
    }
}

/// Render a [`TimePicker`] bound to `changeset[field]`.
#[must_use]
pub fn time_picker<T: Serialize>(
    changeset: &Changeset<T>,
    field: &str,
    label: &str,
    picker: &TimePicker,
    options: &FieldOptions<'_>,
) -> Markup {
    custom_input_with(changeset, field, label, picker, options)
}

/// A date-time picker: `<input type="datetime-local">`, a calendar and a
/// time list.
#[derive(Debug, Clone, Default)]
pub struct DateTimePicker {
    pub(crate) date: DatePicker,
    pub(crate) time: TimePicker,
    pub(crate) zone: Option<Tz>,
}

impl DateTimePicker {
    /// A [`DatePicker`] and a [`TimePicker`] with their defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the calendar part.
    #[must_use]
    pub fn calendar(mut self, date: DatePicker) -> Self {
        self.date = date;
        self
    }

    /// Set the time part.
    #[must_use]
    pub fn time(mut self, time: TimePicker) -> Self {
        self.time = time;
        self
    }

    /// Set the text of the calendar and the time list.
    #[must_use]
    pub fn locale(mut self, locale: CalendarLocale) -> Self {
        self.time.locale = locale.clone();
        self.date.locale = locale;
        self
    }

    /// Show an RFC 3339 value as the wall clock in `zone`. Today also uses
    /// `zone`. Read the post back with
    /// [`parse_in_zone`](crate::value::parse_in_zone).
    #[must_use]
    pub const fn time_zone(mut self, zone: Tz) -> Self {
        self.zone = Some(zone);
        self.date.zone = Some(zone);
        self
    }
}

impl FieldRenderer for DateTimePicker {
    fn name(&self) -> &'static str {
        "datetimepicker"
    }

    fn render(&self, field: &FieldContext<'_>) -> Markup {
        let current = field.value().and_then(|raw| parse_datetime(raw, self.zone));
        let value = field
            .value()
            .map(|raw| current.map_or_else(|| raw.to_owned(), format_datetime));
        let mut attrs = vec![
            ("type", "datetime-local".to_owned()),
            ("class", INPUT_CLASS.to_owned()),
        ];
        attrs.extend(value.map(|v| ("value", v)));
        attrs.extend(self.date.limits("T00:00", "T23:59:59.999"));
        attrs.push(("step", "any".to_owned()));
        let day = current.map(|dt| dt.date());
        let selection = day.map_or(Selection::Empty, Selection::Day);
        html! {
            div class="autumn-datepicker-field" data-autumn-datepicker="datetime"
                data-autumn-datepicker-inline[self.date.inline] {
                (field.control("input", &borrowed(&attrs), html! {}))
                (self.date.calendar_markup(field.id(), selection, day, Mode::DateTime))
                (self.time.list(field.id(), current.map(|dt| dt.time())))
            }
        }
    }
}

/// Render a [`DateTimePicker`] bound to `changeset[field]`.
#[must_use]
pub fn datetime_picker<T: Serialize>(
    changeset: &Changeset<T>,
    field: &str,
    label: &str,
    picker: &DateTimePicker,
    options: &FieldOptions<'_>,
) -> Markup {
    custom_input_with(changeset, field, label, picker, options)
}

/// A date-range picker: two `<input type="date">` and one calendar.
///
/// Bind it to a [`DateRange`](crate::DateRange) field.
#[derive(Debug, Clone, Default)]
pub struct DateRangePicker {
    pub(crate) date: DatePicker,
}

impl DateRangePicker {
    /// A picker with no limits and English text.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the calendar: rules, locale, today and inline mode.
    #[must_use]
    pub fn calendar(mut self, date: DatePicker) -> Self {
        self.date = date;
        self
    }
}

impl FieldRenderer for DateRangePicker {
    fn name(&self) -> &'static str {
        "daterangepicker"
    }

    fn grouped(&self) -> bool {
        true
    }

    fn render(&self, field: &FieldContext<'_>) -> Markup {
        let raw = field.values();
        let part = |i: usize| raw.get(i).map(String::as_str).filter(|v| !v.is_empty());
        let (start, end) = (part(0).and_then(parse_date), part(1).and_then(parse_date));
        let selection = Selection::range(start, end);
        let locale = &self.date.locale;
        let input = |suffix: &'static str, label: &str, value: Option<&str>| {
            let mut attrs = vec![
                ("type", "date".to_owned()),
                ("class", INPUT_CLASS.to_owned()),
            ];
            attrs.extend(value.map(|v| {
                (
                    "value",
                    parse_date(v).map_or_else(|| v.to_owned(), |d| d.to_string()),
                )
            }));
            attrs.extend(self.date.limits("", ""));
            html! {
                label class="autumn-datepicker-range-part" for=(format!("{}-{suffix}", field.id())) {
                    span class="autumn-datepicker-range-label" { (label) }
                    (field.group_item("input", suffix, &borrowed(&attrs), html! {}))
                }
            }
        };
        html! {
            div class="autumn-datepicker-field autumn-datepicker-range" data-autumn-datepicker="range"
                data-autumn-datepicker-inline[self.date.inline] {
                div class="autumn-datepicker-range-inputs" {
                    (input("start", locale.range_start(), part(0)))
                    (input("end", locale.range_end(), part(1)))
                }
                (self.date.calendar_markup(field.id(), selection, start.or(end), Mode::Range))
            }
        }
    }
}

/// Render a [`DateRangePicker`] bound to `changeset[field]`.
#[must_use]
pub fn date_range_picker<T: Serialize>(
    changeset: &Changeset<T>,
    field: &str,
    label: &str,
    picker: &DateRangePicker,
    options: &FieldOptions<'_>,
) -> Markup {
    custom_input_with(changeset, field, label, picker, options)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn today_uses_the_zone_across_midnight_and_dst() {
        // 04:30 UTC on 8 March 2026 is 23:30 EST on 7 March.
        let now = Utc
            .with_ymd_and_hms(2026, 3, 8, 4, 30, 0)
            .single()
            .expect("t");
        let ny = chrono_tz::America::New_York;
        assert_eq!(
            today_from(now, None),
            NaiveDate::from_ymd_opt(2026, 3, 8).expect("d")
        );
        assert_eq!(
            today_from(now, Some(ny)),
            NaiveDate::from_ymd_opt(2026, 3, 7).expect("d")
        );
        // 04:30 UTC on 9 March is 00:30 EDT: DST started on 8 March.
        let after = Utc
            .with_ymd_and_hms(2026, 3, 9, 4, 30, 0)
            .single()
            .expect("t");
        assert_eq!(
            today_from(after, Some(ny)),
            NaiveDate::from_ymd_opt(2026, 3, 9).expect("d")
        );
    }

    #[test]
    #[should_panic(expected = "past midnight")]
    fn an_overnight_time_range_panics() {
        let t = |h| NaiveTime::from_hms_opt(h, 0, 0).expect("t");
        let _ = TimePicker::new().between(t(22), t(2));
    }

    #[test]
    #[should_panic(expected = "not a valid chrono format")]
    fn a_bad_label_format_panics() {
        let _ = TimePicker::new().label_format("%Q");
    }
}
