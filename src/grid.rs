//! The month grid: a pure function of a year, a month and a week start.
//!
//! The grid uses [`NaiveDate`], so a DST change cannot add, drop or move a
//! day.

use chrono::{Datelike, Months, NaiveDate, Weekday};

/// The number of days in `month` of `year` (proleptic Gregorian).
///
/// Returns 0 when `month` is not `1..=12`.
#[must_use]
pub const fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// `true` for a Gregorian leap year.
const fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// One month, laid out in rows of seven days.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonthGrid {
    first: NaiveDate,
    week_start: Weekday,
}

impl MonthGrid {
    /// The grid for `month` of `year`. `None` when the month is not valid.
    #[must_use]
    pub fn new(year: i32, month: u32, week_start: Weekday) -> Option<Self> {
        NaiveDate::from_ymd_opt(year, month, 1).map(|first| Self { first, week_start })
    }

    /// The grid of the month that holds `date`.
    #[must_use]
    pub fn containing(date: NaiveDate, week_start: Weekday) -> Self {
        // Day 1 of a valid date's month is always valid.
        let first = date.with_day(1).unwrap_or(date);
        Self { first, week_start }
    }

    /// The year.
    #[must_use]
    pub fn year(&self) -> i32 {
        self.first.year()
    }

    /// The month, `1..=12`.
    #[must_use]
    pub fn month(&self) -> u32 {
        self.first.month()
    }

    /// The first weekday of each row.
    #[must_use]
    pub const fn week_start(&self) -> Weekday {
        self.week_start
    }

    /// The first day of the month.
    #[must_use]
    pub const fn first(&self) -> NaiveDate {
        self.first
    }

    /// The last day of the month.
    #[must_use]
    pub fn last(&self) -> NaiveDate {
        self.first.with_day(self.days()).unwrap_or(self.first)
    }

    /// The number of days in the month.
    #[must_use]
    pub fn days(&self) -> u32 {
        days_in_month(self.year(), self.month())
    }

    /// The number of empty cells before day 1.
    #[must_use]
    pub fn offset(&self) -> u32 {
        (self.first.weekday().num_days_from_monday() + 7 - self.week_start.num_days_from_monday())
            % 7
    }

    /// The weekday of each column.
    #[must_use]
    pub fn weekdays(&self) -> [Weekday; 7] {
        let mut day = self.week_start;
        std::array::from_fn(|_| {
            let this = day;
            day = day.succ();
            this
        })
    }

    /// The rows. A cell outside the month is `None`.
    #[must_use]
    pub fn weeks(&self) -> Vec<[Option<NaiveDate>; 7]> {
        let offset = self.offset();
        let days = self.days();
        let rows = (offset + days).div_ceil(7);
        (0..rows)
            .map(|row| {
                std::array::from_fn(|col| {
                    // `col` is below 7, so the cast is exact.
                    #[allow(clippy::cast_possible_truncation)]
                    let cell = row * 7 + col as u32;
                    if cell < offset || cell >= offset + days {
                        None
                    } else {
                        self.first.with_day(cell - offset + 1)
                    }
                })
            })
            .collect()
    }

    /// The month before. `None` before the first month chrono supports.
    #[must_use]
    pub fn previous(&self) -> Option<Self> {
        self.first
            .checked_sub_months(Months::new(1))
            .map(|first| Self { first, ..*self })
    }

    /// The month after. `None` after the last month chrono supports.
    #[must_use]
    pub fn next(&self) -> Option<Self> {
        self.first
            .checked_add_months(Months::new(1))
            .map(|first| Self { first, ..*self })
    }

    /// The month as `YYYY-MM`, the shape of `<input type="month">`.
    #[must_use]
    pub fn key(&self) -> String {
        format!("{:04}-{:02}", self.year(), self.month())
    }

