# Plan: the datepicker widget pack (autumn#3413)

Part of autumn#3393, Phase 2. The seam (autumn#3400), the keyboard core
(autumn#3397) and the popover (autumn#3402) are in `autumn-web` 0.10.0.

## Goal

Ship `autumn-datepicker-plugin`: a widget pack with a date picker, a time
picker, a date-time picker and a date-range picker. Each one starts as a
native input. JavaScript adds a calendar or a time list in a popover.

## Brainstorming

1. The native `<input type="date">` stays. It is the no-JS control, and it
   posts the value that the six `autumn_web::form` deserializers read.
2. A toggle button opens a native `popover`. The button has `hidden` until the
   pack script runs, so a no-JS page shows no dead button.
3. The server renders the month grid (`<table role="grid">`). The pack JS
   never builds day cells.
4. A month change is one htmx `GET` to a plugin route. The route returns the
   new grid. `hx-include` sends the current input value, so the new grid
   shows the selection.
5. The grid is a pure function: `MonthGrid::new(year, month, week_start)`.
   It uses `chrono::NaiveDate`, so DST cannot move a day.
6. `DateRules` holds `min`, `max`, the disabled weekdays and the disabled
   dates. The grid and the route read the same rules. The route reads them
   from the query string. The server needs no state.
7. `CalendarLocale` holds the month and weekday names, the week start and
   the button labels. `CalendarLocale::english()` is the default.
   `CalendarLocale::from_i18n(&Locale)` reads `datepicker-*` keys from the app
   bundle (`i18n` feature).
8. The keyboard core gives the roving tab stop, ArrowLeft/ArrowRight, focus
   tracking and `aria-expanded`. The core has no 2-D grid. A small pack
   script adds ArrowUp/ArrowDown (one week), PageUp/PageDown (one month),
   Shift+PageUp/PageDown (one year), Home/End (week start and end), and the
   move to the next month at the month edge.
9. A time picker is a `role="listbox"` of slots on the keyboard core
   (`data-autumn-roving`, typeahead). It needs no pack key code.
10. A date-time picker is a `datetime-local` input with both toggles.
11. A date range is one form field. `DateRange` serializes to
    `[start, end]`. The two native inputs post `field=start&field=end`.
    `serde_html_form` reads the repeated key back into `DateRange`.
12. An inline calendar mode shows the grid without a popover. The stories
    use it, so axe and the roving check see the grid.
13. Each picker is a `FieldRenderer`. It binds to `Changeset<T>` with a
    helper and to `form_for` with `FieldControl::custom`.
14. `DatepickerPlugin` installs the pack and the route.

## Reverse brainstorming

Question: "How can a date picker fail?" Each answer gives a guard.

| Way to fail | Guard |
|---|---|
| A day is missing or shows two times in a DST month. | `NaiveDate` grid. Unit tests for the US and EU DST months. An exhaustive test over 1900–2100 and all week starts. |
| 29 February is wrong. | Leap-year tests: 2024, 2100 (not leap), 2000 (leap). |
| The week start moves the first day to the wrong column. | Tests for each of the seven week starts. |
| A user picks a date outside `min`/`max`. | The cell gets `aria-disabled="true"`. The input keeps `min` and `max`, so the browser also checks it. The app still validates on the server. |
| The previous-month button goes to a month with no valid day. | The button is `disabled` when the month is before `min` or after `max`. |
| A forged query string breaks the route. | The route parses each value. A bad value gives 400. Maud escapes all text. The id has a strict charset. |
| A long disabled-date list makes a long URL. | At most 400 dates in a query, counted with repeats. At most 16 KiB of query. |
| JS date math slips one day across DST. | The pack JS uses UTC methods only (`setUTCFullYear`, `getUTC*`). A browser test runs in `America/New_York` across the DST week. |
| PageUp lands on 31 February. | The day clamps to the month length. A browser test checks 31 January to February. |
| Focus is lost after an htmx swap. | The script keeps the target date and focuses it after the swap. A browser test checks it. |
| The core moves Home/End to the first or last day of the month. | The pack script handles Home/End on the grid first and calls `preventDefault`. The core then skips the key. |
| A popover opens with focus on the toggle. | On `toggle` to open, the script focuses the grid tab stop. |
| The datetime value loses seconds or moves one hour. | A property test round-trips each value through the six deserializers and `parse_in_zone`, and through `parse_local_datetime` on whole minutes. |
| A `DateTime<Utc>` value shows with a `Z` and the browser drops it. | The renderer writes the `datetime-local` shape. A test checks it. |
| The no-JS page shows a dead button. | The toggle has `hidden`. A browser test with JS off checks it. |
| A pack class collides with core, or CSS sets a public token. | `run_widget_conformance` in the test suite. |

## Six thinking hats

- **White (facts).** `FieldContext::value` is the raw changeset string. For
  `DateTime<Utc>` it is RFC 3339 with `Z`, which a `datetime-local` input
  rejects. The core keyboard list is 1-D. The browser conformance runner
  mounts only stories, not plugin routes. `FieldOptions` has no getter for
  `min`/`max`, so the picker config holds its own rules. No Verus
  toolchain is used.
- **Red (feeling).** Users want the calendar to feel like other date pickers:
  arrows move days, PageUp moves months, Escape closes.
- **Black (risk).** The pack script has key code, and the seam guide says
  "no arrow-key code". The core cannot do a grid, so the script handles only
  the keys that the core does not have. The ADR records this. A fall-back
  hour is ambiguous: `datetime-local` has no offset, so the later instant
  maps to the earlier one. The property test states this exactly.
- **Yellow (value).** Apps get a first-party picker. Most styled component
  suites ship one (autumn#3413). The grid and the rules are a pure, tested core.
- **Green (alternatives).** Render all months in the page (rejected: large
  HTML). Build the grid in JS (rejected: two renderers, and DST bugs). Keep
  state on the server per picker (rejected: the query string is enough). A
  grid mode in the core (good, but it is an `autumn-web` change; this repo
  does not change core).
- **Blue (process).** RED: tests for the grid, the rules, the locale, the
  values, the renderers, the route, conformance, the round trip and the
  browser keys. Run them and see them fail. GREEN: the smallest code.
  REFACTOR: clippy pedantic, shared helpers. Then the README, the guide, an
  ADR, CI, a code review and fixes.

## Spec (the invariants)

An exhaustive test checks each invariant over 1900–2100: invariants 1–4 in
`src/grid.rs` for the seven week starts, invariant 5 in `src/rules.rs` for
each day. The test is the proof for that finite domain.

1. Each day `1..=days_in_month` occurs exactly one time.
2. The day `d` is at cell `offset + d - 1`, with
   `offset = (weekday(first) - week_start) mod 7`.
3. Each row has seven cells. The grid has 4, 5 or 6 rows: the fewest that
   hold `offset + days_in_month` cells.
4. Column `c` has the weekday `(week_start + c) mod 7`.
5. A day is enabled if and only if `min <= day <= max`, its weekday is not
   disabled, and it is not in the disabled list.

## Scope

In scope: the four pickers, the inline calendar, the route, the locale, the
stories, the assets, the static and browser conformance, keyboard browser
tests, the property test, the README, the guide, an ADR, CI.

Out of scope: a core grid mode in `autumn-widgets.js`, a `#[model]`
attribute that selects the picker, week numbers, multi-month views.
