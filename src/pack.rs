//! The widget pack: the assets and one story for each picker.

use autumn_web::assets::PluginAssets;
use autumn_web::form::{Changeset, FieldOptions};
use autumn_web::stories::story;
use autumn_web::widget_pack::WidgetPack;
use chrono::{NaiveDate, NaiveTime, Weekday};

use crate::locale::CalendarLocale;
use crate::rules::DateRules;
use crate::value::DateRange;
use crate::widgets::{
    DatePicker, DateRangePicker, DateTimePicker, TimePicker, date_picker, date_range_picker,
    datetime_picker, time_picker,
};

/// The pack name. Classes are `autumn-datepicker-*`. Story slugs are
/// `datepicker-*`.
pub const PACK: &str = "datepicker";

/// The pack CSS and JS.
pub static ASSETS: PluginAssets = PluginAssets::from_files(
    PACK,
    &[
        ("datepicker.css", include_bytes!("../assets/datepicker.css")),
        ("datepicker.js", include_bytes!("../assets/datepicker.js")),
    ],
);

/// A fixed today, so a story renders the same on each run.
fn story_today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 5).unwrap_or_default()
}

#[derive(serde::Serialize)]
struct Example {
    due: Option<NaiveDate>,
    at: Option<NaiveTime>,
    starts_at: Option<chrono::NaiveDateTime>,
    stay: Option<DateRange>,
}

fn example() -> Changeset<Example> {
    let day = story_today();
    Changeset::new(Example {
        due: Some(day),
        at: NaiveTime::from_hms_opt(9, 30, 0),
        starts_at: day.and_hms_opt(14, 0, 0),
        stay: day
            .succ_opt()
            .map(|end| DateRange::new(day, end + chrono::Days::new(2))),
    })
}

fn german() -> CalendarLocale {
    CalendarLocale::english()
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
        .weekday_names(
            [
                "Montag",
                "Dienstag",
                "Mittwoch",
                "Donnerstag",
                "Freitag",
                "Samstag",
                "Sonntag",
            ],
            ["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"],
        )
        .week_start(Weekday::Mon)
}

/// The pack: its stories and its assets.
#[must_use]
pub fn pack() -> WidgetPack {
    WidgetPack::new(PACK)
        .extend([
            story! {
                "Datepicker",
                "Datepicker: date picker",
                {
                    date_picker(
                        &example(),
                        "due",
                        "Due date",
                        &DatePicker::new().today(story_today()),
                        &FieldOptions::new().hint("Type a date, or choose one."),
                    )
                }
            },
            story! {
                "Datepicker",
                "Datepicker: inline calendar",
                {
                    date_picker(
                        &example(),
                        "due",
                        "Due date",
                        &DatePicker::new().inline().today(story_today()),
                        &FieldOptions::new(),
                    )
                }
            },
            story! {
                "Datepicker",
                "Datepicker: limits and disabled days",
                {
                    let day = story_today();
                    let rules = DateRules::new()
                        .min(day)
                        .max(day + chrono::Days::new(45))
                        .disable_weekends();
                    date_picker(
                        &example(),
                        "due",
                        "Delivery date",
                        &DatePicker::new().rules(rules).inline().today(day),
                        &FieldOptions::new().hint("Work days in the next 45 days."),
                    )
                }
            },
            story! {
                "Datepicker",
                "Datepicker: week start and names",
                {
                    date_picker(
                        &example(),
                        "due",
                        "Fälligkeitsdatum",
                        &DatePicker::new().locale(german()).inline().today(story_today()),
                        &FieldOptions::new(),
                    )
                }
            },
            story! {
                "Datepicker",
                "Datepicker: time picker",
                {
                    time_picker(
                        &example(),
                        "at",
                        "Start time",
                        &TimePicker::new().step_minutes(30),
                        &FieldOptions::new(),
                    )
                }
            },
            story! {
                "Datepicker",
                "Datepicker: date-time picker",
                {
                    datetime_picker(
                        &example(),
                        "starts_at",
                        "Starts at",
                        // Inline: Chromium gives `datetime-local` one Tab stop per
                        // part, and the inline grid adds stops for the Tab check.
                        &DateTimePicker::new().calendar(DatePicker::new().inline().today(story_today())),
                        &FieldOptions::new(),
                    )
                }
            },
            story! {
                "Datepicker",
                "Datepicker: date-range picker",
                {
                    date_range_picker(
                        &example(),
                        "stay",
                        "Stay",
                        &DateRangePicker::new().calendar(DatePicker::new().inline().today(story_today())),
                        &FieldOptions::new(),
                    )
                }
            },
        ])
        .assets(&ASSETS)
}
