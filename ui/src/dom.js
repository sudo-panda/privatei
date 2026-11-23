// Tiny DOM helpers. All text goes in through textContent, so model output is always escaped.

export function h(tag, props = {}, ...kids) {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(props || {})) {
    if (v === undefined || v === null || v === false) continue;
    if (k === 'class') el.className = v;
    else if (k === 'text') el.textContent = v;
    else if (k === 'html') throw new Error('no innerHTML');
    else if (k.startsWith('on') && typeof v === 'function') el.addEventListener(k.slice(2), busyWhileSaving(el, k.slice(2), v));
    else if (k === 'value') el.value = v;
    else if (k === 'checked') el.checked = !!v;
    else if (k === 'disabled') el.disabled = !!v;
    else if (k === 'style') el.style.cssText = v;
    else el.setAttribute(k, v === true ? '' : String(v));
  }
  append(el, kids);
  return el;
}

/**
 * A button click (or checkbox change) that saves something returns a promise. Until it
 * settles the control is disabled, so a double click can't save twice.
 */
function busyWhileSaving(el, type, fn) {
  const guard = (el.tagName === 'BUTTON' && type === 'click') || (el.tagName === 'INPUT' && type === 'change');
  if (!guard) return fn;
  return function (e) {
    const out = fn.call(this, e);
    if (!out || typeof out.then !== 'function') return out;
    const was = el.disabled;
    el.disabled = true;
    el.setAttribute('aria-busy', 'true');
    const done = () => {
      el.disabled = was;
      el.removeAttribute('aria-busy');
    };
    out.then(done, done);
    return out;
  };
}

function append(el, kids) {
  for (const k of kids.flat(Infinity)) {
    if (k === null || k === undefined || k === false) continue;
    Element.prototype.append.call(el, k instanceof Node ? k : document.createTextNode(String(k)));
  }
}

export function fill(el, ...kids) {
  el.replaceChildren();
  append(el, kids);
  return el;
}

const SVG = 'http://www.w3.org/2000/svg';
const PATHS = {
  today: 'M4 6h16M4 12h16M4 18h10',
  add: 'M12 5v14M5 12h14',
  meds: 'M10.5 3.5a5 5 0 017 7l-7 7a5 5 0 01-7-7l7-7zM7 10l7 7',
  stock: 'M4 7l8-4 8 4v10l-8 4-8-4V7zM4 7l8 4 8-4M12 11v10',
  week: 'M4 6h16v14H4zM4 10h16M8 3v4M16 3v4',
  shop: 'M5 7h14l-1.5 11h-11L5 7zM9 7a3 3 0 016 0',
  gear: 'M12 15a3 3 0 100-6 3 3 0 000 6zM19.4 15a1.7 1.7 0 00.3 1.8l.1.1a2 2 0 11-2.8 2.8l-.1-.1a1.7 1.7 0 00-1.8-.3 1.7 1.7 0 00-1 1.5V21a2 2 0 11-4 0v-.1a1.7 1.7 0 00-1.1-1.5 1.7 1.7 0 00-1.8.3l-.1.1a2 2 0 11-2.8-2.8l.1-.1a1.7 1.7 0 00.3-1.8 1.7 1.7 0 00-1.5-1H3a2 2 0 110-4h.1a1.7 1.7 0 001.5-1.1 1.7 1.7 0 00-.3-1.8l-.1-.1a2 2 0 112.8-2.8l.1.1a1.7 1.7 0 001.8.3H9a1.7 1.7 0 001-1.5V3a2 2 0 114 0v.1a1.7 1.7 0 001 1.5 1.7 1.7 0 001.8-.3l.1-.1a2 2 0 112.8 2.8l-.1.1a1.7 1.7 0 00-.3 1.8V9a1.7 1.7 0 001.5 1H21a2 2 0 110 4h-.1a1.7 1.7 0 00-1.5 1z',
  chat: 'M21 12a8 8 0 01-11.6 7.1L4 20l1-4.6A8 8 0 1121 12z',
  close: 'M6 6l12 12M18 6L6 18',
  trash: 'M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3',
  left: 'M15 6l-6 6 6 6',
  right: 'M9 6l6 6-6 6',
  minus: 'M5 12h14',
  kitchen: 'M6 3h12v18H6zM6 10h12M9 6v1M9 13v3',
  chart: 'M4 19h16M7 16v-5M12 16V7M17 16v-8',
  heart: 'M12 20s-7-4.4-7-10a4 4 0 017-2.6A4 4 0 0119 10c0 5.6-7 10-7 10z',
  run: 'M13 4a1.5 1.5 0 100 3 1.5 1.5 0 000-3zM9 21l2-6 3 3v3M7 11l3-3 3 1 2 3h3M10 8l1 5',
  book: 'M5 4h10a3 3 0 013 3v13H8a3 3 0 01-3-3zM5 17a3 3 0 013-3h10',
  plus: 'M12 5v14M5 12h14',
};

