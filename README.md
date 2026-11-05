# autumn-datepicker-plugin

Date, time, date-time and date-range pickers for
[Autumn](https://github.com/autumn-foundation/autumn).

- Each picker is a native input. It works with JavaScript off.
- With JavaScript, a button opens a calendar or a time list in a native
  popover.
- The server renders the month grid. A month change is one htmx request.
- The grid keys follow the WAI-ARIA APG date picker dialog. They use the
  Autumn keyboard core.
- `min`, `max`, disabled weekdays, disabled dates, the week start and the
  names (from i18n) are supported.
- Each picker binds to `Changeset<T>` and to `form_for`.

The pack passes the Autumn widget conformance check: the static check, and
axe, console and keyboard checks in Chromium.

## Install

```toml
[dependencies]
autumn-datepicker-plugin = "0.1"
```

Add the plugin. It serves the CSS, the JS, the month route and the
`/_stories` entries:

```rust
autumn_web::app()
    .plugin(autumn_datepicker_plugin::DatepickerPlugin::new())
```

Link the assets in the layout, after the core assets:

```rust
head {
    link rel="stylesheet" href=(autumn_web::ui::WIDGETS_CSS_PATH);
    script src="/static/js/htmx.min.js" defer {}
    script src="/static/js/autumn-widgets.js" defer {}
    (autumn_datepicker_plugin::pack().head())
}
```

## Use

```rust
use autumn_datepicker_plugin::{DatePicker, DateRules, date_picker};
use autumn_web::form::FieldOptions;

let picker = DatePicker::new()
    .rules(DateRules::new().min(today).disable_weekends())
    .today(today);
date_picker(&changeset, "due", "Due date", &picker, &FieldOptions::new())
```

| Picker | Input | Helper | Field type |
|---|---|---|---|
| `DatePicker` | `date` | `date_picker` | `NaiveDate` |
| `TimePicker` | `time` | `time_picker` | `NaiveTime` |
| `DateTimePicker` | `datetime-local` | `datetime_picker` | `NaiveDateTime`, `DateTime<Utc>`, `DateTime<Local>` |
| `DateRangePicker` | two `date` | `date_range_picker` | `DateRange` |

With `form_for`, use `.override_field("due", FieldControl::custom(DatePicker::new()))`.

Read the [guide](docs/guide/datepicker.md) for the keyboard, the i18n keys,
the time zones and the month route.

## Features

| Feature | Effect |
|---|---|
| `i18n` | `CalendarLocale::from_i18n` reads `datepicker-*` keys. The month route uses the request locale. |
| `system-tests` | The Chromium tests in `tests/browser.rs`. |

## Develop

```bash
cargo test --all-features
```

```bash
AUTUMN_CHROMIUM=/path/to/chrome TZ=America/New_York \
  cargo test --all-features --test browser -- --include-ignored
```

`tests/fixtures/axe.min.js` is axe-core 4.14 (MPL-2.0, see
`tests/fixtures/axe-LICENSE`). The crate does not ship it.

## License

Apache-2.0.
