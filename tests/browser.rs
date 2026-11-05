//! Browser tests in Chromium: the plugin conformance check (axe, console,
//! Tab and roving keys) and the grid keyboard.
//!
//! Run them with `cargo test --features system-tests --test browser --
//! --include-ignored`. CI runs them with `TZ=America/New_York`, so the DST
//! cases cross a real DST change.
#![cfg(feature = "system-tests")]

use std::time::Duration;

use autumn_datepicker_plugin::{
    DatePicker, DateRange, DateRangePicker, DateTimePicker, TimePicker, date_picker,
    date_range_picker, datetime_picker, pack, time_picker,
};
use autumn_web::form::{Changeset, FieldOptions};
use autumn_web::plugin_conformance::widgets::browser::run_widget_browser_conformance;
use autumn_web::prelude::*;
use autumn_web::system_test::{Page, SystemTest, SystemTestRunner};
use chrono::{NaiveDate, NaiveTime};

const AXE: &str = include_str!("fixtures/axe.min.js");

const fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).expect("a valid test date")
}

#[derive(serde::Serialize)]
struct Form {
    due: NaiveDate,
    end: NaiveDate,
    at: NaiveTime,
    starts_at: chrono::NaiveDateTime,
    stay: Option<DateRange>,
}

#[get("/")]
async fn index() -> Markup {
    let changeset = Changeset::new(Form {
        due: date(2026, 3, 5),
        end: date(2026, 1, 31),
        at: NaiveTime::from_hms_opt(9, 0, 0).expect("t"),
        starts_at: date(2026, 11, 5).and_hms_opt(14, 0, 0).expect("t"),
        stay: None,
    });
    let today = date(2026, 3, 1);
    let picker = DatePicker::new().today(today);
    html! {
        (maud::DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                title { "Datepicker" }
                link rel="stylesheet" href=(autumn_web::ui::WIDGETS_CSS_PATH);
                script src="/static/js/htmx.min.js" defer {}
                script src="/static/js/autumn-widgets.js" defer {}
                (pack().head())
            }
            body {
                main {
                    h1 { "Datepicker" }
                    form {
                        (date_picker(&changeset, "due", "Due", &picker, &FieldOptions::new()))
                        (date_picker(&changeset, "end", "End", &picker, &FieldOptions::new()))
                        (time_picker(&changeset, "at", "At",
                            &TimePicker::new().step_minutes(30), &FieldOptions::new()))
                        (datetime_picker(&changeset, "starts_at", "Starts",
                            &DateTimePicker::new().date(picker.clone()), &FieldOptions::new()))
                        (date_range_picker(&changeset, "stay", "Stay",
                            &DateRangePicker::new().date(picker), &FieldOptions::new()))
                        button type="submit" { "Save" }
                    }
                }
            }
        }
    }
}

async fn app() -> (SystemTestRunner, Page) {
    let runner = Box::pin(
        SystemTest::new()
            .routes(routes![index])
            .routes(autumn_datepicker_plugin::route::routes())
            .plugin_assets(&autumn_datepicker_plugin::ASSETS)
            .build(),
    )
    .await
    .expect("Chromium starts");
    let page = runner.page().await.expect("a page");
    page.visit("/").await.expect("visit");
    wait_for(&page, "window.__autumnWidgets === true && !!window.htmx").await;
    wait_for(
        &page,
        "document.querySelector('.autumn-datepicker-toggle:not([hidden])') !== null",
    )
    .await;
    (runner, page)
}

async fn eval_string(page: &Page, js: &str) -> String {
    page.evaluate(js)
        .await
        .expect("evaluate")
        .into_value::<String>()
        .unwrap_or_default()
}