export function icon(name) {
  const s = document.createElementNS(SVG, 'svg');
  s.setAttribute('viewBox', '0 0 24 24');
  s.setAttribute('fill', 'none');
  s.setAttribute('stroke', 'currentColor');
  s.setAttribute('stroke-width', '2');
  s.setAttribute('stroke-linecap', 'round');
  s.setAttribute('stroke-linejoin', 'round');
  s.setAttribute('aria-hidden', 'true');
  const p = document.createElementNS(SVG, 'path');
  p.setAttribute('d', PATHS[name] || '');
  s.append(p);
  return s;
}

export function iconButton(name, label, onclick, cls = 'icon sm') {
  return h('button', { class: cls, 'aria-label': label, title: label, type: 'button', onclick }, icon(name));
}

export function toast(msg, kind = '') {
  const box = document.getElementById('toasts');
  const t = h('div', { class: `toast ${kind}`, text: msg });
  box.append(t);
  setTimeout(() => t.remove(), 3000);
}

const sheets = [];

/** Open a bottom sheet. `build(body, close)` fills it. Escape or a backdrop click closes it. */
export function openSheet(title, build, { onClose } = {}) {
  const prevFocus = document.activeElement;
  const body = h('div', { class: 'sheet', role: 'dialog', 'aria-modal': 'true', 'aria-label': title });
  // Builders pass `cond ? el : null`; skip those instead of printing "null".
  body.append = (...kids) => append(body, kids);
  const back = h('div', { class: 'backdrop' }, body);
  let closed = false;
  const close = () => {
    if (closed) return;
    closed = true;
    back.remove();
    sheets.splice(sheets.indexOf(close), 1);
    onClose && onClose();
    refocus(prevFocus);
  };
  back.addEventListener('mousedown', (e) => {
    if (e.target === back) close();
  });
  body.append(h('div', { class: 'sheet-head' }, h('h2', { text: title }), iconButton('close', 'Close', close, 'icon')));
  build(body, close);
  document.body.append(back);
  close.el = body;
  sheets.push(close);
  // Into the sheet: the first box, else its main button (a confirm has no box), else Close.
  const first = body.querySelector('input, textarea, select') || body.querySelector('.actions button, button.primary') || body.querySelector('button');
  if (first) first.focus();
  return close;
}

/**
 * A large overlay built once and shown/hidden, so text typed in it survives closing.
 * Escape or a backdrop click closes it, like a sheet.
 */
export function overlay(title) {
  const body = h('div', { class: 'sheet overlay-sheet', role: 'dialog', 'aria-modal': 'true', 'aria-label': title });
  const back = h('div', { class: 'backdrop overlay-backdrop hidden' }, body);
  let open = false;
  let prevFocus = null;
  const close = () => {
    if (!open) return;
    open = false;
    back.classList.add('hidden');
    sheets.splice(sheets.indexOf(close), 1);
    refocus(prevFocus);
  };
  close.el = body;
  back.addEventListener('mousedown', (e) => {
    if (e.target === back) close();
  });
  const content = h('div', { class: 'overlay-body' });
  body.append(h('div', { class: 'sheet-head' }, h('h2', { text: title }), iconButton('close', 'Close', close, 'icon')), content);
  document.body.append(back);
  return {
    body: content,
    isOpen: () => open,
    close,
    open(focusSel) {
      if (open) return;
      open = true;
      prevFocus = document.activeElement;
      back.classList.remove('hidden');
      sheets.push(close);
      const f = focusSel ? body.querySelector(focusSel) : null;
      if (f) f.focus();
    },
  };
}

export function anySheetOpen() {
  return sheets.length > 0;
}

let tabBack = false;
const focusables = (box) => [...box.querySelectorAll(FOCUSABLE)].filter((x) => x.getClientRects().length && !x.closest('.hidden') && !inClosedDetails(x));

const FOCUSABLE = 'button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), a[href], summary, [tabindex]:not([tabindex="-1"])';

