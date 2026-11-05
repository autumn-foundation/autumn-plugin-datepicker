//! Input values: read a changeset value, and write the shape that the
//! native input and the `autumn_web::form` deserializers accept.

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
use serde::de::{self, SeqAccess, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Read a date from a changeset value.
///
/// Accepts `YYYY-MM-DD`, a `datetime-local` value and RFC 3339. For RFC 3339
/// the date is the one of the written offset.
#[must_use]
pub fn parse_date(raw: &str) -> Option<NaiveDate> {
    crate::rules::parse_iso_date(raw).or_else(|| parse_datetime(raw, None).map(|dt| dt.date()))
}

/// Read a wall-clock date and time from a changeset value.
///
/// Accepts `YYYY-MM-DDTHH:MM[:SS[.f]]`, the same with a space, and RFC 3339.
/// With `zone`, the result for an RFC 3339 instant is the wall clock in
/// `zone`. Without `zone`, the result is the wall clock of its own offset, as
/// `autumn_web::form::datetime_input` does.
#[must_use]
pub fn parse_datetime(raw: &str, zone: Option<Tz>) -> Option<NaiveDateTime> {
    let raw = raw.trim().replacen(' ', "T", 1);
    if let Ok(instant) = DateTime::parse_from_rfc3339(&raw) {
        return Some(zone.map_or_else(
            || instant.naive_local(),
            |tz| instant.with_timezone(&tz).naive_local(),
        ));
    }
    parse_wall_clock(&raw)
}

/// Parse `YYYY-MM-DDTHH:MM[:SS[.f]]`.
fn parse_wall_clock(raw: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S%.f")
        .or_else(|_| NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M"))
        .ok()
}

/// Read a time from a changeset value: `HH:MM[:SS[.f]]`.
#[must_use]
pub fn parse_time(raw: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(raw, "%H:%M:%S%.f")
        .or_else(|_| NaiveTime::parse_from_str(raw, "%H:%M"))
        .ok()
}

/// Write a `datetime-local` value: `YYYY-MM-DDTHH:MM`, then `:SS` and the
/// milliseconds only when they are not zero. See [`format_time`].
#[must_use]
pub fn format_datetime(value: NaiveDateTime) -> String {
    format!("{}T{}", value.date(), format_time(value.time()))
}

/// Write a `time` value: `HH:MM`, then `:SS` and the milliseconds only when
/// they are not zero.
///
/// HTML allows at most three fraction digits, so the value drops the time
/// below one millisecond. Without that, a browser can clear the input.
#[must_use]
pub fn format_time(value: NaiveTime) -> String {
    let millis = value.nanosecond() / 1_000_000;
    if millis != 0 {
        value.format("%H:%M:%S%.3f").to_string()
    } else if value.second() != 0 {
        value.format("%H:%M:%S").to_string()
    } else {
        value.format("%H:%M").to_string()
    }
}

/// Read a `datetime-local` value as a wall clock in `zone`.
///
/// Accepts `YYYY-MM-DDTHH:MM[:SS[.f]]`. A wall clock that a DST change
/// repeats gives the earlier instant. A wall clock in a DST gap gives
/// `None`. `autumn_web::time_zone::parse_local_datetime` gives the same
/// result, but it reads whole minutes only.
#[must_use]
pub fn parse_in_zone(raw: &str, zone: Tz) -> Option<DateTime<Utc>> {
    let wall = parse_wall_clock(raw.trim())?;
    zone.from_local_datetime(&wall)
        .earliest()
        .map(|dt| dt.with_timezone(&Utc))
}

/// A date range: one form field with a start and an end.
///
/// It serializes to `[start, end]`. A form posts it as two values of one
/// key (`stay=2026-11-05&stay=2026-11-12`). It deserializes from two
/// values, or from one ISO 8601 interval string (`2026-11-05/2026-11-12`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DateRange {
    start: NaiveDate,
    end: NaiveDate,
}

impl DateRange {
    /// The range from the earlier date to the later date.
    #[must_use]
    pub fn new(a: NaiveDate, b: NaiveDate) -> Self {
        Self {
            start: a.min(b),
            end: a.max(b),
        }
    }

    /// The first day.
    #[must_use]
    pub const fn start(&self) -> NaiveDate {
        self.start
    }

    /// The last day.
    #[must_use]
    pub const fn end(&self) -> NaiveDate {
        self.end
    }

    /// `true` when `date` is in the range. Both ends are in it.
    #[must_use]
    pub fn contains(&self, date: NaiveDate) -> bool {
        self.start <= date && date <= self.end
    }

    /// Use with `#[serde(default, deserialize_with = "…")]` on an
    /// `Option<DateRange>` field. Two empty values give `None`.
    ///
    /// # Errors
    ///
    /// The deserializer error when a value is not a date, or only one of the
    /// two values is empty.
    pub fn deserialize_option<'de, D>(deserializer: D) -> Result<Option<Self>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(RangeVisitor { optional: true })
    }

    /// Read two values. Both empty is `None` when `optional`.
    fn from_parts<E: de::Error>(start: &str, end: &str, optional: bool) -> Result<Option<Self>, E> {
        match (start.is_empty(), end.is_empty()) {
            (true, true) if optional => return Ok(None),
            (false, false) => {}
            _ => return Err(E::custom("a date range needs a start and an end")),
        }
        let part = |raw: &str| {
            crate::rules::parse_iso_date(raw)
                .ok_or_else(|| E::custom(format!("`{raw}` is not a YYYY-MM-DD date")))
        };
        Ok(Some(Self::new(part(start)?, part(end)?)))
    }
}

