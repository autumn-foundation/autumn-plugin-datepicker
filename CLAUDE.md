# CLAUDE.md - autumn-datepicker-plugin

A widget pack for Autumn (`autumn-web` 0.10). See `README.md` and
`docs/guide/datepicker.md`.

## Layout

| Path | Contents |
|---|---|
| `src/grid.rs` | The pure month grid. The exhaustive invariant test is here. |
| `src/rules.rs` | `DateRules` and its query encoding |
| `src/locale.rs` | `CalendarLocale`, `from_i18n` |
| `src/value.rs` | Input value parse and format, `DateRange`, `parse_in_zone` |
| `src/calendar.rs` | The grid markup and the route URL |
| `src/widgets.rs` | The four `FieldRenderer` pickers |
| `src/route.rs` | `GET /_datepicker/month` |
| `src/pack.rs` | The `WidgetPack`, the assets and the stories |
| `assets/` | `datepicker.css` (tokens only), `datepicker.js` |
| `i18n/` | The `datepicker-*` keys. `en.ftl` is the full list. |

## Rules

- Follow the Autumn widget contract (`autumn_web::widget_pack`). Run
  `tests/conformance.rs` after each change to markup, CSS or JS.
- CSS uses only the public tokens and `--autumn-datepicker-*`. No raw colors.
- JS date math uses UTC only (`setUTCFullYear`, `getUTC*`).
- The JS handles only the grid keys that the core does not have. See
  `docs/adr/0001-server-grid-and-keyboard-split.md`.
- Each route query key starts with `dp-`.
- Comments and docs are short, in ASD-STE100 style.
- No `unwrap()` in `src/` outside tests.

## Commands

- Lint: `cargo fmt --all && cargo clippy --all-features --all-targets -- -D warnings`
- Test: `cargo test --all-features`
- DST: `TZ=America/New_York cargo test --all-features --test round_trip`
- Browser: `AUTUMN_CHROMIUM=<chrome> TZ=America/New_York cargo test --all-features --test browser -- --include-ignored`
- Coverage: `cargo llvm-cov --all-features --summary-only`
