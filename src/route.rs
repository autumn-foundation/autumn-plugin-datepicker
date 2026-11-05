//! The month route: `GET /_datepicker/month` returns one calendar.
//!
//! The query holds all the state: the field id, the month, the mode, the
//! week start, today and the rules, each under a `dp-` key. `hx-include`
//! adds the current input values under the field name. The route needs no
//! server state.

use std::sync::Arc;

use autumn_web::reexports::axum::extract::{RawQuery, State};
use autumn_web::reexports::axum::response::{IntoResponse, Response};
use autumn_web::reexports::http::{StatusCode, header};
use autumn_web::{AppState, Route};
use maud::Markup;

use crate::calendar::{Calendar, Mode, QUERY_PREFIX, Selection, is_valid_id};
use crate::grid::MonthGrid;
use crate::locale::CalendarLocale;
use crate::rules::{DateRules, parse_iso_date, weekday_from_iso};
use crate::value::{parse_date, parse_datetime};

/// The longest query the route reads, in bytes. A longer query gets 400.
pub const MAX_QUERY_BYTES: usize = 16 * 1024;

/// The error for a query that does not parse. The route answers 400. The
/// message names the key, not the value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryError(String);

impl QueryError {
    /// The message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for QueryError {}

fn bad(message: impl Into<String>) -> QueryError {
    QueryError(message.into())
}

/// Render the calendar that `query` (a raw query string) asks for.
///
/// # Errors
///
/// [`QueryError`] when a key is missing or a value does not parse.
pub fn month_markup(query: &str, locale: &CalendarLocale) -> Result<Markup, QueryError> {
    if query.len() > MAX_QUERY_BYTES {
        return Err(bad("the query is too long"));
    }
    let pairs: Vec<(String, String)> =
        serde_html_form::from_str(query).map_err(|e| bad(format!("query: {e}")))?;
    let get = |key: &str| {
        pairs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    };
    let id = get("dp-id")
        .filter(|id| is_valid_id(id))
        .ok_or_else(|| bad("`dp-id` is missing or not valid"))?;
    let mode = get("dp-mode")
        .and_then(Mode::parse)
        .ok_or_else(|| bad("`dp-mode` is not date, datetime or range"))?;
    let week_start = get("dp-ws")
        .and_then(|ws| ws.parse().ok())
        .and_then(weekday_from_iso)
        .ok_or_else(|| bad("`dp-ws` is not an ISO weekday digit 1-7"))?;
    let month = get("dp-month")
        .and_then(|m| MonthGrid::parse_key(m, week_start))
        .ok_or_else(|| bad("`dp-month` is not YYYY-MM"))?;
    let today = get("dp-today")
        .filter(|t| !t.is_empty())
        .map(|t| parse_iso_date(t).ok_or_else(|| bad("`dp-today` is not YYYY-MM-DD")))
        .transpose()?;
    let rules = DateRules::from_query(|key| get(&format!("{QUERY_PREFIX}{key}")))
        .map_err(|e| bad(format!("{QUERY_PREFIX}{e}")))?;

    // `hx-include` sends the input values under the field name.
    let values: Vec<&str> = pairs
        .iter()
        .filter(|(k, _)| k == id)
        .map(|(_, v)| v.as_str())
        .collect();
    let first = values.first().copied().unwrap_or_default();
    let selection = match mode {
        Mode::Date => parse_date(first).map_or(Selection::Empty, Selection::Day),
        Mode::DateTime => {
            parse_datetime(first, None).map_or(Selection::Empty, |dt| Selection::Day(dt.date()))
        }
        Mode::Range => Selection::range(
            parse_date(first),
            values.get(1).copied().and_then(parse_date),
        ),
    };

    let mut calendar = Calendar::new(id, month, &rules, locale)
        .selection(selection)
        .mode(mode);
    if let Some(today) = today {
        calendar = calendar.today(today);
    }
    Ok(calendar.render())
}

/// The text of the month route when the request has no i18n bundle. The
/// plugin puts it in the app state.
#[derive(Debug, Clone, Default)]
pub(crate) struct RouteLocale(pub(crate) CalendarLocale);

fn plugin_locale(state: &AppState) -> CalendarLocale {
    state
        .extension::<RouteLocale>()
        .map_or_else(CalendarLocale::english, |l: Arc<RouteLocale>| l.0.clone())
}

fn respond(query: Option<&str>, locale: &CalendarLocale) -> Response {
    match month_markup(query.unwrap_or_default(), locale) {
        // With `i18n`, the text follows the request locale: the header or a cookie.
        Ok(markup) => (
            [
                (header::CACHE_CONTROL, "private, max-age=300"),
                (header::VARY, "Accept-Language, Cookie"),
            ],
            markup,
        )
            .into_response(),
        Err(err) => (StatusCode::BAD_REQUEST, err.0).into_response(),
    }
}

/// With a request bundle, the route uses its text. Without one, the route
/// uses the plugin text.
#[cfg(feature = "i18n")]
#[autumn_web::get("/_datepicker/month")]
async fn month(
    State(state): State<AppState>,
    locale: autumn_web::i18n::Locale,
    RawQuery(query): RawQuery,
) -> Response {
    let text = if locale.bundle().is_some() {
        CalendarLocale::from_i18n(&locale)
    } else {
        plugin_locale(&state)
    };
    respond(query.as_deref(), &text)
}

#[cfg(not(feature = "i18n"))]
#[autumn_web::get("/_datepicker/month")]
async fn month(State(state): State<AppState>, RawQuery(query): RawQuery) -> Response {
    respond(query.as_deref(), &plugin_locale(&state))
}

/// The month route, for `AppBuilder::routes` or `SystemTest::routes`.
#[must_use]
pub fn routes() -> Vec<Route> {
    autumn_web::routes![month]
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_route_path_is_month_path() {
        let routes = super::routes();
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].path, crate::calendar::MONTH_PATH);
    }
}
