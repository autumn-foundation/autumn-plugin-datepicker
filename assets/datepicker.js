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

  var DAY = 86400000;
  var FIELD = '[data-autumn-datepicker]';
  var CALENDAR = '[data-autumn-datepicker-calendar]';
  var CELL = CALENDAR + ' [data-date]';
  var DIALOG = '.autumn-datepicker-dialog';

  var ready = new WeakSet();
  var pending = new WeakMap(); // field -> { date, step } or { nav }, after a swap
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
    return id ? document.querySelector('[aria-controls="' + id + '"]') : null;
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

  function load(field, cal, iso, focus) {
    if (!window.htmx || !cal.isConnected) return;
    pending.set(field, focus);
    var params = new URLSearchParams();
    inputs(field).forEach(function (input) {
      if (input.name) params.append(input.name, input.value);
    });
    var url = cal.getAttribute('data-src') + '&dp-month=' + iso.slice(0, 7) + '&' + params;
    window.htmx.ajax('GET', url, { target: cal, swap: 'outerHTML' });
  }

  // Focus `iso`. A disabled day moves on by `step` days. A day outside the
  // grid loads its month when `mayLoad` is true.
  function focusDate(field, cal, iso, step, mayLoad) {
    var target = clamp(cal, iso);
    for (var i = 0; i < 400; i++) {
      var td = cellOf(cal, target);
      if (!td) {
        if (mayLoad) load(field, cal, target, { date: target, step: step });
        return !!mayLoad;
      }
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

  function focusStop(cal) {
    var stop = cal.querySelector('[data-date][tabindex="0"]') ||
      cal.querySelector('[data-date]:not([aria-disabled="true"])');
    if (stop) stop.focus();
  }

  // After a swap: focus what the key or the button asked for.
  function restore(field, cal) {
    var want = pending.get(field);
    if (!want) return;
    pending.delete(field);
    // The keyboard core picks the tab stop after `autumn:init`. Focus after it.
    setTimeout(function () {
      if (want.nav) {
        var button = cal.querySelector('[data-nav="' + want.nav + '"]');
        if (button && !button.disabled) button.focus();
        else focusStop(cal);
        return;
      }
      if (!focusDate(field, cal, want.date, want.step, false)) focusStop(cal);
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
        rangeStart.set(field, iso);
        setValue(list[0], iso);
        setValue(list[1], '');
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
    var wanted = wantedDate(field);
    if (wanted && wanted.slice(0, 7) !== cal.getAttribute('data-month')) {
      load(field, cal, wanted, { date: wanted, step: 0 });
      return;
    }
    if (wanted && cal.getAttribute('data-mode') !== 'range') mark(cal, wanted, wanted);
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
    each(root, FIELD, setUp);
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
    switch (e.key) {
      case 'Enter':
      case ' ':
        e.preventDefault();
        pickDate(field, td);
        return;
      case 'ArrowUp': target = addDays(iso, -7); step = -1; break;
      case 'ArrowDown': target = addDays(iso, 7); step = 1; break;
      case 'Home': target = addDays(iso, -td.cellIndex); step = 1; break;
      case 'End': target = addDays(iso, 6 - td.cellIndex); step = -1; break;
      case 'PageUp': target = addMonths(iso, e.shiftKey ? -12 : -1); step = -1; break;
      case 'PageDown': target = addMonths(iso, e.shiftKey ? 12 : 1); step = 1; break;
      case 'ArrowLeft':
      case 'ArrowRight': {
        var rtl = getComputedStyle(td.closest('table')).direction === 'rtl';
        var forward = (e.key === 'ArrowRight') !== rtl;
        var days = Array.prototype.filter.call(cal.querySelectorAll('[data-date]'), enabled);
        var index = days.indexOf(td);
        // The keyboard core moves inside the grid. Only the month edge is ours.
        if (forward ? index < days.length - 1 : index > 0) return;
        target = addDays(iso, forward ? 1 : -1);
        step = forward ? 1 : -1;
        break;
      }
      default:
        return;
    }
    e.preventDefault();
    if (clamp(cal, target) === iso) return;
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

  // A typed value moves an inline calendar to its month.
  document.addEventListener('change', function (e) {
    var input = e.target;
    if (!input.matches || !input.matches('input.autumn-datepicker-input')) return;
    var field = fieldOf(input);
    var cal = field && field.querySelector('.autumn-datepicker-inline ' + CALENDAR);
    var wanted = field && wantedDate(field);
    if (!cal || !wanted) return;
    if (wanted.slice(0, 7) !== cal.getAttribute('data-month')) load(field, cal, wanted, null);
    else if (cal.getAttribute('data-mode') !== 'range') mark(cal, wanted, wanted);
  });
})();