document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape' && sheets.length) {
    e.preventDefault();
    sheets[sheets.length - 1]();
  }
  // Tab stays inside the open sheet instead of wandering into the page behind it.
  if (e.key === 'Tab' && sheets.length) {
    tabBack = e.shiftKey;
    const box = sheets[sheets.length - 1].el;
    const items = focusables(box);
    if (!items.length) return;
    // Step through the sheet's controls ourselves, wrapping at the ends. Doing it by hand
    // works the same in every browser (Safari's own Tab skips buttons and leaves the page).
    e.preventDefault();
    const i = items.indexOf(document.activeElement);
    const n = items.length;
    const next = i < 0 ? (e.shiftKey ? n - 1 : 0) : (i + (e.shiftKey ? -1 : 1) + n) % n;
    items[next].focus();
  }
});

/** Inside a closed <details> (its summary excepted): laid out, but Tab skips it. */
function inClosedDetails(x) {
  for (let d = x.closest('details'); d; d = d.parentElement && d.parentElement.closest('details')) {
    if (!d.open && !(x.closest('summary') && x.closest('summary').parentElement === d)) return true;
  }
  return false;
}

/** A selector that finds the same control after a redraw: id, date, food, part, chip value or label. */
function focusKey(el) {
  if (!el || !el.tagName) return null;
  if (el.id) return `#${CSS.escape(el.id)}`;
  const tag = el.tagName.toLowerCase();
  for (const a of ['data-date', 'data-ref', 'data-part', 'data-range', 'data-trip', 'aria-label']) {
    const v = el.getAttribute(a);
    if (v) return `${tag}[${a}="${CSS.escape(v)}"]`;
  }
  return null;
}

/** Focus `el`, or, if a redraw replaced it, the control that took its place. */
function refocus(el) {
  if (!el || !el.focus) return;
  if (el.isConnected) return el.focus({ preventScroll: true });
  const sel = focusKey(el);
  const next = sel && document.querySelector(sel);
  if (next && !next.closest('.hidden')) next.focus({ preventScroll: true });
}

// Screens redraw their read-only parts after every change. When that removes the focused
// control (a day in the date bar, a ticked checkbox), keyboard focus would fall back to the
// top of the page; put it on the replacement instead.
let lastFocus = null;
document.addEventListener('focusin', (e) => {
  lastFocus = e.target;
  // Safari's Tab skips buttons, so it can step past the sheet's ends: bring it back in.
  if (!sheets.length) return;
  const box = sheets[sheets.length - 1].el;
  if (box.contains(e.target)) return;
  const items = focusables(box);
  if (items.length) (tabBack ? items[items.length - 1] : items[0]).focus();
});
new MutationObserver(() => {
  const a = lastFocus;
  if (!a || a.isConnected) return;
  if (document.activeElement && document.activeElement !== document.body) return;
  lastFocus = null;
  refocus(a);
}).observe(document.body, { childList: true, subtree: true });

/** A yes/no confirmation sheet. */
export function confirmSheet(title, message, okLabel) {
  return new Promise((resolve) => {
    let answered = false;
    const close = openSheet(
      title,
      (body, close) => {
        body.append(
          h('p', { text: message }),
          h(
            'div',
            { class: 'actions' },
            h('button', { type: 'button', onclick: () => close() }, 'Cancel'),
            h('button', { type: 'button', class: 'primary danger-fill', onclick: () => { answered = true; resolve(true); close(); } }, okLabel),
          ),
        );
      },
      { onClose: () => { if (!answered) resolve(false); } },
    );
    void close;
  });
}

export const n0 = (x) => String(Math.round(Number(x) || 0));
export const n1 = (x) => {
  const v = Math.round((Number(x) || 0) * 10) / 10;
  return Number.isInteger(v) ? String(v) : v.toFixed(1);
};

const DOW = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
const MON = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

/** "Sun 22 Nov" for a YYYY-MM-DD string (display only). */
export function dayLabel(date, today) {
  if (date === today) return 'Today';
  const [y, m, d] = date.split('-').map(Number);
  const dt = new Date(y, m - 1, d);
  return `${DOW[dt.getDay()]} ${d} ${MON[m - 1]}`;
}

export const weekdayShort = DOW;

export function chipGroup(options, value, onPick, label) {
  const wrap = h('div', { class: 'chips', role: 'group', 'aria-label': label });
  const set = (v) => {
    for (const b of wrap.children) b.setAttribute('aria-pressed', String(b.dataset.v === v));
  };
  for (const [v, text] of options) {
    wrap.append(h('button', { type: 'button', class: 'chip', 'data-v': v, 'aria-pressed': String(v === value), onclick: () => { set(v); onPick(v); } }, text));
  }
  wrap.setValue = set;
  return wrap;
}
