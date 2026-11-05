# Date Picker

`autumn-datepicker-plugin` adds four pickers. Each one is a native input
first. JavaScript adds a calendar or a time list.

| Picker | Native input | Popover |
|---|---|---|
| `DatePicker` | `<input type="date">` | Calendar |
| `TimePicker` | `<input type="time">` | Time list |
| `DateTimePicker` | `<input type="datetime-local">` | Calendar and time list |
| `DateRangePicker` | Two `<input type="date">` | One calendar |

## Set up

1. Add the plugin:

   ```rust
   autumn_web::app().plugin(autumn_datepicker_plugin::DatepickerPlugin::new())
   ```

   The plugin serves `datepicker.css`, `datepicker.js` and
   `GET /_datepicker/month`. It adds the stories to `/_stories`.

2. Link the assets after `autumn-widgets.js` and htmx:

   ```rust
   (autumn_datepicker_plugin::pack().head())
   ```

## Bind a field

With a `Changeset<T>`:

```rust
use autumn_datepicker_plugin::{DatePicker, date_picker};

date_picker(&changeset, "due", "Due date", &DatePicker::new(), &FieldOptions::new())
```

With `form_for`:

```rust
form_for(&changeset, "/tasks", "post")
    .override_field("due", FieldControl::custom(DatePicker::new()))
```

The framework writes the label, the hint, the errors and the ARIA links.
The `FieldOptions` passthrough (`data-*`, `hx-*`, `required`) goes on the
native input.

Do not start a field name with `dp-`. The month route uses that prefix.

## Limits and disabled days

```rust
use autumn_datepicker_plugin::DateRules;

let rules = DateRules::new()
    .min(today)
    .max(today + chrono::Days::new(90))
    .disable_weekends()
    .disable_date(christmas);
let picker = DatePicker::new().rules(rules);
```

- A disabled day gets `aria-disabled="true"`. A click does nothing. The
  arrow keys skip it.
- The input gets `min` and `max`, so the browser also checks them.
- A month button is `disabled` when the month is outside `min..=max`.
- The rules go into the month route URL. At most 400 disabled dates fit
  (`MAX_DISABLED_DATES`). For more, check the dates on the server.

The rules help the user. They are not validation. Validate the posted value
on the server.

## Today

Pass today in the user zone:

```rust
async fn edit(clock: Clock, tz: TimeZone) -> Markup {
    let today = clock.now().with_timezone(&*tz).date_naive();
    date_picker(&cs, "due", "Due", &DatePicker::new().today(today), &FieldOptions::new())
}
```

Without `today`, the picker uses the ambient clock in UTC.

## Locale

`CalendarLocale` holds the month names, the weekday names, the week start,
the heading template and the labels. The default is English, with Sunday
first.

```rust
let locale = CalendarLocale::english().week_start(chrono::Weekday::Mon);
let picker = DatePicker::new().locale(locale);
```

With the `i18n` feature, read the text from the app bundle:

```rust
async fn edit(locale: Locale) -> Markup {
    let picker = DatePicker::new().locale(CalendarLocale::from_i18n(&locale));
    // …
}
```

Copy the keys from `i18n/en.ftl` of this crate into your
`i18n/<locale>.ftl`. `i18n/de.ftl` is an example. A missing key keeps the
English text.

| Key | Value |
|---|---|
| `datepicker-week-start` | ISO weekday digit: `1` is Monday, `7` is Sunday |
| `datepicker-title` | `{ $month } { $year }` |
| `datepicker-month-1` … `-12` | Month names |
| `datepicker-weekday-1` … `-7` | Weekday names, Monday first |
| `datepicker-weekday-short-1` … `-7` | Column headers |
| `datepicker-previous-month`, `datepicker-next-month` | Button labels |
| `datepicker-choose-date`, `datepicker-choose-time` | Toggle labels |
| `datepicker-calendar`, `datepicker-times` | Popover labels |
| `datepicker-range-start`, `datepicker-range-end` | Range input labels |

The month route uses the request locale when the request has a bundle.
Without a bundle, it uses `DatepickerPlugin::locale`.

## Time picker

```rust
let picker = TimePicker::new()
    .range(NaiveTime::from_hms_opt(9, 0, 0).unwrap(), NaiveTime::from_hms_opt(17, 0, 0).unwrap())
    .step_minutes(15)
    .label_format("%-I:%M %p");
```

