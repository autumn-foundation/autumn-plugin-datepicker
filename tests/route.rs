//! The month route renders one calendar from its query string.

use autumn_datepicker_plugin::route::month_markup;
use autumn_datepicker_plugin::{CalendarLocale, DatepickerPlugin, MONTH_PATH};
use autumn_web::test::TestApp;

fn render(query: &str) -> String {
    month_markup(query, &CalendarLocale::english())
        .expect("a valid query")
        .into_string()
}

#[test]
fn the_route_renders_the_asked_month() {
    let html = render("dp-id=due&dp-mode=date&dp-ws=7&dp-month=2026-12");
    assert!(html.contains(">December 2026<"), "{html}");
    assert!(html.contains(r#"data-date="2026-12-31""#), "{html}");
    assert!(html.contains(r#"id="due-calendar-heading""#), "{html}");
}

#[test]
fn the_field_value_marks_the_selection() {
    let html = render("dp-id=due&dp-mode=date&dp-ws=1&dp-month=2026-11&due=2026-11-05");
    assert!(
        html.contains(r#"data-date="2026-11-05" tabindex="0" aria-selected="true""#),
        "{html}"
    );
    // A Monday start: 1 November 2026 is a Sunday, in the last column.
    let mo = html.find(r#"abbr="Monday""#).expect("Monday");
    let su = html.find(r#"abbr="Sunday""#).expect("Sunday");
    assert!(mo < su, "{html}");
}

#[test]
fn a_field_named_like_a_route_key_still_works() {
    let html = render("dp-id=month&dp-mode=date&dp-ws=7&dp-month=2026-11&month=2026-11-05");
    assert!(html.contains(">November 2026<"), "{html}");
    assert!(
        html.contains(r#"data-date="2026-11-05" tabindex="0" aria-selected="true""#),
        "{html}"
    );
}

#[test]
fn a_datetime_value_marks_its_date() {
    let html = render("dp-id=at&dp-mode=datetime&dp-ws=7&dp-month=2026-11&at=2026-11-05T09:30");
    assert!(
        html.contains(r#"data-date="2026-11-05" tabindex="0" aria-selected="true""#),
        "{html}"
    );
}

#[test]
fn a_range_value_marks_the_range() {
    let html =
        render("dp-id=stay&dp-mode=range&dp-ws=7&dp-month=2026-11&stay=2026-11-07&stay=2026-11-05");
    assert_eq!(html.matches(r#"aria-selected="true""#).count(), 3, "{html}");
    let start_only =
        render("dp-id=stay&dp-mode=range&dp-ws=7&dp-month=2026-11&stay=2026-11-05&stay=");
    assert_eq!(
        start_only.matches(r#"aria-selected="true""#).count(),
        1,
        "{start_only}"
    );
}

#[test]
fn the_rules_come_from_the_query() {
    let html = render(
        "dp-id=due&dp-mode=date&dp-ws=7&dp-month=2026-11&dp-min=2026-11-10&dp-off=67&dp-x=2026-11-12",
    );
    assert!(
        html.contains(r#"data-date="2026-11-09" tabindex="-1" aria-disabled="true""#),
        "{html}"
    );
    assert!(
        html.contains(r#"data-date="2026-11-12" tabindex="-1" aria-disabled="true""#),
        "{html}"
    );
    assert!(
        html.contains(r#"data-date="2026-11-14" tabindex="-1" aria-disabled="true""#),
        "{html}"
    );
    assert!(
        html.contains(r#"data-date="2026-11-10" tabindex="0""#),
        "{html}"
    );
}

#[test]
fn today_comes_from_the_query() {
    let html = render("dp-id=due&dp-mode=date&dp-ws=7&dp-month=2026-11&dp-today=2026-11-03");
    assert!(
        html.contains(r#"data-date="2026-11-03" tabindex="0" aria-current="date""#),
        "{html}"
    );
}

#[test]
fn a_bad_query_is_an_error() {
    for query in [
        "",
        "dp-mode=date&dp-ws=7&dp-month=2026-11",
        "dp-id=due&dp-mode=date&dp-ws=7",
        "dp-id=due&dp-mode=date&dp-ws=7&dp-month=2026-13",
        "dp-id=due&dp-mode=week&dp-ws=7&dp-month=2026-11",
        "dp-id=due&dp-mode=date&dp-ws=0&dp-month=2026-11",
        "dp-id=due&dp-mode=date&dp-ws=7&dp-month=2026-11&dp-min=never",
        "dp-id=a%22b&dp-mode=date&dp-ws=7&dp-month=2026-11",
        "dp-id=due&dp-mode=date&dp-ws=7&dp-month=2026-11&dp-today=soon",
    ] {
        assert!(
            month_markup(query, &CalendarLocale::english()).is_err(),
            "{query:?}"
        );
    }
}

#[test]
fn a_long_query_is_an_error() {
    let long = format!(
        "dp-id=due&dp-mode=date&dp-ws=7&dp-month=2026-11&due={}",
        "x".repeat(autumn_datepicker_plugin::route::MAX_QUERY_BYTES)
    );
    assert!(month_markup(&long, &CalendarLocale::english()).is_err());
}

#[test]
fn an_unparsable_field_value_selects_nothing() {
    let html = render("dp-id=due&dp-mode=date&dp-ws=7&dp-month=2026-11&due=not-a-date");
    assert!(!html.contains("aria-selected"), "{html}");
}

#[tokio::test]
async fn the_plugin_serves_the_route_and_the_assets() {
    let client = TestApp::new().plugin(DatepickerPlugin::new()).build();
    let ok = client
        .get(&format!(
            "{MONTH_PATH}?dp-id=due&dp-mode=date&dp-ws=7&dp-month=2026-11"
        ))
        .send()
        .await;
    ok.assert_ok();
    ok.assert_body_contains(">November 2026<");
    assert!(
        ok.header("content-type")
            .is_some_and(|v| v.starts_with("text/html")),
        "{:?}",
        ok.header("content-type")
    );

    assert_eq!(ok.header("vary"), Some("Accept-Language, Cookie"));

    let bad = client.get(&format!("{MONTH_PATH}?dp-id=due")).send().await;
    bad.assert_status(400);
    assert!(
        !bad.text().contains("due"),
        "the error does not echo input: {}",
        bad.text()
    );

    for path in autumn_datepicker_plugin::pack()
        .stylesheet_urls()
        .into_iter()
        .chain(autumn_datepicker_plugin::pack().script_urls())
    {
        client.get(&path).send().await.assert_ok();
    }
}

#[test]
fn the_plugin_declares_its_route_and_contract() {
    use autumn_web::plugin::Plugin;
    let plugin = DatepickerPlugin::new();
    let contract = plugin.contract().expect("a contract");
    assert_eq!(contract.plugin, "autumn-datepicker-plugin");
    let routes = autumn_web::app()
        .plugin(DatepickerPlugin::new())
        .plugin_route_infos()
        .expect("routes");
    assert!(
        routes
            .iter()
            .any(|r| r.path == MONTH_PATH && r.method == "GET"),
        "{routes:?}"
    );
}

#[cfg(feature = "i18n")]
mod i18n {
    use super::*;
    use autumn_web::i18n::{Bundle, I18nConfig, Locale};
    use std::collections::HashMap;
    use std::sync::Arc;

    #[test]
    fn the_route_text_comes_from_the_request_locale() {
        let config = I18nConfig {
            supported_locales: vec!["en".into(), "de".into()],
            ..I18nConfig::default()
        };
        let mut messages = HashMap::new();
        messages.insert("en".to_owned(), HashMap::new());
        messages.insert(
            "de".to_owned(),
            HashMap::from([
                ("datepicker-month-11".to_owned(), "November".to_owned()),
                (
                    "datepicker-next-month".to_owned(),
                    "Nächster Monat".to_owned(),
                ),
            ]),
        );
        let bundle = Arc::new(Bundle::from_messages(messages, &config));
        let locale = CalendarLocale::from_i18n(&Locale::new("de").with_bundle(bundle));
        let html = month_markup("dp-id=due&dp-mode=date&dp-ws=1&dp-month=2026-11", &locale)
            .expect("query")
            .into_string();
        assert!(html.contains(r#"aria-label="Nächster Monat""#), "{html}");
    }
}
