//! Date, time, date-time and date-range pickers for Autumn.
//!
//! Each picker is a native input. With JavaScript, a toggle button opens a
//! calendar or a time list in a native popover. The server renders the
//! month grid. A month change is one htmx request to [`MONTH_PATH`].
//!
//! ```rust,no_run
//! use autumn_datepicker_plugin::{DatePicker, DatepickerPlugin, date_picker};
//! use autumn_web::form::{Changeset, FieldOptions};
//!
//! #[derive(serde::Serialize)]
//! struct Task {
//!     due: chrono::NaiveDate,
//! }
//!
//! let changeset = Changeset::new(Task {
//!     due: chrono::NaiveDate::from_ymd_opt(2026, 11, 5).unwrap(),
//! });
//! let html = date_picker(&changeset, "due", "Due", &DatePicker::new(), &FieldOptions::new());
//!
//! // In `main`: `autumn_web::app().plugin(DatepickerPlugin::new())`.
//! # let _ = html;
//! ```
//!
//! See the [guide](https://github.com/autumn-foundation/autumn-plugin-datepicker/blob/main/docs/guide/datepicker.md).

use std::borrow::Cow;

use autumn_web::app::AppBuilder;
use autumn_web::plugin::Plugin;
use autumn_web::plugin_contract::PluginContract;

pub mod calendar;
pub mod grid;
pub mod locale;
pub mod pack;
pub mod route;
pub mod rules;
pub mod value;
pub mod widgets;

pub use calendar::{Calendar, MONTH_PATH, Mode, Selection};
pub use grid::{MonthGrid, days_in_month};
pub use locale::{CalendarLocale, Label};
pub use pack::{ASSETS, pack};
pub use rules::{DateRules, MAX_DISABLED_DATES};
pub use value::DateRange;
pub use widgets::{
    DatePicker, DateRangePicker, DateTimePicker, TimePicker, date_picker, date_range_picker,
    datetime_picker, time_picker,
};

/// The `autumn-web` range this plugin supports.
pub const AUTUMN_WEB_RANGE: &str = "0.10";

/// Installs the widget pack and the month route.
#[derive(Debug, Clone, Default)]
pub struct DatepickerPlugin {
    locale: CalendarLocale,
}

impl DatepickerPlugin {
    /// A plugin with English text for the month route.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The text of the month route when the request has no i18n bundle.
    #[must_use]
    pub fn locale(mut self, locale: CalendarLocale) -> Self {
        self.locale = locale;
        self
    }
}

impl Plugin for DatepickerPlugin {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed(env!("CARGO_PKG_NAME"))
    }

    fn contract(&self) -> Option<PluginContract> {
        Some(
            PluginContract::new(env!("CARGO_PKG_NAME"))
                .plugin_version(env!("CARGO_PKG_VERSION"))
                .autumn_web(AUTUMN_WEB_RANGE),
        )
    }

    fn build(self, app: AppBuilder) -> AppBuilder {
        app.with_extension(route::RouteLocale(self.locale))
            .widget_pack(pack())
            .routes(route::routes())
    }
}