/// Poll `js` until it is true. Panic after 5 s.
async fn wait_for(page: &Page, js: &str) {
    for _ in 0..100 {
        if page
            .evaluate(js)
            .await
            .ok()
            .and_then(|r| r.into_value::<bool>().ok())
            .unwrap_or(false)
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("timed out waiting for `{js}`");
}

async fn focused_date(page: &Page) -> String {
    eval_string(
        page,
        "document.activeElement && document.activeElement.dataset.date || ''",
    )
    .await
}

async fn wait_focused(page: &Page, expected: &str) {
    wait_for(
        page,
        &format!("document.activeElement && document.activeElement.dataset.date === '{expected}'"),
    )
    .await;
}

async fn open(page: &Page, field: &str) {
    page.evaluate(format!(
        "document.querySelector('[commandfor=\"{field}-dialog\"]').click()"
    ))
    .await
    .expect("click the toggle");
    wait_for(
        page,
        &format!("document.getElementById('{field}-dialog').matches(':popover-open')"),
    )
    .await;
}

#[tokio::test]
#[ignore = "requires Chromium"]
async fn the_pack_passes_widget_browser_conformance() {
    let report = run_widget_browser_conformance(&pack(), AXE)
        .await
        .expect("browser conformance runs");
    assert!(report.passed(), "{}", report.to_text_report());
}

#[tokio::test]
#[ignore = "requires Chromium"]
async fn opening_the_calendar_focuses_the_selected_date() {
    let (_runner, page) = Box::pin(app()).await;
    open(&page, "due").await;
    wait_focused(&page, "2026-03-05").await;
    let expanded = eval_string(
        &page,
        "document.querySelector('[commandfor=\"due-dialog\"]').getAttribute('aria-expanded')",
    )
    .await;
    assert_eq!(expanded, "true");
}

#[tokio::test]
#[ignore = "requires Chromium"]
async fn arrow_keys_move_by_day_and_week_across_dst() {
    let (_runner, page) = Box::pin(app()).await;
    open(&page, "due").await;
    wait_focused(&page, "2026-03-05").await;
    page.press("ArrowRight").await.expect("key");
    assert_eq!(focused_date(&page).await, "2026-03-06");
    page.press("ArrowLeft").await.expect("key");
    assert_eq!(focused_date(&page).await, "2026-03-05");
    // US DST starts on 8 March 2026.
    page.press("ArrowDown").await.expect("key");
    assert_eq!(focused_date(&page).await, "2026-03-12");
    page.press("ArrowDown").await.expect("key");
    assert_eq!(focused_date(&page).await, "2026-03-19");
    page.press("ArrowUp").await.expect("key");
    page.press("ArrowUp").await.expect("key");
    assert_eq!(focused_date(&page).await, "2026-03-05");
    // Exactly one tab stop follows focus.
    let stops = eval_string(
        &page,
        "String(document.querySelectorAll('#due-dialog [data-date][tabindex=\"0\"]').length)",
    )
    .await;
    assert_eq!(stops, "1");
}

#[tokio::test]
#[ignore = "requires Chromium"]
async fn home_and_end_move_to_the_week_bounds() {
    let (_runner, page) = Box::pin(app()).await;
    open(&page, "due").await;
    wait_focused(&page, "2026-03-05").await;
    // The week starts on Sunday: 1 to 7 March 2026.
    page.press("Home").await.expect("key");
    assert_eq!(focused_date(&page).await, "2026-03-01");
    page.press("End").await.expect("key");
    assert_eq!(focused_date(&page).await, "2026-03-07");
}

#[tokio::test]
#[ignore = "requires Chromium"]
async fn page_keys_change_the_month_with_htmx() {
    let (_runner, page) = Box::pin(app()).await;
    open(&page, "end").await;
    wait_focused(&page, "2026-01-31").await;
    page.press("PageDown").await.expect("key");
    // 31 February does not exist: the day clamps.
    wait_focused(&page, "2026-02-28").await;
    let heading = eval_string(
        &page,
        "document.getElementById('end-calendar-heading').textContent",
    )
    .await;
    assert_eq!(heading, "February 2026");
    page.press("PageUp").await.expect("key");
    wait_focused(&page, "2026-01-28").await;
    // Shift+PageDown moves one year.
    page.evaluate(
        "document.activeElement.dispatchEvent(new KeyboardEvent('keydown', \
         { key: 'PageDown', shiftKey: true, bubbles: true, cancelable: true }))",
    )
    .await
    .expect("key");
    wait_focused(&page, "2027-01-28").await;
}

#[tokio::test]
#[ignore = "requires Chromium"]
async fn an_arrow_at_the_month_edge_loads_the_next_month() {
    let (_runner, page) = Box::pin(app()).await;
    open(&page, "end").await;
    wait_focused(&page, "2026-01-31").await;
    page.press("ArrowRight").await.expect("key");
    wait_focused(&page, "2026-02-01").await;
    page.press("ArrowLeft").await.expect("key");
    wait_focused(&page, "2026-01-31").await;
}

#[tokio::test]
#[ignore = "requires Chromium"]
async fn the_buttons_change_the_month() {
    let (_runner, page) = Box::pin(app()).await;
    open(&page, "due").await;
    page.evaluate("document.querySelector('#due-dialog [data-nav=\"next\"]').click()")
        .await
        .expect("click");
    wait_for(
        &page,
        "document.getElementById('due-calendar-heading').textContent === 'April 2026'",
    )
    .await;
}

#[tokio::test]
#[ignore = "requires Chromium"]
async fn enter_picks_the_date_and_closes_the_popover() {
    let (_runner, page) = Box::pin(app()).await;
    open(&page, "due").await;
    wait_focused(&page, "2026-03-05").await;
    page.press("ArrowRight").await.expect("key");
    page.press("Enter").await.expect("key");
    wait_for(
        &page,
        "document.getElementById('due').value === '2026-03-06'",
    )
    .await;
    wait_for(
        &page,
        "!document.getElementById('due-dialog').matches(':popover-open')",
    )
    .await;
    let focus = eval_string(
        &page,
        "document.activeElement.getAttribute('commandfor') || ''",
    )
    .await;
    assert_eq!(focus, "due-dialog", "focus returns to the toggle");
}

#[tokio::test]
#[ignore = "requires Chromium"]
async fn escape_closes_the_popover() {
    let (_runner, page) = Box::pin(app()).await;
    open(&page, "due").await;
    wait_focused(&page, "2026-03-05").await;
    page.press("Escape").await.expect("key");
    wait_for(
        &page,
        "!document.getElementById('due-dialog').matches(':popover-open')",
    )
    .await;
    assert_eq!(
        eval_string(&page, "document.getElementById('due').value").await,
        "2026-03-05"
    );
    wait_for(
        &page,
        "document.activeElement.getAttribute('commandfor') === 'due-dialog'",
    )
    .await;
}

#[tokio::test]
#[ignore = "requires Chromium"]
async fn a_typed_value_shows_its_month_on_open() {
    let (_runner, page) = Box::pin(app()).await;
    page.evaluate("document.getElementById('due').value = '2026-07-14'")
        .await
        .expect("set");
    open(&page, "due").await;
    wait_focused(&page, "2026-07-14").await;
}

#[tokio::test]
#[ignore = "requires Chromium"]
async fn the_time_list_uses_the_keyboard_core() {
    let (_runner, page) = Box::pin(app()).await;
    open(&page, "at-times").await;
    wait_for(&page, "document.activeElement.dataset.time === '09:00'").await;
    page.press("ArrowDown").await.expect("key");
    wait_for(&page, "document.activeElement.dataset.time === '09:30'").await;
    page.press("Enter").await.expect("key");
    wait_for(&page, "document.getElementById('at').value === '09:30'").await;
    wait_for(
        &page,
        "!document.getElementById('at-times-dialog').matches(':popover-open')",
    )
    .await;
}

#[tokio::test]
#[ignore = "requires Chromium"]
async fn the_datetime_picker_sets_each_part() {
    let (_runner, page) = Box::pin(app()).await;
    open(&page, "starts_at").await;
    wait_focused(&page, "2026-11-05").await;
    page.press("ArrowRight").await.expect("key");
    page.press("Enter").await.expect("key");
    wait_for(
        &page,
        "document.getElementById('starts_at').value === '2026-11-06T14:00'",
    )
    .await;
    open(&page, "starts_at-times").await;
    page.press("ArrowDown").await.expect("key");
    page.press("Enter").await.expect("key");
    wait_for(
        &page,
        "document.getElementById('starts_at').value === '2026-11-06T14:30'",
    )
    .await;
}

#[tokio::test]
#[ignore = "requires Chromium"]
async fn two_clicks_pick_a_range() {
    let (_runner, page) = Box::pin(app()).await;
    open(&page, "stay").await;
    page.evaluate("document.querySelector('#stay-dialog [data-date=\"2026-03-12\"]').click()")
        .await
        .expect("click");
    wait_for(
        &page,
        "document.getElementById('stay-start').value === '2026-03-12'",
    )
    .await;
    assert!(
        eval_string(
            &page,
            "String(document.getElementById('stay-dialog').matches(':popover-open'))"
        )
        .await
            == "true",
        "the popover stays open for the end"
    );
    page.evaluate("document.querySelector('#stay-dialog [data-date=\"2026-03-09\"]').click()")
        .await
        .expect("click");
    wait_for(
        &page,
        "document.getElementById('stay-start').value === '2026-03-09'",
    )
    .await;
    wait_for(
        &page,
        "document.getElementById('stay-end').value === '2026-03-12'",
    )
    .await;
    let selected = eval_string(
        &page,
        "String(document.querySelectorAll('#stay-dialog [aria-selected=\"true\"]').length)",
    )
    .await;
    assert_eq!(selected, "4");
}

#[tokio::test]
#[ignore = "requires Chromium"]
async fn with_javascript_off_the_native_input_is_alone() {
    let runner = Box::pin(
        SystemTest::new()
            .routes(routes![index])
            .routes(autumn_datepicker_plugin::route::routes())
            .plugin_assets(&autumn_datepicker_plugin::ASSETS)
            .build(),
    )
    .await
    .expect("Chromium starts");
    let page = runner.page().await.expect("a page");
    page.set_javascript_enabled(false).await.expect("JS off");
    page.visit("/").await.expect("visit");
    let visible = eval_string(
        &page,
        "String([...document.querySelectorAll('.autumn-datepicker-toggle')]\
         .some(b => b.checkVisibility()))",
    )
    .await;
    // `evaluate` runs in the DevTools world, which still works with page JS off.
    assert_eq!(visible, "false");
    let input = eval_string(&page, "document.getElementById('due').type").await;
    assert_eq!(input, "date");
}
