//! Which days a user can pick: `min`, `max`, disabled weekdays and disabled
//! dates.
//!
//! The rules go into the month route query string, so the route needs no
//! server state.

use std::collections::BTreeSet;

use chrono::{Datelike, NaiveDate, Weekday};

use crate::grid::MonthGrid;

/// The most disabled dates that one query string can hold.
pub const MAX_DISABLED_DATES: usize = 400;

/// The error for a rule value that does not parse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleError {
    /// The query key.
    pub key: &'static str,
    /// What is wrong.
    pub reason: &'static str,
}

impl std::fmt::Display for RuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "`{}`: {}", self.key, self.reason)
    }
}

impl std::error::Error for RuleError {}

/// The days a user can pick.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DateRules {
    min: Option<NaiveDate>,
    max: Option<NaiveDate>,
    /// Bit `n` is the weekday `n` days from Monday.
    weekdays: u8,
    dates: BTreeSet<NaiveDate>,
}

impl DateRules {
    /// No limits: each day is enabled.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            min: None,
            max: None,
            weekdays: 0,
            dates: BTreeSet::new(),
        }
    }

    /// The first day a user can pick.
    ///
    /// # Panics
    ///
    /// Panics when `date` is after [`max`](Self::max).
    #[must_use]
    pub fn min(mut self, date: NaiveDate) -> Self {
        assert!(
            self.max.is_none_or(|max| date <= max),
            "DateRules::min {date} is after max"
        );
        self.min = Some(date);
        self
    }

    /// The last day a user can pick.
    ///
    /// # Panics
    ///
    /// Panics when `date` is before [`min`](Self::min).
    #[must_use]
    pub fn max(mut self, date: NaiveDate) -> Self {
        assert!(
            self.min.is_none_or(|min| date >= min),
            "DateRules::max {date} is before min"
        );
        self.max = Some(date);
        self
    }

    /// Disable each `weekday`.
    #[must_use]
    pub const fn disable_weekday(mut self, weekday: Weekday) -> Self {
        self.weekdays |= 1 << weekday.num_days_from_monday();
        self
    }

    /// Disable Saturday and Sunday.
    #[must_use]
    pub const fn disable_weekends(self) -> Self {
        self.disable_weekday(Weekday::Sat)
            .disable_weekday(Weekday::Sun)
    }

    /// Disable one date.
    ///
    /// # Panics
    ///
    /// Panics when the rules hold more than [`MAX_DISABLED_DATES`] dates. A
    /// larger list does not fit in a URL.
    #[must_use]
    pub fn disable_date(mut self, date: NaiveDate) -> Self {
        self.dates.insert(date);
        assert!(
            self.dates.len() <= MAX_DISABLED_DATES,
            "more than MAX_DISABLED_DATES ({MAX_DISABLED_DATES}) disabled dates do not fit in \
             the month route URL; validate those dates on the server"
        );
        self
    }

    /// Disable each date in `dates`.
    ///
    /// # Panics
    ///
    /// Panics as [`disable_date`](Self::disable_date) does.
    #[must_use]
    pub fn disable_dates(self, dates: impl IntoIterator<Item = NaiveDate>) -> Self {
        dates.into_iter().fold(self, Self::disable_date)
    }

    /// Disable each date in `dates`, for a list from a database.
    ///
    /// # Errors
    ///
    /// [`RuleError`] when the rules then hold more than
    /// [`MAX_DISABLED_DATES`] dates. The rules do not change.
    pub fn try_disable_dates(
        mut self,
        dates: impl IntoIterator<Item = NaiveDate>,
    ) -> Result<Self, RuleError> {
        let before = self.dates.clone();
        for date in dates {
            self.dates.insert(date);
            if self.dates.len() > MAX_DISABLED_DATES {
                self.dates = before;
                return Err(RuleError {
                    key: "x",
                    reason: "too many dates",
                });
            }
        }
        Ok(self)
    }

    /// The first day a user can pick, if set.
    #[must_use]
    pub const fn min_date(&self) -> Option<NaiveDate> {
        self.min
    }

    /// The last day a user can pick, if set.
    #[must_use]
    pub const fn max_date(&self) -> Option<NaiveDate> {
        self.max
    }

    /// `true` when a user can pick `date`.
    #[must_use]
    pub fn is_enabled(&self, date: NaiveDate) -> bool {
        self.min.is_none_or(|min| date >= min)
            && self.max.is_none_or(|max| date <= max)
            && self.weekdays & (1 << date.weekday().num_days_from_monday()) == 0
            && !self.dates.contains(&date)
    }

    /// `true` when part of the month is inside `min..=max`.
    #[must_use]
    pub fn allows_month(&self, month: &MonthGrid) -> bool {
        self.min.is_none_or(|min| month.last() >= min)
            && self.max.is_none_or(|max| month.first() <= max)
    }

    /// The rules as query pairs. An unset rule gives no pair.
    #[must_use]
    pub fn to_query(&self) -> Vec<(&'static str, String)> {
        let mut pairs = Vec::new();
        if let Some(min) = self.min {
            pairs.push(("min", min.to_string()));
        }
        if let Some(max) = self.max {
            pairs.push(("max", max.to_string()));
        }
        if self.weekdays != 0 {
            let digits = (0..7)
                .filter(|bit| self.weekdays & (1 << bit) != 0)
                .map(|bit| char::from(b'1' + bit))
                .collect();
            pairs.push(("off", digits));
        }
        if !self.dates.is_empty() {
            let list: Vec<String> = self.dates.iter().map(NaiveDate::to_string).collect();
            pairs.push(("x", list.join(",")));
        }
        pairs
    }

    /// Read the rules from query values. `get` returns the value of a key.
    ///
    /// # Errors
    ///
    /// [`RuleError`] when a value does not parse, or when the list holds
    /// more than [`MAX_DISABLED_DATES`] dates.
    pub fn from_query<'a>(get: impl Fn(&str) -> Option<&'a str>) -> Result<Self, RuleError> {
        let value = |key| get(key).filter(|v| !v.is_empty());
        let date = |key: &'static str| {
            value(key)
                .map(|v| {
                    parse_iso_date(v).ok_or(RuleError {
                        key,
                        reason: "not a YYYY-MM-DD date",
                    })
                })
                .transpose()
        };
        let mut rules = Self {
            min: date("min")?,
            max: date("max")?,
            ..Self::new()
        };
        if let (Some(min), Some(max)) = (rules.min, rules.max)
            && min > max
        {
            return Err(RuleError {
                key: "max",
                reason: "is before min",
            });
        }
        for digit in value("off").unwrap_or_default().chars() {
            let weekday = digit
                .to_digit(10)
                .and_then(weekday_from_iso)
                .ok_or(RuleError {
                    key: "off",
                    reason: "not ISO weekday digits 1-7",
                })?;
            rules = rules.disable_weekday(weekday);
        }
        if let Some(list) = value("x") {
            // Count the items, not the distinct dates: a list of one date
            // repeated is long work too.
            if list.split(',').count() > MAX_DISABLED_DATES {
                return Err(RuleError {
                    key: "x",
                    reason: "too many dates",
                });
            }
            for item in list.split(',') {
                let day = parse_iso_date(item).ok_or(RuleError {
                    key: "x",
                    reason: "not a list of YYYY-MM-DD dates",
                })?;
                rules.dates.insert(day);
                if rules.dates.len() > MAX_DISABLED_DATES {
                    return Err(RuleError {
                        key: "x",
                        reason: "too many dates",
                    });
                }
            }
        }
        Ok(rules)
    }
}

