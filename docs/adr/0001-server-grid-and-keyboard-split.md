# ADR 0001: The server renders the grid, and the pack script adds the 2-D keys

- Status: accepted
- Date: 2026-11-05
- Issue: autumn-foundation/autumn#3413

## Context

The widget contract says that pack JavaScript uses the keyboard core and
has no arrow-key code of its own. The core is 1-D: a roving list moves with
ArrowLeft/ArrowRight or ArrowUp/ArrowDown, and Home/End go to the first and
the last item. A date grid needs 2-D keys: a week up or down, a month with
PageUp/PageDown, the week bounds with Home/End, and the next month at the
edge.

The grid can also come from the server or from JavaScript.

## Decision

1. The server renders each month. A month change is one htmx `GET`. The
   route reads all state from the query string.
2. The grid uses the core for the roving tab stop, ArrowLeft/ArrowRight,
   focus tracking and `aria-expanded`.
3. This is an exception to the contract rule "no arrow-key code". The
   core has no grid mode, so `datepicker.js` handles only ArrowUp/ArrowDown,
   PageUp/PageDown, Shift+PageUp/PageDown, Home/End and the month edge. It
   listens in the capture phase and calls `preventDefault`, so the core
   skips those keys.
4. The script does date math in UTC only.

## Consequences

- One renderer. The server tests cover the grid. The script uses UTC, so a
  DST change cannot move a day.
- The pack script has key code. A future core grid mode
  (`data-autumn-orientation="grid"`) can replace it. That is a change in
  `autumn-web`, not in this crate.
- A month change needs the network. The route is small and has no state,
  so it can be cached (`Cache-Control: private, max-age=300`).

## Rejected

- Render each month in the page: the HTML grows with each month.
- Build the grid in JavaScript: two renderers, and local-time DST bugs.
- Keep picker state on the server: the query string is enough.
