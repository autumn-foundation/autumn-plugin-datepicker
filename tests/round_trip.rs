//! A value that a picker writes reads back through the `autumn_web::form`
//! time-zone deserializers, and the round trip is stable.
//!
//! "Stable" means: render, post, read, render again gives the same value.
//! The read value equals the original, except for a wall clock that a DST
//! fall-back change repeats: `datetime-local` has no offset, so the later
//! instant reads back as the earlier one.
//!
//! CI also runs this file with `TZ=America/New_York`, so the `Local` cases
//! cross real DST changes.

use autumn_datepicker_plugin::value::parse_in_zone;
use autumn_datepicker_plugin::{
    DatePicker, DateTimePicker, TimePicker, date_picker, datetime_picker, time_picker,
};
use autumn_web::form::{
    Changeset, FieldOptions, deserialize_datetime_local_local,
    deserialize_datetime_local_local_option, deserialize_datetime_local_utc,
    deserialize_datetime_local_utc_option, deserialize_naive_datetime_local,
    deserialize_naive_datetime_local_option,
};
use autumn_web::time_zone::parse_local_datetime;
use chrono::{
    DateTime, Local, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike, Utc,
};
use chrono_tz::Tz;
use proptest::prelude::*;
use serde::{Deserialize, Serialize};