/// Parse an ISO weekday digit: `1` is Monday, `7` is Sunday.
pub(crate) fn weekday_from_iso(digit: u32) -> Option<Weekday> {
    match digit {
        1..=7 => Weekday::try_from(u8::try_from(digit - 1).ok()?).ok(),
        _ => None,
    }
}

/// The ISO weekday digit: `1` is Monday, `7` is Sunday.
pub(crate) const fn weekday_to_iso(weekday: Weekday) -> u32 {
    weekday.number_from_monday()
}

/// Parse exactly `YYYY-MM-DD`.
pub(crate) fn parse_iso_date(raw: &str) -> Option<NaiveDate> {
    let b = raw.as_bytes();
    let shape = b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit());
    shape
        .then(|| NaiveDate::parse_from_str(raw, "%Y-%m-%d").ok())
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("a valid test date")
    }

    fn round_trip(rules: &DateRules) -> Result<DateRules, RuleError> {
        let pairs: HashMap<&str, String> = rules.to_query().into_iter().collect();
        DateRules::from_query(|k| pairs.get(k).map(String::as_str))
    }

    #[test]
    fn no_rules_enable_each_day() {
        let rules = DateRules::new();
        assert!(rules.is_enabled(date(2026, 11, 5)));
        assert_eq!(rules.to_query().len(), 0, "no rules, no query pairs");
    }

    #[test]
    fn min_and_max_are_inclusive() {
        let rules = DateRules::new()
            .min(date(2026, 11, 5))
            .max(date(2026, 11, 20));
        assert!(!rules.is_enabled(date(2026, 11, 4)));
        assert!(rules.is_enabled(date(2026, 11, 5)));
        assert!(rules.is_enabled(date(2026, 11, 20)));
        assert!(!rules.is_enabled(date(2026, 11, 21)));
    }

    #[test]
    fn weekends_are_disabled() {
        let rules = DateRules::new().disable_weekends();
        assert!(!rules.is_enabled(date(2026, 11, 7)), "Saturday");
        assert!(!rules.is_enabled(date(2026, 11, 8)), "Sunday");
        assert!(rules.is_enabled(date(2026, 11, 9)), "Monday");
    }

    #[test]
    fn one_date_is_disabled() {
        let rules = DateRules::new().disable_date(date(2026, 12, 25));
        assert!(!rules.is_enabled(date(2026, 12, 25)));
        assert!(rules.is_enabled(date(2026, 12, 24)));
    }

    #[test]
    fn allows_month_compares_months_not_days() {
        let rules = DateRules::new()
            .min(date(2026, 11, 20))
            .max(date(2027, 1, 3));
        let ok = |y, m| rules.allows_month(&MonthGrid::new(y, m, Weekday::Mon).expect("month"));
        assert!(!ok(2026, 10));
        assert!(ok(2026, 11), "the min month");
        assert!(ok(2026, 12));
        assert!(ok(2027, 1), "the max month");
        assert!(!ok(2027, 2));
    }

    #[test]
    fn the_query_round_trips() {
        let rules = DateRules::new()
            .min(date(2026, 1, 1))
            .max(date(2026, 12, 31))
            .disable_weekday(Weekday::Wed)
            .disable_weekends()
            .disable_dates([date(2026, 12, 25), date(2026, 1, 1)]);
        let query = rules.to_query();
        assert!(
            query.contains(&("min", "2026-01-01".to_owned())),
            "{query:?}"
        );
        assert!(query.contains(&("off", "367".to_owned())), "{query:?}");
        assert!(
            query.contains(&("x", "2026-01-01,2026-12-25".to_owned())),
            "sorted: {query:?}"
        );
        assert_eq!(round_trip(&rules), Ok(rules));
    }

    #[test]
    fn a_bad_query_value_is_an_error() {
        for (key, value) in [
            ("min", "2026-13-01"),
            ("max", "tomorrow"),
            ("off", "8"),
            ("off", "1a"),
            ("x", "2026-01-01,nope"),
        ] {
            let err = DateRules::from_query(|k| (k == key).then_some(value));
            assert!(err.is_err(), "{key}={value}");
        }
    }

    #[test]
    fn an_empty_query_value_is_no_rule() {
        let rules = DateRules::from_query(|_| Some(""));
        assert_eq!(rules, Ok(DateRules::new()));
    }

    #[test]
    fn too_many_dates_in_a_query_is_an_error() {
        let list: Vec<String> = (0..=MAX_DISABLED_DATES as u64)
            .map(|i| (date(2020, 1, 1) + chrono::Days::new(i)).to_string())
            .collect();
        let joined = list.join(",");
        let err = DateRules::from_query(|k| (k == "x").then_some(joined.as_str()));
        assert!(err.is_err());
    }

    #[test]
    #[should_panic(expected = "MAX_DISABLED_DATES")]
    fn too_many_disabled_dates_panics() {
        let _ = DateRules::new().disable_dates(
            (0..=MAX_DISABLED_DATES as u64).map(|i| date(2020, 1, 1) + chrono::Days::new(i)),
        );
    }

    #[test]
    #[should_panic(expected = "after max")]
    fn min_after_max_panics() {
        let _ = DateRules::new()
            .max(date(2026, 12, 1))
            .min(date(2026, 12, 10));
    }

    #[test]
    #[should_panic(expected = "before min")]
    fn max_before_min_panics() {
        let _ = DateRules::new()
            .min(date(2026, 12, 10))
            .max(date(2026, 12, 1));
    }

    #[test]
    fn a_query_with_min_after_max_is_an_error() {
        let err = DateRules::from_query(|k| match k {
            "min" => Some("2026-12-10"),
            "max" => Some("2026-12-01"),
            _ => None,
        });
        assert!(err.is_err());
    }

    #[test]
    fn a_repeated_date_counts_toward_the_query_limit() {
        let list = vec!["2026-11-05"; MAX_DISABLED_DATES + 1].join(",");
        let err = DateRules::from_query(|k| (k == "x").then_some(list.as_str()));
        assert!(err.is_err());
    }

    #[test]
    fn try_disable_dates_keeps_the_rules_on_error() {
        let many = (0..=MAX_DISABLED_DATES as u64).map(|i| date(2020, 1, 1) + chrono::Days::new(i));
        let rules = DateRules::new().disable_date(date(2019, 1, 1));
        let err = rules.clone().try_disable_dates(many);
        assert!(err.is_err());
        let ok = rules.try_disable_dates([date(2026, 12, 25)]).expect("fits");
        assert!(!ok.is_enabled(date(2026, 12, 25)));
        assert!(!ok.is_enabled(date(2019, 1, 1)));
    }

    /// Spec invariant 5, for each day of 1900–2100: a day is enabled if and
    /// only if `min <= day <= max`, its weekday is not disabled, and it is
    /// not in the list.
    #[test]
    fn invariant_5_holds_exhaustively() {
        let (min, max) = (date(1950, 3, 15), date(2080, 9, 10));
        let listed = [date(2000, 2, 29), date(2026, 12, 25)];
        let rules = DateRules::new()
            .min(min)
            .max(max)
            .disable_weekday(Weekday::Wed)
            .disable_weekends()
            .disable_dates(listed);
        let mut day = date(1900, 1, 1);
        while day <= date(2100, 12, 31) {
            let weekday_off = matches!(day.weekday(), Weekday::Wed | Weekday::Sat | Weekday::Sun);
            let expected = min <= day && day <= max && !weekday_off && !listed.contains(&day);
            assert_eq!(rules.is_enabled(day), expected, "{day}");
            day = day.succ_opt().expect("next day");
        }
    }

    #[test]
    fn iso_weekday_digits() {
        assert_eq!(weekday_from_iso(1), Some(Weekday::Mon));
        assert_eq!(weekday_from_iso(7), Some(Weekday::Sun));
        assert_eq!(weekday_from_iso(0), None);
        assert_eq!(weekday_from_iso(8), None);
    }
}