impl Serialize for DateRange {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(2))?;
        seq.serialize_element(&self.start.to_string())?;
        seq.serialize_element(&self.end.to_string())?;
        seq.end()
    }
}

impl<'de> Deserialize<'de> for DateRange {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer
            .deserialize_any(RangeVisitor { optional: false })?
            .ok_or_else(|| de::Error::custom("a date range needs a start and an end"))
    }
}

/// Reads two values, or one `start/end` string.
struct RangeVisitor {
    optional: bool,
}

impl<'de> Visitor<'de> for RangeVisitor {
    type Value = Option<DateRange>;

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("two YYYY-MM-DD dates, or a `start/end` string")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        match value.split_once('/') {
            Some((start, end)) => DateRange::from_parts(start, end, self.optional),
            // One value of a repeated key: the end is missing.
            None => DateRange::from_parts(value, "", self.optional),
        }
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        if self.optional {
            Ok(None)
        } else {
            Err(E::custom("a date range needs a start and an end"))
        }
    }

    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        self.visit_unit()
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(self)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let start: String = seq.next_element()?.unwrap_or_default();
        let end: String = seq.next_element()?.unwrap_or_default();
        if seq.next_element::<de::IgnoredAny>()?.is_some() {
            return Err(de::Error::custom("a date range has two values"));
        }
        DateRange::from_parts(&start, &end, self.optional)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("a valid test date")
    }

    #[test]
    fn parse_date_reads_each_changeset_shape() {
        let d = Some(date(2026, 11, 5));
        assert_eq!(parse_date("2026-11-05"), d);
        assert_eq!(parse_date("2026-11-05T10:30:00"), d);
        assert_eq!(parse_date("2026-11-05 10:30:00"), d);
        assert_eq!(
            parse_date("2026-11-05T23:30:00-05:00"),
            d,
            "the written offset"
        );
        assert_eq!(parse_date(""), None);
        assert_eq!(parse_date("tomorrow"), None);
    }

    #[test]
    fn parse_datetime_reads_each_changeset_shape() {
        let wall = date(2026, 3, 8).and_hms_opt(7, 30, 0);
        assert_eq!(parse_datetime("2026-03-08T07:30", None), wall);
        assert_eq!(parse_datetime("2026-03-08T07:30:00", None), wall);
        assert_eq!(parse_datetime("2026-03-08 07:30:00", None), wall);
        assert_eq!(parse_datetime("2026-03-08T07:30:00Z", None), wall);
        assert_eq!(parse_datetime("nope", None), None);
    }

    #[test]
    fn parse_datetime_in_a_zone_shows_the_zone_wall_clock() {
        // US DST starts at 07:00 UTC on 8 March 2026. 07:30 UTC is 03:30 EDT.
        let ny = chrono_tz::America::New_York;
        assert_eq!(
            parse_datetime("2026-03-08T07:30:00Z", Some(ny)),
            date(2026, 3, 8).and_hms_opt(3, 30, 0)
        );
        // A wall clock has no offset: it stays as it is.
        assert_eq!(
            parse_datetime("2026-03-08T07:30", Some(ny)),
            date(2026, 3, 8).and_hms_opt(7, 30, 0)
        );
    }

    #[test]
    fn format_datetime_drops_zero_seconds() {
        let d = date(2026, 11, 5);
        assert_eq!(
            format_datetime(d.and_hms_opt(9, 5, 0).expect("t")),
            "2026-11-05T09:05"
        );
        assert_eq!(
            format_datetime(d.and_hms_opt(9, 5, 7).expect("t")),
            "2026-11-05T09:05:07"
        );
        assert_eq!(
            format_datetime(d.and_hms_milli_opt(9, 5, 7, 250).expect("t")),
            "2026-11-05T09:05:07.250"
        );
    }

    #[test]
    fn the_fraction_stops_at_milliseconds() {
        let d = date(2026, 11, 5);
        let t = |nanos| d.and_hms_nano_opt(14, 0, 7, nanos).expect("t");
        assert_eq!(format_datetime(t(123_456_789)), "2026-11-05T14:00:07.123");
        assert_eq!(
            format_datetime(t(999_999)),
            "2026-11-05T14:00:07",
            "below 1 ms"
        );
        assert_eq!(format_time(t(5_000_000).time()), "14:00:07.005");
    }

    #[test]
    fn time_values() {
        assert_eq!(parse_time("09:30"), NaiveTime::from_hms_opt(9, 30, 0));
        assert_eq!(parse_time("09:30:15"), NaiveTime::from_hms_opt(9, 30, 15));
        assert_eq!(parse_time("9.30"), None);
        assert_eq!(
            format_time(NaiveTime::from_hms_opt(9, 30, 0).expect("t")),
            "09:30"
        );
        assert_eq!(
            format_time(NaiveTime::from_hms_opt(9, 30, 15).expect("t")),
            "09:30:15"
        );
    }

    #[test]
    fn a_range_orders_its_ends() {
        let r = DateRange::new(date(2026, 11, 12), date(2026, 11, 5));
        assert_eq!(r.start(), date(2026, 11, 5));
        assert_eq!(r.end(), date(2026, 11, 12));
        assert!(r.contains(date(2026, 11, 5)));
        assert!(r.contains(date(2026, 11, 12)));
        assert!(!r.contains(date(2026, 11, 13)));
    }

    #[test]
    fn a_range_serializes_to_two_values() {
        let r = DateRange::new(date(2026, 11, 5), date(2026, 11, 12));
        let json = serde_json::to_string(&r).expect("json");
        assert_eq!(json, r#"["2026-11-05","2026-11-12"]"#);
        assert_eq!(serde_json::from_str::<DateRange>(&json).expect("back"), r);
        let interval: DateRange =
            serde_json::from_str(r#""2026-11-05/2026-11-12""#).expect("interval");
        assert_eq!(interval, r);
    }

    #[test]
    fn a_range_reads_a_repeated_form_key() {
        #[derive(Deserialize)]
        struct Booking {
            stay: DateRange,
        }
        let b: Booking =
            serde_html_form::from_str("stay=2026-11-12&stay=2026-11-05").expect("form");
        assert_eq!(
            b.stay,
            DateRange::new(date(2026, 11, 5), date(2026, 11, 12))
        );
        assert!(serde_html_form::from_str::<Booking>("stay=2026-11-12").is_err());
        assert!(serde_html_form::from_str::<Booking>("stay=2026-11-12&stay=").is_err());
        assert!(serde_html_form::from_str::<Booking>("stay=a&stay=b").is_err());
    }

    #[test]
    fn an_optional_range_is_none_when_both_values_are_empty() {
        #[derive(Deserialize)]
        struct Booking {
            #[serde(default, deserialize_with = "DateRange::deserialize_option")]
            stay: Option<DateRange>,
        }
        let none: Booking = serde_html_form::from_str("stay=&stay=").expect("form");
        assert_eq!(none.stay, None);
        let absent: Booking = serde_html_form::from_str("").expect("form");
        assert_eq!(absent.stay, None);
        let some: Booking =
            serde_html_form::from_str("stay=2026-11-05&stay=2026-11-06").expect("form");
        assert!(some.stay.is_some());
        assert!(serde_html_form::from_str::<Booking>("stay=2026-11-05&stay=").is_err());
    }
}
