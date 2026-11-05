// autumn-datepicker-plugin. Load it with `defer`, after autumn-widgets.js.
//
// The keyboard core in autumn-widgets.js gives the roving tab stop,
// ArrowLeft/ArrowRight in the grid, the time list keys and aria-expanded.
// The core has no 2-D grid mode, so this file adds only the grid keys that
// the core does not have: ArrowUp/ArrowDown (one week), PageUp/PageDown (one
// month), Shift+PageUp/PageDown (one year), Home/End (week start and end),
// and the move to the next month at the month edge. It also writes the
// input values. Date math uses UTC only, so a DST change cannot move a day.
(function () {
  'use strict';

  // Install one time, even when the script loads two times.
  if (window.__autumnDatepicker) return;
  window.__autumnDatepicker = true;

  var DAY = 86400000;
  var FIELD = '[data-autumn-datepicker]';
  var CALENDAR = '[data-autumn-datepicker-calendar]';
  var CELL = CALENDAR + ' [data-date]';
  var DIALOG = '.autumn-datepicker-dialog';

  var ready = new WeakSet();
  var pending = new WeakMap(); // field -> { date, step, reload } or { nav }, after a swap
  var loading = new WeakMap(); // field -> the calendar that an open request replaces
  var shownMonth = new WeakMap(); // field -> the month on screen, for the live region
  var rangeStart = new WeakMap(); // field -> start date, while the user picks the end

  // ── Dates (`YYYY-MM-DD` strings, UTC milliseconds) ──────────────────────

  function parse(iso) {
    var m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(iso || '');
    if (!m) return NaN;
    var d = new Date(0);
    d.setUTCFullYear(+m[1], +m[2] - 1, +m[3]);
    return d.getTime();
  }

  function pad(n, width) {
    var s = String(n);
    while (s.length < width) s = '0' + s;
    return s;
  }

  function format(t) {
    var d = new Date(t);
    return pad(d.getUTCFullYear(), 4) + '-' + pad(d.getUTCMonth() + 1, 2) + '-' +
      pad(d.getUTCDate(), 2);
  }

  function addDays(iso, n) {
    return format(parse(iso) + n * DAY);
  }

  // The same day in the month `n` months away, clamped to its length.
  function addMonths(iso, n) {
    var d = new Date(parse(iso));
    var day = d.getUTCDate();
    var first = new Date(0);
    first.setUTCFullYear(d.getUTCFullYear(), d.getUTCMonth() + n, 1);
    var last = new Date(first.getTime());
    last.setUTCFullYear(first.getUTCFullYear(), first.getUTCMonth() + 1, 0);
    return format(first.getTime() + (Math.min(day, last.getUTCDate()) - 1) * DAY);
  }

  function isDate(value) {
    return !isNaN(parse(value));
  }

  // ── The DOM ──────────────────────────────────────────────────────────────

  function fieldOf(el) {
    return el && el.closest ? el.closest(FIELD) : null;
  }

  function inputs(field) {
    return field.querySelectorAll('input.autumn-datepicker-input');
  }

  function cellOf(cal, iso) {
    return cal.querySelector('[data-date="' + iso + '"]');
  }

  function enabled(td) {
    return !!td && td.getAttribute('aria-disabled') !== 'true';
  }

  function clamp(cal, iso) {
    var min = cal.getAttribute('data-min');
    var max = cal.getAttribute('data-max');
    if (min && iso < min) return min;
    if (max && iso > max) return max;
    return iso;
  }

  function popoverOpen(el) {
    try { return el.matches(':popover-open'); } catch (_) { return false; }
  }

  function toggleOf(dialog) {
    var id = dialog.id && CSS.escape(dialog.id);
    return id ? document.querySelector('[aria-controls~="' + id + '"]') : null;
  }

  function setValue(input, value) {
    if (!input || input.value === value) return;
    input.value = value;
    input.dispatchEvent(new Event('input', { bubbles: true }));
    input.dispatchEvent(new Event('change', { bubbles: true }));
  }

  // The date that the inputs hold: the start of a range, the date part of a
  // date-time value.
  function wantedDate(field) {
    var value = (inputs(field)[0] || {}).value || '';
    var day = value.slice(0, 10);
    return isDate(day) ? day : null;
  }

  // The values of a range: [start, end]. A lone end acts as the start.
  function rangeValues(field) {
    var list = inputs(field);
    var a = list[0] && isDate(list[0].value) ? list[0].value : '';
    var b = list[1] && isDate(list[1].value) ? list[1].value : '';
    if (!a) return [b, b];
    return [a, b || a];
  }

  // Mark the days from `start` to `end` as selected.
  function mark(cal, start, end) {
    var range = cal.getAttribute('data-mode') === 'range';
    cal.querySelectorAll('[data-date]').forEach(function (td) {
      var iso = td.getAttribute('data-date');
      var on = !!start && iso >= start && iso <= end;
      if (on) td.setAttribute('aria-selected', 'true');
      else td.removeAttribute('aria-selected');
      if (!range) return;
      td.classList.toggle('autumn-datepicker-day--range-start', on && iso === start);
      td.classList.toggle('autumn-datepicker-day--range-end', on && iso === end);
      td.classList.toggle('autumn-datepicker-day--in-range', on && iso !== start && iso !== end);
    });
  }

  // ── Month changes (htmx) ─────────────────────────────────────────────────

  // Load the month of `iso`. Return false when htmx is not on the page.
  function load(field, cal, iso, focus) {
    if (!window.htmx || !cal.isConnected) return false;
    if (focus) pending.set(field, focus);
    else pending.delete(field);
    loading.set(field, cal);
    var params = new URLSearchParams();
    inputs(field).forEach(function (input) {
      if (input.name) params.append(input.name, input.value);
    });
    var url = cal.getAttribute('data-src') + '&dp-month=' + iso.slice(0, 7) + '&' + params;
    var done = function () {
      // htmx does not swap an error response. Then the old grid stays.
      setTimeout(function () {
        if (!cal.isConnected || loading.get(field) !== cal) return;
        loading.delete(field);
        pending.delete(field);
        focusStop(cal);
      }, 0);
    };
    var request = window.htmx.ajax('GET', url, { target: cal, swap: 'outerHTML' });
    if (request && request.then) request.then(done, done);
    return true;
  }

  // Focus `iso`. A disabled day moves on by `step` days. A day outside the
  // grid loads its month when `mayLoad` is true.
  function focusDate(field, cal, iso, step, mayLoad) {
    var target = clamp(cal, iso);
    for (var i = 0; i < 400; i++) {
      var td = cellOf(cal, target);
      if (!td) return mayLoad && load(field, cal, target, { date: target, step: step });
      if (enabled(td)) {
        td.focus();
        return true;
      }
      if (!step) return false;
      var next = clamp(cal, addDays(target, step));
      if (next === target) return false;
      target = next;
    }
    return false;
  }

  // The day that holds the tab stop: the first selected day, else today,
  // else the first enabled day.
  function stopOf(cal) {
    var open = '[data-date]:not([aria-disabled="true"])';
    return cal.querySelector(open + '[aria-selected="true"]') ||
      cal.querySelector(open + '[aria-current="date"]') || cal.querySelector(open);
  }

  function focusStop(cal) {
    var stop = stopOf(cal);
    if (stop) stop.focus();
  }

  // The keyboard core puts the tab stop on the first selected or current
  // day in DOM order. Put it on the selected day, as the server does.
  function setStop(cal) {
    var stop = stopOf(cal);
    if (!stop) return;
    cal.querySelectorAll('[data-date]').forEach(function (td) {
      td.tabIndex = td === stop ? 0 : -1;
    });
  }

  // Say the new month in the live region, which htmx does not replace.
  function announce(field, cal) {
    var month = cal.getAttribute('data-month');
    var before = shownMonth.get(field);
    shownMonth.set(field, month);
    var status = field.querySelector('.autumn-datepicker-status');
    var heading = cal.querySelector('.autumn-datepicker-heading');
    if (before && before !== month && status && heading) status.textContent = heading.textContent;
  }

  // A month button needs htmx. Without it, the button does nothing.
  function disableNavWithoutHtmx(cal) {
    if (window.htmx) return;
    cal.querySelectorAll('[data-nav]').forEach(function (button) { button.disabled = true; });
  }

  // A new calendar (page load or swap): mark a half-picked range, fix the
  // tab stop and focus what the key or the button asked for.
  function restore(field, cal) {
    if (loading.has(field) && !loading.get(field).isConnected) loading.delete(field);
    var start = rangeStart.get(field);
    if (start && cal.getAttribute('data-mode') === 'range') mark(cal, start, start);
    disableNavWithoutHtmx(cal);
    announce(field, cal);
    var want = pending.get(field);
    pending.delete(field);
    // The keyboard core sets the tab stop after `autumn:init`. Act after it.
    setTimeout(function () {
      setStop(cal);
      if (!want) return;
      if (want.nav) {
        var button = cal.querySelector('[data-nav="' + CSS.escape(want.nav) + '"]');
        if (button && !button.disabled) button.focus();
        else focusStop(cal);
        return;
      }
      if (!focusDate(field, cal, want.date, want.step, !!want.reload)) focusStop(cal);
    }, 0);
  }

  // ── Picking ──────────────────────────────────────────────────────────────

  function close(field, dialog, focus) {
    if (!dialog) {
      if (focus) focus.focus();
      return;
    }
    var toggle = toggleOf(dialog);
    if (toggle) toggle.focus();
    if (popoverOpen(dialog)) dialog.hidePopover();
  }

  function pickDate(field, td) {
    if (!enabled(td)) return;
    var cal = td.closest(CALENDAR);
    var iso = td.getAttribute('data-date');
    var list = inputs(field);
    var dialog = td.closest(DIALOG);
    var mode = cal.getAttribute('data-mode');
    if (mode === 'range') {
      var start = rangeStart.get(field);
      if (!start) {
        // Mark the start only. Escape then keeps the old range.
        rangeStart.set(field, iso);
        mark(cal, iso, iso);
        td.focus();
        return;
      }
      rangeStart.delete(field);
      var a = start < iso ? start : iso;
      var b = start < iso ? iso : start;
      setValue(list[0], a);
      setValue(list[1], b);
      mark(cal, a, b);
    } else if (mode === 'datetime') {
      var time = (list[0].value.split('T')[1] || '').slice(0, 12) || '00:00';
      setValue(list[0], iso + 'T' + time);
      mark(cal, iso, iso);
    } else {
      setValue(list[0], iso);
      mark(cal, iso, iso);
    }
    close(field, dialog, td);
  }

  function pickTime(field, option) {
    var list = option.closest('[role="listbox"]');
    var input = inputs(field)[0];
    var time = option.getAttribute('data-time');
    if (field.getAttribute('data-autumn-datepicker') === 'datetime') {
      var cal = field.querySelector(CALENDAR);
      var day = wantedDate(field) || (cal && cal.getAttribute('data-today')) ||
        format(Date.now());
      setValue(input, day + 'T' + time);
    } else {
      setValue(input, time);
    }
    list.querySelectorAll('[role="option"]').forEach(function (o) {
      o.setAttribute('aria-selected', o === option ? 'true' : 'false');
    });
    close(field, list, option);
  }

  // ── Opening ──────────────────────────────────────────────────────────────

  function openCalendar(field, cal) {
    rangeStart.delete(field);
    disableNavWithoutHtmx(cal);
    var wanted = wantedDate(field);
    var range = cal.getAttribute('data-mode') === 'range';
    if (range) wanted = rangeValues(field)[0] || null;
    if (wanted && wanted.slice(0, 7) !== cal.getAttribute('data-month') &&
        load(field, cal, wanted, { date: wanted, step: 0 })) {
      return;
    }
    if (range) {
      var values = rangeValues(field);
      mark(cal, values[0], values[1]);
    } else if (wanted) {
      mark(cal, wanted, wanted);
    }
    setStop(cal);
    if (!(wanted && focusDate(field, cal, wanted, 0, false))) focusStop(cal);
  }

  function openTimes(field, list) {
    var input = inputs(field)[0];
    var value = input ? input.value : '';
    var time = value.indexOf('T') >= 0 ? value.split('T')[1] : value;
    var options = Array.prototype.slice.call(list.querySelectorAll('[role="option"]'));
    var pick = null;
    options.forEach(function (o) {
      var t = o.getAttribute('data-time');
      o.setAttribute('aria-selected', t === time ? 'true' : 'false');
      if (!pick && time && t >= time) pick = o;
    });
    pick = pick || list.querySelector('[role="option"][tabindex="0"]') || options[0];
    if (pick) pick.focus();
  }

  // ── Set-up ───────────────────────────────────────────────────────────────

  function setUp(field) {
    if (ready.has(field)) return;
    ready.add(field);
    field.querySelectorAll('.autumn-datepicker-toggle').forEach(function (toggle) {
      toggle.setAttribute('aria-expanded', 'false');
      toggle.hidden = false;
    });
    field.querySelectorAll('.autumn-datepicker-inline').forEach(function (inline) {
      inline.hidden = false;
    });
  }

  function each(root, selector, fn) {
    if (root.matches && root.matches(selector)) fn(root);
    root.querySelectorAll(selector).forEach(fn);
  }

  document.addEventListener('autumn:init', function (e) {
    var root = e.target;
    if (!root.querySelectorAll) return;
    each(root, FIELD, function (field) {
      setUp(field);
      var cal = field.querySelector(CALENDAR);
      if (cal && !shownMonth.has(field)) shownMonth.set(field, cal.getAttribute('data-month'));
    });
    each(root, CALENDAR, function (cal) {
      var field = fieldOf(cal);
      if (field) restore(field, cal);
    });
  });

  // ── Events ───────────────────────────────────────────────────────────────

  // `toggle` does not bubble, so listen in the capture phase.
  document.addEventListener('toggle', function (e) {
    var dialog = e.target;
    if (e.newState !== 'open' || !dialog.matches || !dialog.matches(DIALOG)) return;
    var field = fieldOf(dialog);
    if (!field) return;
    var cal = dialog.querySelector(CALENDAR);
    if (cal) openCalendar(field, cal);
    else openTimes(field, dialog);
  }, true);

  document.addEventListener('click', function (e) {
    if (!e.target.closest) return;
    var td = e.target.closest(CELL);
    var field = fieldOf(td || e.target);
    if (!field) return;
    if (td) {
      pickDate(field, td);
      return;
    }
    var option = e.target.closest('.autumn-datepicker-time');
    if (option) {
      pickTime(field, option);
      return;
    }
    // htmx sends the request. Keep focus on the button after the swap.
    var nav = e.target.closest(CALENDAR + ' [data-nav]');
    if (nav) pending.set(field, { nav: nav.getAttribute('data-nav') });
  });

  // Capture phase: this runs before the keyboard core, which skips a key
  // that has `defaultPrevented`.
  document.addEventListener('keydown', function (e) {
    var td = e.target;
    if (e.defaultPrevented || e.altKey || e.ctrlKey || e.metaKey || e.isComposing) return;
    if (!td.closest) return;
    // Escape: close, and focus the toggle (APG).
    var open = e.key === 'Escape' && td.closest(DIALOG);
    if (open && popoverOpen(open)) {
      e.preventDefault();
      close(fieldOf(open), open, null);
      return;
    }
    if (!td.matches(CELL)) return;
    var cal = td.closest(CALENDAR);
    var field = fieldOf(cal);
    if (!field) return;
    var iso = td.getAttribute('data-date');
    var target;
    var step;
    var row = td.parentElement.querySelectorAll('[data-date]');
    // A key during a month request moves on from the date it asked for.
    var queued = loading.has(field) && pending.get(field);
    if (queued && queued.date && e.key !== 'Enter' && e.key !== ' ') {
      iso = queued.date;
      if (e.key === 'Home' || e.key === 'End') {
        e.preventDefault();
        return;
      }
    }
    switch (e.key) {
      case 'Enter':
      case ' ':
        e.preventDefault();
        if (!e.repeat) pickDate(field, td);
        return;
      case 'ArrowUp': target = addDays(iso, -7); step = -1; break;
      case 'ArrowDown': target = addDays(iso, 7); step = 1; break;
      // The first and the last day of this week in this month.
      case 'Home': target = row[0].getAttribute('data-date'); step = 1; break;
      case 'End': target = row[row.length - 1].getAttribute('data-date'); step = -1; break;
      case 'PageUp': target = addMonths(iso, e.shiftKey ? -12 : -1); step = -1; break;
      case 'PageDown': target = addMonths(iso, e.shiftKey ? 12 : 1); step = 1; break;
      case 'ArrowLeft':
      case 'ArrowRight': {
        var rtl = getComputedStyle(td.closest('table')).direction === 'rtl';
        var forward = (e.key === 'ArrowRight') !== rtl;
        var days = Array.prototype.filter.call(cal.querySelectorAll('[data-date]'), enabled);
        var index = days.indexOf(td);
        // The keyboard core moves inside the grid. Only the month edge is ours.
        if (!queued && (forward ? index < days.length - 1 : index > 0)) return;
        target = addDays(iso, forward ? 1 : -1);
        step = forward ? 1 : -1;
        break;
      }
      default:
        return;
    }
    e.preventDefault();
    if (clamp(cal, target) === iso) return;
    if (queued && queued.date) {
      pending.set(field, { date: clamp(cal, target), step: step, reload: true });
      return;
    }
    focusDate(field, cal, target, step, true);
  }, true);

  // Focus that leaves an open popover and its toggle closes the popover.
  document.addEventListener('focusout', function (e) {
    var to = e.relatedTarget;
    var dialog = to && e.target.closest && e.target.closest(DIALOG);
    if (!dialog || !popoverOpen(dialog) || dialog.contains(to)) return;
    var toggle = toggleOf(dialog);
    if (toggle && toggle.contains(to)) return;
    dialog.hidePopover();
  });

  // A typed value moves an inline calendar to its month. A change that this
  // script fires is not trusted, and needs nothing.
  document.addEventListener('change', function (e) {
    var input = e.target;
    if (!e.isTrusted || !input.matches || !input.matches('input.autumn-datepicker-input')) return;
    var field = fieldOf(input);
    var cal = field && field.querySelector('.autumn-datepicker-inline ' + CALENDAR);
    var wanted = field && wantedDate(field);
    if (!cal || !wanted) return;
    if (cal.getAttribute('data-mode') === 'range') wanted = rangeValues(field)[0] || wanted;
    if (wanted.slice(0, 7) !== cal.getAttribute('data-month')) load(field, cal, wanted, null);
    else if (cal.getAttribute('data-mode') === 'range') mark(cal, rangeValues(field)[0], rangeValues(field)[1]);
    else mark(cal, wanted, wanted);
  });
})();