The list is a `role="listbox"` on the keyboard core. The input value stays
`HH:MM`.

## Date and time zones

`DateTimePicker` writes the `datetime-local` shape: `YYYY-MM-DDTHH:MM`, and
`:SS` only when the seconds are not zero. It never writes an offset.

| Field type | Read the post with |
|---|---|
| `NaiveDateTime` | `deserialize_naive_datetime_local` |
| `DateTime<Utc>` | `deserialize_datetime_local_utc` |
| `DateTime<Local>` | `deserialize_datetime_local_local` |
| `DateTime<Utc>` shown in a user zone | `value::parse_in_zone(raw, tz)` |

For a user zone, set `DateTimePicker::time_zone(tz)`. The input shows the
wall clock in `tz`.

A DST fall-back change repeats one hour. `datetime-local` has no offset, so
the later instant of that hour reads back as the earlier one. A wall clock
in the spring-forward gap does not exist: `parse_in_zone` returns `None`.
`tests/round_trip.rs` checks these rules with a property test.

## Date range

Bind a `DateRange` field:

```rust
#[derive(Serialize, Deserialize)]
struct Booking {
    stay: DateRange,
}
```

The two inputs post `stay=2026-11-05&stay=2026-11-12`. `DateRange` reads the
repeated key. For an optional range, use
`#[serde(default, deserialize_with = "DateRange::deserialize_option")]`.

In the calendar, the first click sets the start. The second click sets the
end and closes the popover.

## Keyboard

The calendar follows the APG date picker dialog.

| Key | Effect |
|---|---|
| ArrowLeft, ArrowRight | Previous or next day. At the month edge, the next month loads. |
| ArrowUp, ArrowDown | Same day, previous or next week |
| Home, End | First or last day of the week |
| PageUp, PageDown | Same day, previous or next month. The day clamps to the month length. |
| Shift+PageUp, Shift+PageDown | Same day, previous or next year |
| Enter, Space | Pick the day |
| Escape | Close the popover. Focus goes to the toggle. |
| Tab | Move between the month buttons and the grid. Focus that leaves closes the popover. |

The keyboard core gives the roving tab stop, ArrowLeft/ArrowRight inside
the grid and `aria-expanded`. The core has no 2-D grid mode, so
`datepicker.js` handles the other keys. It calls `preventDefault`, so the
core skips them. See [ADR 0001](../adr/0001-server-grid-and-keyboard-split.md).

## Inline calendar

`DatePicker::inline()` shows the calendar under the input, with no popover.

## Month route

`GET /_datepicker/month` returns one calendar. Each route key starts with
`dp-`:

| Key | Value |
|---|---|
| `dp-id` | The field id |
| `dp-mode` | `date`, `datetime` or `range` |
| `dp-ws` | The week start, ISO digit |
| `dp-month` | `YYYY-MM` |
| `dp-today` | `YYYY-MM-DD`, optional |
| `dp-min`, `dp-max` | `YYYY-MM-DD`, optional |
| `dp-off` | Disabled ISO weekdays, for example `67` |
| `dp-x` | Disabled dates, comma-separated |
| `<field name>` | The input value or values |

A bad value gives 400. The route keeps no state.

## CSS hooks

| Class | Element |
|---|---|
| `autumn-datepicker-field` | The picker root |
| `autumn-datepicker-input` | Each native input |
| `autumn-datepicker-toggle` | Each toggle button |
| `autumn-datepicker-dialog` | Each popover |
| `autumn-datepicker-calendar` | The calendar |
| `autumn-datepicker-day` | A day cell. `--today`, `--range-start`, `--range-end`, `--in-range` |
| `autumn-datepicker-time` | A time option |

`--autumn-datepicker-cell` sets the cell size. The CSS uses only the public
Autumn tokens, so the dark scheme works.

## Tests

| Suite | What it checks |
|---|---|
| `src/grid.rs` | Leap years, DST months, the seven week starts, and each month of 1900–2100 |
| `tests/round_trip.rs` | The six deserializers and `parse_in_zone` (property test) |
| `tests/conformance.rs` | The static widget conformance check |
| `tests/browser.rs` | axe, console, Tab, the grid keys, htmx month changes, no-JS |
