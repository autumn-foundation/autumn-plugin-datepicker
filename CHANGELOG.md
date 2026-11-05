# Changelog

## [Unreleased]

### Added

- `DatePicker`, `TimePicker`, `DateTimePicker` and `DateRangePicker` on
  native inputs, with a server-rendered calendar or time list in a native
  popover (autumn-foundation/autumn#3413).
- `GET /_datepicker/month`: one calendar for an htmx month change.
- `DateRules`: `min`, `max`, disabled weekdays and disabled dates, with
  `try_disable_dates` for a list from a database.
- `CalendarLocale` with `Label` setters, and `CalendarLocale::from_i18n` with
  the `i18n` feature.
- `DatePicker::time_zone` and `DateTimePicker::time_zone`: today and the wall
  clock in the user zone.
- `DateRange`: one form field with a start and an end.
- `value::parse_in_zone`: read a `datetime-local` value in a user zone.
- The `datepicker` widget pack with seven stories, and `DatepickerPlugin`.