    /// Parse a `YYYY-MM` key.
    #[must_use]
    pub fn parse_key(key: &str, week_start: Weekday) -> Option<Self> {
        let (year, month) = key.split_once('-')?;
        let digits = |s: &str, n: usize| s.len() == n && s.bytes().all(|b| b.is_ascii_digit());
        if !digits(year, 4) || !digits(month, 2) {
            return None;
        }
        Self::new(year.parse().ok()?, month.parse().ok()?, week_start)
    }
}

#[cfg(test)]
#[allow(clippy::cast_possible_truncation)]
mod tests {
    use super::*;

    const ALL_WEEKDAYS: [Weekday; 7] = [
        Weekday::Mon,
        Weekday::Tue,
        Weekday::Wed,
        Weekday::Thu,
        Weekday::Fri,
        Weekday::Sat,
        Weekday::Sun,
    ];

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("a valid test date")
    }

    fn grid(y: i32, m: u32, ws: Weekday) -> MonthGrid {
        MonthGrid::new(y, m, ws).expect("a valid test month")
    }

    /// The days in grid order, without the empty cells.
    fn days(g: MonthGrid) -> Vec<NaiveDate> {
        g.weeks().iter().flatten().flatten().copied().collect()
    }

    #[test]
    fn days_in_month_follows_the_gregorian_leap_rule() {
        assert_eq!(days_in_month(2024, 2), 29, "divisible by 4");
        assert_eq!(days_in_month(2026, 2), 28, "not divisible by 4");
        assert_eq!(days_in_month(2100, 2), 28, "divisible by 100, not 400");
        assert_eq!(days_in_month(2000, 2), 29, "divisible by 400");
        assert_eq!(days_in_month(2026, 1), 31);
        assert_eq!(days_in_month(2026, 4), 30);
        assert_eq!(days_in_month(2026, 12), 31);
        assert_eq!(days_in_month(2026, 0), 0);
        assert_eq!(days_in_month(2026, 13), 0);
    }

    #[test]
    fn an_invalid_month_has_no_grid() {
        assert!(MonthGrid::new(2026, 0, Weekday::Mon).is_none());
        assert!(MonthGrid::new(2026, 13, Weekday::Mon).is_none());
    }

    #[test]
    fn a_leap_february_ends_on_the_29th() {
        let g = grid(2024, 2, Weekday::Sun);
        assert_eq!(g.last(), date(2024, 2, 29));
        assert_eq!(days(g).len(), 29);
        let g = grid(2100, 2, Weekday::Sun);
        assert_eq!(g.last(), date(2100, 2, 28));
    }

    #[test]
    fn november_2026_with_a_sunday_start() {
        // 1 November 2026 is a Sunday.
        let g = grid(2026, 11, Weekday::Sun);
        assert_eq!(g.offset(), 0);
        assert_eq!(g.weeks().len(), 5);
        assert_eq!(g.weeks()[0][0], Some(date(2026, 11, 1)));
        assert_eq!(g.weeks()[4][1], Some(date(2026, 11, 30)));
        assert_eq!(g.weeks()[4][2], None);
    }

    #[test]
    fn november_2026_with_a_monday_start() {
        let g = grid(2026, 11, Weekday::Mon);
        assert_eq!(g.offset(), 6);
        assert_eq!(g.weeks().len(), 6);
        assert_eq!(g.weeks()[0][6], Some(date(2026, 11, 1)));
        assert_eq!(g.weeks()[0][5], None);
        assert_eq!(g.weekdays()[0], Weekday::Mon);
        assert_eq!(g.weekdays()[6], Weekday::Sun);
    }

    #[test]
    fn each_week_start_puts_day_one_in_its_weekday_column() {
        for ws in ALL_WEEKDAYS {
            let g = grid(2026, 11, ws);
            let column = g.weeks()[0]
                .iter()
                .position(|c| *c == Some(date(2026, 11, 1)))
                .expect("day 1 is in row 0");
            assert_eq!(g.weekdays()[column], Weekday::Sun, "week start {ws}");
            assert_eq!(g.weekdays()[0], ws);
        }
    }

    #[test]
    fn february_2026_with_a_sunday_start_fills_four_rows() {
        // 1 February 2026 is a Sunday, and the month has 28 days.
        let g = grid(2026, 2, Weekday::Sun);
        assert_eq!(g.weeks().len(), 4);
    }

    #[test]
    fn a_dst_month_has_each_day_one_time() {
        // US DST starts 8 March 2026. EU DST ends 25 October 2026.
        for (y, m, n) in [(2026, 3, 31), (2026, 10, 31), (2026, 11, 30)] {
            let g = grid(y, m, Weekday::Sun);
            let all = days(g);
            assert_eq!(all.len(), n, "{y}-{m}");
            for (i, d) in all.iter().enumerate() {
                assert_eq!(d.day() as usize, i + 1, "{y}-{m}: no gap, no repeat");
            }
        }
        let march = grid(2026, 3, Weekday::Sun);
        assert_eq!(
            march.weeks()[1][0],
            Some(date(2026, 3, 8)),
            "8 March is a Sunday"
        );
    }

    #[test]
    fn previous_and_next_cross_the_year() {
        let g = grid(2026, 1, Weekday::Mon);
        assert_eq!(g.previous().map(|p| p.key()), Some("2025-12".to_owned()));
        let g = grid(2026, 12, Weekday::Mon);
        assert_eq!(g.next().map(|n| n.key()), Some("2027-01".to_owned()));
        assert_eq!(g.next().map(|n| n.week_start()), Some(Weekday::Mon));
    }

    #[test]
    fn the_key_round_trips() {
        let g = grid(2026, 3, Weekday::Sat);
        assert_eq!(g.key(), "2026-03");
        assert_eq!(MonthGrid::parse_key("2026-03", Weekday::Sat), Some(g));
        for bad in [
            "2026-13",
            "2026-3",
            "26-03",
            "2026-03-01",
            "",
            "abcd-ef",
            "+2026-03",
        ] {
            assert_eq!(MonthGrid::parse_key(bad, Weekday::Sat), None, "{bad:?}");
        }
    }

    #[test]
    fn containing_finds_the_month() {
        let g = MonthGrid::containing(date(2024, 2, 29), Weekday::Mon);
        assert_eq!(g.key(), "2024-02");
        assert_eq!(g.first(), date(2024, 2, 1));
    }

    /// The spec, checked for each month of 1900–2100 and each week start.
    #[test]
    fn the_grid_invariants_hold_exhaustively() {
        for year in 1900..=2100 {
            for month in 1..=12 {
                for ws in ALL_WEEKDAYS {
                    let g = grid(year, month, ws);
                    let n = days_in_month(year, month);
                    let offset = g.offset();
                    // 2: day `d` is at cell `offset + d - 1`.
                    let expected_offset = (g.first().weekday().num_days_from_monday() + 7
                        - ws.num_days_from_monday())
                        % 7;
                    assert_eq!(offset, expected_offset);
                    let weeks = g.weeks();
                    // 3: the fewest rows that hold the cells.
                    assert_eq!(weeks.len() as u32, (offset + n).div_ceil(7));
                    assert!((4..=6).contains(&weeks.len()));
                    let cells: Vec<Option<NaiveDate>> = weeks.iter().flatten().copied().collect();
                    for (i, cell) in cells.iter().enumerate() {
                        let i = i as u32;
                        if i < offset || i >= offset + n {
                            assert_eq!(*cell, None);
                        } else {
                            // 1: each day exactly one time, in order.
                            assert_eq!(*cell, Some(date(year, month, i - offset + 1)));
                        }
                    }
                    // 4: column `c` has the weekday `week_start + c`.
                    for (c, wd) in g.weekdays().iter().enumerate() {
                        assert_eq!(
                            wd.num_days_from_monday(),
                            (ws.num_days_from_monday() + c as u32) % 7
                        );
                        for row in &weeks {
                            if let Some(d) = row[c] {
                                assert_eq!(d.weekday(), *wd);
                            }
                        }
                    }
                }
            }
        }
    }
}
