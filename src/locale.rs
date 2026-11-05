//! Month names, weekday names, the week start and the button labels.
//!
//! [`CalendarLocale::english`] is the default. With the `i18n` feature,
//! [`CalendarLocale::from_i18n`] reads the `datepicker-*` keys from the app
//! bundle. A missing key keeps the English text.

use std::borrow::Cow;

use chrono::Weekday;

use crate::grid::MonthGrid;

type Text = Cow<'static, str>;

/// The text and the week start of a calendar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarLocale {
    months: [Text; 12],
    /// Monday first.
    weekdays: [Text; 7],
    /// Monday first.
    weekdays_short: [Text; 7],
    week_start: Weekday,
    /// `{month}` and `{year}` mark the parts.
    title: Text,
    previous_month: Text,
    next_month: Text,
    choose_date: Text,
    choose_time: Text,
    calendar: Text,
    times: Text,
    range_start: Text,
    range_end: Text,
}

impl Default for CalendarLocale {
    fn default() -> Self {
        Self::english()
    }
}

impl CalendarLocale {
    /// English names. The week starts on Sunday.
    #[must_use]
    pub fn english() -> Self {
        Self {
            months: [
                "January",
                "February",
                "March",
                "April",
                "May",
                "June",
                "July",
                "August",
                "September",
                "October",
                "November",
                "December",
            ]
            .map(Cow::Borrowed),
            weekdays: [
                "Monday",
                "Tuesday",
                "Wednesday",
                "Thursday",
                "Friday",
                "Saturday",
                "Sunday",
            ]
            .map(Cow::Borrowed),
            weekdays_short: ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"].map(Cow::Borrowed),
            week_start: Weekday::Sun,
            title: Cow::Borrowed("{month} {year}"),
            previous_month: Cow::Borrowed("Previous month"),
            next_month: Cow::Borrowed("Next month"),
            choose_date: Cow::Borrowed("Choose date"),
            choose_time: Cow::Borrowed("Choose time"),
            calendar: Cow::Borrowed("Calendar"),
            times: Cow::Borrowed("Times"),
            range_start: Cow::Borrowed("Start date"),
            range_end: Cow::Borrowed("End date"),
        }
    }

    /// Set the first weekday of each row.
    #[must_use]
    pub const fn week_start(mut self, weekday: Weekday) -> Self {
        self.week_start = weekday;
        self
    }