/// The `value` of the input named `name`.
fn value_of(html: &str, name: &str) -> String {
    let marker = format!(r#"name="{name}" value=""#);
    let start = html
        .find(&marker)
        .map_or_else(|| panic!("no value: {html}"), |i| i + marker.len());
    let end = html[start..].find('"').expect("closing quote") + start;
    html[start..end].to_owned()
}

/// Post `value` as the form field `at` and read it with `T`.
fn post<T: for<'de> Deserialize<'de>>(value: &str) -> T {
    let body = serde_html_form::to_string([("at", value)]).expect("encode");
    serde_html_form::from_str(&body).unwrap_or_else(|e| panic!("{value:?} does not read back: {e}"))
}

fn render_datetime<T: Serialize>(at: T, picker: &DateTimePicker) -> String {
    #[derive(Serialize)]
    struct Form<T> {
        at: T,
    }
    let html = datetime_picker(
        &Changeset::new(Form { at }),
        "at",
        "At",
        picker,
        &FieldOptions::new(),
    )
    .into_string();
    value_of(&html, "at")
}

fn picker() -> DateTimePicker {
    DateTimePicker::new().calendar(DatePicker::new().today(NaiveDate::MIN))
}

#[derive(Deserialize)]
struct Utcs {
    #[serde(deserialize_with = "deserialize_datetime_local_utc")]
    at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct UtcOptions {
    #[serde(default, deserialize_with = "deserialize_datetime_local_utc_option")]
    at: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
struct Locals {
    #[serde(deserialize_with = "deserialize_datetime_local_local")]
    at: DateTime<Local>,
}

#[derive(Deserialize)]
struct LocalOptions {
    #[serde(default, deserialize_with = "deserialize_datetime_local_local_option")]
    at: Option<DateTime<Local>>,
}

#[derive(Deserialize)]
struct Naives {
    #[serde(deserialize_with = "deserialize_naive_datetime_local")]
    at: NaiveDateTime,
}

#[derive(Deserialize)]
struct NaiveOptions {
    #[serde(default, deserialize_with = "deserialize_naive_datetime_local_option")]
    at: Option<NaiveDateTime>,
}

#[derive(Deserialize)]
struct Dates {
    at: NaiveDate,
}

#[derive(Deserialize)]
struct Times {
    at: NaiveTime,
}

/// Instants from 1900 to 2100, with millisecond fractions one time in four.
fn instant() -> impl Strategy<Value = DateTime<Utc>> {
    (
        -2_208_988_800_i64..4_102_444_800,
        prop_oneof![3 => Just(0_u32), 1 => 0_u32..1000],
    )
        .prop_map(|(secs, millis)| {
            Utc.timestamp_opt(secs, millis * 1_000_000)
                .single()
                .expect("in range")
        })
}

/// Zones with and without DST, on both sides of UTC.
fn zone() -> impl Strategy<Value = Tz> {
    prop::sample::select(vec![
        chrono_tz::UTC,
        chrono_tz::America::New_York,
        chrono_tz::Europe::London,
        chrono_tz::Europe::Berlin,
        chrono_tz::Australia::Lord_Howe,
        chrono_tz::Asia::Kolkata,
        chrono_tz::Pacific::Chatham,
        chrono_tz::America::Sao_Paulo,
    ])
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn utc_round_trips_exactly(at in instant()) {
        let value = render_datetime(at, &picker());
        prop_assert!(!value.ends_with('Z'), "{value}");
        prop_assert_eq!(post::<Utcs>(&value).at, at);
        prop_assert_eq!(post::<UtcOptions>(&value).at, Some(at));
    }

    #[test]
    fn naive_round_trips_exactly(at in instant()) {
        let naive = at.naive_utc();
        let value = render_datetime(naive, &picker());
        prop_assert_eq!(post::<Naives>(&value).at, naive);
        prop_assert_eq!(post::<NaiveOptions>(&value).at, Some(naive));
    }

    #[test]
    fn local_round_trip_is_stable(at in instant()) {
        let local = at.with_timezone(&Local);
        let value = render_datetime(local, &picker());
        let read = post::<Locals>(&value).at;
        prop_assert_eq!(post::<LocalOptions>(&value).at, Some(read));
        // Stable: the read value renders to the same text.
        prop_assert_eq!(&render_datetime(read, &picker()), &value);
        match Local.from_local_datetime(&local.naive_local()) {
            LocalResult::Single(_) => prop_assert_eq!(read, local),
            LocalResult::Ambiguous(earliest, _) => prop_assert_eq!(read, earliest),
            LocalResult::None => prop_assert!(false, "an instant cannot be in a gap"),
        }
    }

    #[test]
    fn a_zoned_round_trip_is_stable(at in instant(), tz in zone()) {
        let zoned = picker().time_zone(tz);
        let value = render_datetime(at, &zoned);
        let read = parse_in_zone(&value, tz).expect("an instant has a wall clock");
        prop_assert_eq!(&render_datetime(read, &zoned), &value);
        let wall = at.with_timezone(&tz).naive_local();
        match tz.from_local_datetime(&wall) {
            LocalResult::Single(_) => prop_assert_eq!(read, at),
            LocalResult::Ambiguous(earliest, _) => {
                prop_assert_eq!(read, earliest.with_timezone(&Utc));
            }
            LocalResult::None => prop_assert!(false, "an instant cannot be in a gap"),
        }
        // `parse_local_datetime` reads minutes. It agrees on a whole minute.
        if at.second() == 0 && at.nanosecond() == 0 {
            prop_assert_eq!(parse_local_datetime(&value, tz).ok(), Some(read));
        }
    }

    #[test]
    fn a_date_round_trips_exactly(at in instant()) {
        #[derive(Serialize)]
        struct Form { at: NaiveDate }
        let day = at.date_naive();
        let html = date_picker(&Changeset::new(Form { at: day }), "at", "At",
            &DatePicker::new().today(day), &FieldOptions::new()).into_string();
        prop_assert_eq!(post::<Dates>(&value_of(&html, "at")).at, day);
    }

    #[test]
    fn a_time_round_trips_exactly(secs in 0_u32..86_400) {
        #[derive(Serialize)]
        struct Form { at: NaiveTime }
        let t = NaiveTime::from_num_seconds_from_midnight_opt(secs, 0).expect("t");
        let html = time_picker(&Changeset::new(Form { at: t }), "at", "At",
            &TimePicker::new(), &FieldOptions::new()).into_string();
        prop_assert_eq!(post::<Times>(&value_of(&html, "at")).at, t);
    }
}

/// An empty post reads as `None` in each `_option` deserializer.
#[test]
fn an_empty_value_reads_as_none() {
    assert_eq!(post::<UtcOptions>("").at, None);
    assert_eq!(post::<LocalOptions>("").at, None);
    assert_eq!(post::<NaiveOptions>("").at, None);
}

/// The fall-back hour, named: 01:30 EDT and 01:30 EST share one wall clock.
#[test]
fn the_repeated_hour_reads_back_as_the_earlier_instant() {
    let ny = chrono_tz::America::New_York;
    let edt = Utc
        .with_ymd_and_hms(2026, 11, 1, 5, 30, 0)
        .single()
        .expect("t");
    let est = Utc
        .with_ymd_and_hms(2026, 11, 1, 6, 30, 0)
        .single()
        .expect("t");
    let zoned = picker().time_zone(ny);
    assert_eq!(render_datetime(edt, &zoned), "2026-11-01T01:30");
    assert_eq!(render_datetime(est, &zoned), "2026-11-01T01:30");
    assert_eq!(parse_in_zone("2026-11-01T01:30", ny), Some(edt));
}

/// A wall clock in the spring-forward gap does not exist.
#[test]
fn a_wall_clock_in_the_gap_does_not_read() {
    let ny = chrono_tz::America::New_York;
    assert_eq!(parse_in_zone("2026-03-08T02:30", ny), None);
    assert_eq!(parse_in_zone("garbage", ny), None);
}
