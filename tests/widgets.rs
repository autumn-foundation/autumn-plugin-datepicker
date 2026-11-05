//! The pickers bind to `Changeset<T>` and to `form_for`.

use autumn_datepicker_plugin::{
    CalendarLocale, DatePicker, DateRange, DateRangePicker, DateRules, DateTimePicker, TimePicker,
    date_picker, date_range_picker, datetime_picker, time_picker,
};
use autumn_web::form::{Changeset, FieldControl, FieldOptions, FormField, FormModel, form_for};
use chrono::{NaiveDate, NaiveTime, TimeZone, Utc, Weekday};

const fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).expect("a valid test date")
}

#[derive(serde::Serialize)]
struct Task {
    due: Option<NaiveDate>,
}

#[test]
fn the_date_picker_is_a_native_input_with_a_hidden_toggle() {
    let changeset = Changeset::new(Task {
        due: Some(date(2026, 11, 5)),
    });
    let picker = DatePicker::new().today(date(2026, 11, 1));
    let html = date_picker(
        &changeset,
        "due",
        "Due",
        &picker,
        &FieldOptions::new().required(),
    )
    .into_string();
    assert!(
        html.contains(r#"<div id="due-field" class="autumn-field""#),
        "{html}"
    );
    assert!(
        html.contains(r#"<label id="due-label" for="due""#),
        "{html}"
    );
    assert!(
        html.contains(r#"<input type="date" id="due" name="due" value="2026-11-05" required"#),
        "{html}"
    );
    assert!(html.contains("autumn-datepicker-input"), "{html}");
    // The toggle stays hidden until the pack script runs.
    assert!(html.contains(r#"aria-haspopup="dialog""#), "{html}");
    assert!(html.contains(r#"commandfor="due-dialog""#), "{html}");
    assert!(
        html.contains(r#"aria-label="Choose date" hidden"#),
        "{html}"
    );
    assert!(html.contains(r#"id="due-dialog""#), "{html}");
    assert!(html.contains(" popover "), "{html}");
    assert!(html.contains(r#"role="dialog""#), "{html}");
    assert!(
        html.contains(r#"data-date="2026-11-05" tabindex="0" aria-selected="true""#),
        "{html}"
    );
    assert!(!html.contains("style="), "{html}");
    assert!(!html.contains("<script"), "{html}");
    // One live region per picker, outside the calendar that htmx swaps.
    let status = html.find(r#"class="autumn-datepicker-status" role="status" aria-live="polite""#);
    let calendar = html.find(r#"class="autumn-datepicker-calendar""#);
    assert!(status.is_some() && status < calendar, "{html}");
}

#[test]
fn an_empty_value_shows_the_month_of_today() {
    let changeset = Changeset::new(Task { due: None });
    let picker = DatePicker::new().today(date(2027, 2, 14));
    let html = date_picker(&changeset, "due", "Due", &picker, &FieldOptions::new()).into_string();
    assert!(html.contains(">February 2027<"), "{html}");
    assert!(!html.contains("value="), "no value attribute: {html}");
}

#[test]
fn min_and_max_go_on_the_input_and_the_grid() {
    let changeset = Changeset::new(Task { due: None });
    let picker = DatePicker::new()
        .min(date(2026, 11, 10))
        .max(date(2026, 12, 20))
        .today(date(2026, 1, 1));
    let html = date_picker(&changeset, "due", "Due", &picker, &FieldOptions::new()).into_string();
    assert!(html.contains(r#"min="2026-11-10""#), "{html}");
    assert!(html.contains(r#"max="2026-12-20""#), "{html}");
    assert!(
        html.contains(">November 2026<"),
        "today is before min: {html}"
    );
    assert!(
        html.contains(r#"data-date="2026-11-09" tabindex="-1" aria-disabled="true""#),
        "{html}"
    );
}

#[test]
fn the_locale_sets_the_week_start_and_the_names() {
    let changeset = Changeset::new(Task { due: None });
    let locale = CalendarLocale::english()
        .month_names([
            "Januar",
            "Februar",
            "März",
            "April",
            "Mai",
            "Juni",
            "Juli",
            "August",
            "September",
            "Oktober",
            "November",
            "Dezember",
        ])
        .week_start(Weekday::Mon);
    let picker = DatePicker::new().locale(locale).today(date(2026, 3, 1));
    let html = date_picker(&changeset, "due", "Due", &picker, &FieldOptions::new()).into_string();
    assert!(html.contains(">März 2026<"), "{html}");
    assert!(html.contains("dp-ws=1"), "{html}");
    let mo = html.find(r#"abbr="Monday""#).expect("Monday");
    let su = html.find(r#"abbr="Sunday""#).expect("Sunday");
    assert!(mo < su, "{html}");
}

#[test]
fn the_inline_calendar_has_no_popover() {
    let changeset = Changeset::new(Task { due: None });
    let picker = DatePicker::new().inline().today(date(2026, 11, 1));
    let html = date_picker(&changeset, "due", "Due", &picker, &FieldOptions::new()).into_string();
    assert!(!html.contains("popover"), "{html}");
    assert!(html.contains("data-autumn-datepicker-inline"), "{html}");
    assert!(html.contains(r#"role="grid""#), "{html}");
    assert!(html.contains(r#"role="status""#), "{html}");
}

#[test]
fn the_passthrough_reaches_the_input() {
    let changeset = Changeset::new(Task { due: None });
    let html = date_picker(
        &changeset,
        "due",
        "Due",
        &DatePicker::new().today(date(2026, 11, 1)),
        &FieldOptions::new()
            .hint("Pick a work day.")
            .data("test-id", "due")
            .hx("trigger", "change"),
    )
    .into_string();
    assert!(html.contains(r#"aria-describedby="due-hint""#), "{html}");
    assert!(html.contains(r#"data-test-id="due""#), "{html}");
    assert!(html.contains(r#"hx-trigger="change""#), "{html}");
}

#[test]
fn errors_mark_the_input_invalid() {
    let mut changeset = Changeset::new(Task { due: None });
    changeset.add_error("due", "Pick a date.");
    let html = date_picker(
        &changeset,
        "due",
        "Due",
        &DatePicker::new().today(date(2026, 11, 1)),
        &FieldOptions::new(),
    )
    .into_string();
    assert!(html.contains(r#"aria-invalid="true""#), "{html}");
    assert!(html.contains("Pick a date."), "{html}");
}

impl FormModel for Task {
    fn form_fields() -> Vec<FormField> {
        vec![FormField::new("due", "Due", FieldControl::Date, true)]
    }
}

#[test]
fn the_date_picker_binds_to_form_for() {
    let changeset = Changeset::new(Task {
        due: Some(date(2026, 11, 5)),
    });
    let html = form_for(&changeset, "/tasks", "post")
        .override_field(
            "due",
            FieldControl::custom(DatePicker::new().today(date(2026, 11, 1))),
        )
        .render()
        .into_string();
    assert!(
        html.contains(r#"<input type="date" id="due" name="due" value="2026-11-05" required"#),
        "{html}"
    );
    assert!(html.contains("autumn-datepicker-calendar"), "{html}");
}

#[derive(serde::Serialize)]
struct Meeting {
    at: NaiveTime,
}

#[test]
fn the_time_picker_lists_the_slots() {
    let changeset = Changeset::new(Meeting {
        at: NaiveTime::from_hms_opt(9, 30, 0).expect("t"),
    });
    let picker = TimePicker::new()
        .between(
            NaiveTime::from_hms_opt(9, 0, 0).expect("t"),
            NaiveTime::from_hms_opt(11, 0, 0).expect("t"),
        )
        .step_minutes(30);
    assert_eq!(picker.slots().len(), 5);
    let html = time_picker(&changeset, "at", "At", &picker, &FieldOptions::new()).into_string();
    assert!(
        html.contains(r#"<input type="time" id="at" name="at" value="09:30""#),
        "{html}"
    );
    assert!(html.contains(r#"min="09:00""#), "{html}");
    assert!(html.contains(r#"max="11:00""#), "{html}");
    assert!(html.contains(r#"role="listbox""#), "{html}");
    assert!(html.contains("data-autumn-roving"), "{html}");
    assert!(html.contains("data-autumn-typeahead"), "{html}");
    assert!(
        html.contains(r#"aria-label="Choose time" hidden"#),
        "{html}"
    );
    assert_eq!(html.matches(r#"role="option""#).count(), 5, "{html}");
    assert!(
        html.contains(r#"data-time="09:30" aria-selected="true" tabindex="0""#),
        "{html}"
    );
}

#[test]
fn a_time_label_can_use_a_12_hour_clock() {
    let changeset = Changeset::new(Meeting {
        at: NaiveTime::from_hms_opt(13, 0, 0).expect("t"),
    });
    let picker = TimePicker::new().step_minutes(60).label_format("%-I:%M %p");
    let html = time_picker(&changeset, "at", "At", &picker, &FieldOptions::new()).into_string();
    assert!(html.contains(r#"data-time="13:00""#), "{html}");
    assert!(html.contains(">1:00 PM<"), "{html}");
}

#[test]
#[should_panic(expected = "step_minutes")]
fn a_zero_step_panics() {
    let _ = TimePicker::new().step_minutes(0);
}

#[derive(serde::Serialize)]
struct Event {
    starts_at: chrono::DateTime<Utc>,
}

#[test]
fn the_datetime_picker_writes_the_datetime_local_shape() {
    // 07:30 UTC on 8 March 2026 is 03:30 EDT (DST starts at 07:00 UTC).
    let at = Utc
        .with_ymd_and_hms(2026, 3, 8, 7, 30, 0)
        .single()
        .expect("t");
    let changeset = Changeset::new(Event { starts_at: at });
    let utc = DateTimePicker::new().calendar(DatePicker::new().today(date(2026, 3, 1)));
    let html = datetime_picker(
        &changeset,
        "starts_at",
        "Starts",
        &utc,
        &FieldOptions::new(),
    )
    .into_string();
    assert!(
        html.contains(r#"<input type="datetime-local" id="starts_at" name="starts_at" value="2026-03-08T07:30""#),
        "no Z: {html}"
    );
    assert!(html.contains(r#"step="any""#), "{html}");
    assert!(html.contains(r#"data-mode="datetime""#), "{html}");
    assert!(html.contains(r#"role="listbox""#), "{html}");
    assert!(html.contains(r#"aria-label="Choose date""#), "{html}");
    assert!(html.contains(r#"aria-label="Choose time""#), "{html}");

    let ny = utc.time_zone(chrono_tz::America::New_York);
    let html =
        datetime_picker(&changeset, "starts_at", "Starts", &ny, &FieldOptions::new()).into_string();
    assert!(html.contains(r#"value="2026-03-08T03:30""#), "{html}");
}

#[derive(serde::Serialize)]
struct Booking {
    stay: Option<DateRange>,
}

#[test]
fn the_range_picker_is_two_native_inputs_in_a_fieldset() {
    let changeset = Changeset::new(Booking {
        stay: Some(DateRange::new(date(2026, 11, 5), date(2026, 11, 7))),
    });
    let picker = DateRangePicker::new().calendar(DatePicker::new().today(date(2026, 11, 1)));
    let html =
        date_range_picker(&changeset, "stay", "Stay", &picker, &FieldOptions::new()).into_string();
    assert!(html.contains(r#"<fieldset id="stay-field""#), "{html}");
    assert!(html.contains("<legend"), "{html}");
    assert!(
        html.contains(r#"<input type="date" id="stay-start" name="stay""#),
        "{html}"
    );
    assert!(
        html.contains(r#"<input type="date" id="stay-end" name="stay""#),
        "{html}"
    );
    assert!(html.contains(r#"value="2026-11-05""#), "{html}");
    assert!(html.contains(r#"value="2026-11-07""#), "{html}");
    assert!(html.contains(">Start date<"), "{html}");
    assert!(html.contains(">End date<"), "{html}");
    assert_eq!(html.matches(r#"aria-selected="true""#).count(), 3, "{html}");
    assert!(html.contains(r#"data-mode="range""#), "{html}");
}

#[test]
fn an_empty_range_has_two_empty_inputs() {
    let changeset = Changeset::new(Booking { stay: None });
    let picker = DateRangePicker::new().calendar(
        DatePicker::new()
            .rules(DateRules::new().disable_weekends())
            .today(date(2026, 11, 1)),
    );
    let html =
        date_range_picker(&changeset, "stay", "Stay", &picker, &FieldOptions::new()).into_string();
    assert_eq!(html.matches(r#"name="stay""#).count(), 2, "{html}");
    assert!(!html.contains(r#"aria-selected="true""#), "{html}");
    assert!(html.contains("dp-off=67"), "{html}");
}

#[derive(serde::Serialize)]
struct Clash {
    #[serde(rename = "dp-month")]
    month: Option<NaiveDate>,
}

#[test]
fn a_field_name_that_the_route_cannot_read_keeps_the_native_input_only() {
    let changeset = Changeset::new(Clash { month: None });
    let html = date_picker(
        &changeset,
        "dp-month",
        "Month",
        &DatePicker::new().today(date(2026, 11, 1)),
        &FieldOptions::new(),
    )
    .into_string();
    assert!(
        html.contains(r#"<input type="date" id="dp-month""#),
        "{html}"
    );
    assert!(!html.contains("autumn-datepicker-calendar"), "{html}");
    assert!(!html.contains("autumn-datepicker-toggle"), "{html}");
}

#[test]
fn a_default_time_picker_writes_no_min_or_max() {
    let changeset = Changeset::new(Meeting {
        at: NaiveTime::from_hms_opt(23, 59, 30).expect("t"),
    });
    let html = time_picker(
        &changeset,
        "at",
        "At",
        &TimePicker::new(),
        &FieldOptions::new(),
    )
    .into_string();
    assert!(html.contains(r#"value="23:59:30""#), "{html}");
    assert!(!html.contains("min=") && !html.contains("max="), "{html}");
}

#[test]
fn the_datetime_locale_sets_both_parts_and_max_covers_the_last_day() {
    let at = Utc
        .with_ymd_and_hms(2026, 3, 8, 7, 30, 0)
        .single()
        .expect("t");
    let changeset = Changeset::new(Event { starts_at: at });
    let locale = CalendarLocale::english()
        .label(autumn_datepicker_plugin::Label::ChooseDate, "Datum")
        .label(autumn_datepicker_plugin::Label::ChooseTime, "Zeit");
    let picker = DateTimePicker::new()
        .calendar(
            DatePicker::new()
                .max(date(2026, 3, 31))
                .today(date(2026, 3, 1)),
        )
        .locale(locale);
    let html = datetime_picker(
        &changeset,
        "starts_at",
        "Starts",
        &picker,
        &FieldOptions::new(),
    )
    .into_string();
    assert!(html.contains(r#"aria-label="Datum""#), "{html}");
    assert!(html.contains(r#"aria-label="Zeit""#), "{html}");
    assert!(html.contains(r#"max="2026-03-31T23:59:59.999""#), "{html}");
}

#[test]
fn today_follows_the_time_zone() {
    // With a zone and no fixed today, the grid shows the month of today in
    // that zone. Far east and far west zones are one day apart.
    let changeset = Changeset::new(Task { due: None });
    for zone in [
        chrono_tz::Pacific::Kiritimati,
        chrono_tz::Pacific::Pago_Pago,
    ] {
        let html = date_picker(
            &changeset,
            "due",
            "Due",
            &DatePicker::new().time_zone(zone),
            &FieldOptions::new(),
        )
        .into_string();
        let today = autumn_web::time::ambient_now()
            .with_timezone(&zone)
            .date_naive();
        assert!(
            html.contains(&format!(r#"data-today="{today}""#)),
            "{zone}: {html}"
        );
    }
}
