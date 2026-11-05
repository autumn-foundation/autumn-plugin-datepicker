//! The pack obeys the widget contract: the static half of the plugin
//! conformance check. The browser half is in `tests/browser.rs`.

use autumn_datepicker_plugin::{ASSETS, DatepickerPlugin, pack};
use autumn_web::plugin_conformance::widgets::run_widget_conformance;
use autumn_web::stories::Story;

#[test]
fn the_pack_passes_widget_conformance() {
    let report = run_widget_conformance(&pack());
    assert!(report.passed(), "{}", report.to_text_report());
}

#[test]
fn each_picker_has_a_story() {
    let p = pack();
    let slugs: Vec<&str> = p.stories().iter().map(Story::slug).collect();
    for slug in [
        "datepicker-date-picker",
        "datepicker-inline-calendar",
        "datepicker-limits-and-disabled-days",
        "datepicker-week-start-and-names",
        "datepicker-time-picker",
        "datepicker-date-time-picker",
        "datepicker-date-range-picker",
    ] {
        assert!(slugs.contains(&slug), "{slug} is missing: {slugs:?}");
    }
}

#[test]
fn the_pack_ships_its_css_and_js() {
    let p = pack();
    assert_eq!(p.name(), "datepicker");
    assert!(std::ptr::eq(
        p.asset_bundle().expect("assets"),
        std::ptr::from_ref(&ASSETS)
    ));
    assert_eq!(p.stylesheet_urls().len(), 1);
    assert_eq!(p.script_urls().len(), 1);
    let head = p.head().into_string();
    assert!(head.contains("integrity="), "{head}");
}

#[test]
fn the_script_uses_autumn_init_not_htmx_events() {
    let js = include_str!("../assets/datepicker.js");
    assert!(js.contains("autumn:init"), "set-up runs on autumn:init");
    assert!(
        !js.contains("htmx:afterSwap"),
        "the runtime fires autumn:init"
    );
    assert!(!js.contains("innerHTML"), "the server renders the markup");
    assert!(js.contains("setUTCFullYear"), "date math is in UTC");
    for local in [
        "getDate()",
        "getMonth()",
        "getFullYear()",
        "setHours",
        "setDate(",
        "Date.parse",
    ] {
        assert!(!js.contains(local), "no local-time math: {local}");
    }
}

#[test]
fn mounting_the_plugin_adds_the_stories_and_serves_the_assets() {
    let builder = autumn_web::app()
        .with_story_gallery(autumn_web::stories::StoryGallery::new())
        .plugin(DatepickerPlugin::new());
    let routes = builder.plugin_route_infos().expect("routes");
    for file in ["datepicker.css", "datepicker.js"] {
        assert!(
            routes
                .iter()
                .any(|r| r.path == format!("/static/_plugins/datepicker/{file}")),
            "{file} is not served: {routes:?}"
        );
    }
}