    /// Set the twelve month names, January first.
    #[must_use]
    pub fn month_names(mut self, names: [&'static str; 12]) -> Self {
        self.months = names.map(Cow::Borrowed);
        self
    }

    /// Set the weekday names, Monday first: the long name, then the short.
    #[must_use]
    pub fn weekday_names(mut self, long: [&'static str; 7], short: [&'static str; 7]) -> Self {
        self.weekdays = long.map(Cow::Borrowed);
        self.weekdays_short = short.map(Cow::Borrowed);
        self
    }

    /// Set the heading template. `{month}` and `{year}` mark the parts.
    #[must_use]
    pub fn title_template(mut self, template: &'static str) -> Self {
        self.title = Cow::Borrowed(template);
        self
    }

    /// The first weekday of each row.
    #[must_use]
    pub const fn first_weekday(&self) -> Weekday {
        self.week_start
    }

    /// The name of `month` (`1..=12`). An invalid month gives `""`.
    #[must_use]
    pub fn month_name(&self, month: u32) -> &str {
        month
            .checked_sub(1)
            .and_then(|i| self.months.get(i as usize))
            .map_or("", |name| name)
    }

    /// The long name of `weekday`.
    #[must_use]
    pub fn weekday_name(&self, weekday: Weekday) -> &str {
        &self.weekdays[weekday.num_days_from_monday() as usize]
    }

    /// The short name of `weekday`, for the column header.
    #[must_use]
    pub fn weekday_short(&self, weekday: Weekday) -> &str {
        &self.weekdays_short[weekday.num_days_from_monday() as usize]
    }

    /// The grid heading, for example `November 2026`.
    #[must_use]
    pub fn title(&self, grid: &MonthGrid) -> String {
        self.title
            .replace("{month}", self.month_name(grid.month()))
            .replace("{year}", &grid.year().to_string())
    }

    /// The label of the previous-month button.
    #[must_use]
    pub fn previous_month(&self) -> &str {
        &self.previous_month
    }

    /// The label of the next-month button.
    #[must_use]
    pub fn next_month(&self) -> &str {
        &self.next_month
    }

    /// The label of the button that opens the calendar.
    #[must_use]
    pub fn choose_date(&self) -> &str {
        &self.choose_date
    }

    /// The label of the button that opens the time list.
    #[must_use]
    pub fn choose_time(&self) -> &str {
        &self.choose_time
    }

    /// The label of the calendar dialog.
    #[must_use]
    pub fn calendar(&self) -> &str {
        &self.calendar
    }

    /// The label of the time list.
    #[must_use]
    pub fn times(&self) -> &str {
        &self.times
    }

    /// The label of the first input of a range.
    #[must_use]
    pub fn range_start(&self) -> &str {
        &self.range_start
    }

    /// The label of the second input of a range.
    #[must_use]
    pub fn range_end(&self) -> &str {
        &self.range_end
    }
}

#[cfg(feature = "i18n")]
impl CalendarLocale {
    /// The calendar text for `locale`, from its bundle.
    ///
    /// The keys are in `i18n/en.ftl` of this crate. A key that the bundle
    /// does not have keeps the English text. `datepicker-week-start` is an
    /// ISO weekday digit: `1` is Monday, `7` is Sunday.
    #[must_use]
    pub fn from_i18n(locale: &autumn_web::i18n::Locale) -> Self {
        let mut out = Self::english();
        let Some(bundle) = locale.bundle() else {
            return out;
        };
        let lookup = |key: &str| bundle.lookup(locale.tag(), key).map(Cow::Owned);
        let set = |slot: &mut Text, key: &str| {
            if let Some(text) = lookup(key) {
                *slot = text;
            }
        };
        for (i, slot) in out.months.iter_mut().enumerate() {
            set(slot, &format!("datepicker-month-{}", i + 1));
        }
        for i in 0..7 {
            set(
                &mut out.weekdays[i],
                &format!("datepicker-weekday-{}", i + 1),
            );
            set(
                &mut out.weekdays_short[i],
                &format!("datepicker-weekday-short-{}", i + 1),
            );
        }
        if let Some(start) = lookup("datepicker-week-start")
            .and_then(|digit| digit.trim().parse().ok())
            .and_then(crate::rules::weekday_from_iso)
        {
            out.week_start = start;
        }
        // Fluent writes `{ $month }`. Keep the `{month}` form inside.
        if let Some(title) = lookup("datepicker-title") {
            out.title = Cow::Owned(
                title
                    .replace("{ $month }", "{month}")
                    .replace("{$month}", "{month}")
                    .replace("{ $year }", "{year}")
                    .replace("{$year}", "{year}"),
            );
        }
        set(&mut out.previous_month, "datepicker-previous-month");
        set(&mut out.next_month, "datepicker-next-month");
        set(&mut out.choose_date, "datepicker-choose-date");
        set(&mut out.choose_time, "datepicker-choose-time");
        set(&mut out.calendar, "datepicker-calendar");
        set(&mut out.times, "datepicker-times");
        set(&mut out.range_start, "datepicker-range-start");
        set(&mut out.range_end, "datepicker-range-end");
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_names() {
        let l = CalendarLocale::english();
        assert_eq!(l.month_name(1), "January");
        assert_eq!(l.month_name(12), "December");
        assert_eq!(l.month_name(0), "");
        assert_eq!(l.month_name(13), "");
        assert_eq!(l.weekday_name(Weekday::Mon), "Monday");
        assert_eq!(l.weekday_short(Weekday::Sun), "Su");
        assert_eq!(l.first_weekday(), Weekday::Sun);
        assert_eq!(l.previous_month(), "Previous month");
        assert_eq!(l.next_month(), "Next month");
        assert_eq!(l.choose_date(), "Choose date");
        assert_eq!(l.choose_time(), "Choose time");
    }

    #[test]
    fn the_title_fills_the_template() {
        let grid = MonthGrid::new(2026, 11, Weekday::Sun).expect("month");
        assert_eq!(CalendarLocale::english().title(&grid), "November 2026");
        let german = CalendarLocale::english()
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
            .title_template("{month} {year}");
        let march = MonthGrid::new(2026, 3, Weekday::Mon).expect("month");
        assert_eq!(german.title(&march), "März 2026");
        let year_first = CalendarLocale::english().title_template("{year}年{month}");
        assert_eq!(year_first.title(&march), "2026年March");
    }

    #[test]
    fn custom_weekdays_and_week_start() {
        let l = CalendarLocale::english()
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
            .week_start(Weekday::Mon);
        assert_eq!(l.weekday_name(Weekday::Sun), "Sonntag");
        assert_eq!(l.weekday_short(Weekday::Wed), "Mi");
        assert_eq!(l.first_weekday(), Weekday::Mon);
    }

    #[cfg(feature = "i18n")]
    mod i18n {
        use super::*;
        use autumn_web::i18n::{Bundle, I18nConfig, Locale};
        use std::collections::HashMap;
        use std::sync::Arc;

        fn bundle(de: &[(&str, &str)]) -> Arc<Bundle> {
            let config = I18nConfig {
                supported_locales: vec!["en".into(), "de".into()],
                ..I18nConfig::default()
            };
            let mut messages = HashMap::new();
            messages.insert("en".to_owned(), HashMap::new());
            messages.insert(
                "de".to_owned(),
                de.iter()
                    .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                    .collect(),
            );
            Arc::new(Bundle::from_messages(messages, &config))
        }

        #[test]
        fn from_i18n_reads_the_bundle() {
            let b = bundle(&[
                ("datepicker-month-3", "März"),
                ("datepicker-weekday-1", "Montag"),
                ("datepicker-weekday-short-1", "Mo"),
                ("datepicker-week-start", "1"),
                ("datepicker-title", "{ $month } { $year }"),
                ("datepicker-next-month", "Nächster Monat"),
            ]);
            let l = CalendarLocale::from_i18n(&Locale::new("de").with_bundle(b));
            assert_eq!(l.month_name(3), "März");
            assert_eq!(l.weekday_name(Weekday::Mon), "Montag");
            assert_eq!(l.weekday_short(Weekday::Mon), "Mo");
            assert_eq!(l.first_weekday(), Weekday::Mon);
            assert_eq!(l.next_month(), "Nächster Monat");
            let march = MonthGrid::new(2026, 3, Weekday::Mon).expect("month");
            assert_eq!(l.title(&march), "März 2026");
        }

        #[test]
        fn a_missing_key_keeps_english() {
            let l = CalendarLocale::from_i18n(&Locale::new("de").with_bundle(bundle(&[])));
            assert_eq!(l, CalendarLocale::english());
            let no_bundle = CalendarLocale::from_i18n(&Locale::new("de"));
            assert_eq!(no_bundle, CalendarLocale::english());
        }

        #[test]
        fn a_bad_week_start_keeps_sunday() {
            let b = bundle(&[("datepicker-week-start", "9")]);
            let l = CalendarLocale::from_i18n(&Locale::new("de").with_bundle(b));
            assert_eq!(l.first_weekday(), Weekday::Sun);
        }

        #[test]
        fn the_shipped_ftl_files_load_and_name_each_key() {
            let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("i18n");
            let config = I18nConfig {
                supported_locales: vec!["en".into(), "de".into()],
                ..I18nConfig::default()
            };
            let b = Arc::new(Bundle::load_from_dir(&dir, &config).expect("load i18n/"));
            for tag in ["en", "de"] {
                let l = CalendarLocale::from_i18n(&Locale::new(tag).with_bundle(Arc::clone(&b)));
                for m in 1..=12 {
                    assert!(!l.month_name(m).is_empty(), "{tag} month {m}");
                }
            }
            let de = CalendarLocale::from_i18n(&Locale::new("de").with_bundle(b));
            assert_eq!(de.first_weekday(), Weekday::Mon);
            assert_eq!(de.month_name(3), "März");
        }
    }
}
