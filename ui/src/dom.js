// Tiny DOM helpers. All text goes in through textContent, so model output is always escaped.

export function h(tag, props = {}, ...kids) {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(props || {})) {
    if (v === undefined || v === null || v === false) continue;
    if (k === 'class') el.className = v;
    else if (k === 'text') el.textContent = v;
    else if (k === 'html') throw new Error('no innerHTML');
    else if (k.startsWith('on') && typeof v === 'function') el.addEventListener(k.slice(2), v);
    else if (k === 'value') el.value = v;
    else if (k === 'checked') el.checked = !!v;
    else if (k === 'disabled') el.disabled = !!v;
    else if (k === 'style') el.style.cssText = v;
    else el.setAttribute(k, v === true ? '' : String(v));
  }
  append(el, kids);
  return el;
}

function append(el, kids) {
  for (const k of kids.flat(Infinity)) {
    if (k === null || k === undefined || k === false) continue;
    el.append(k instanceof Node ? k : document.createTextNode(String(k)));
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
  const back = h('div', { class: 'backdrop' }, body);
  let closed = false;
  const close = () => {
    if (closed) return;
    closed = true;
    back.remove();
    sheets.splice(sheets.indexOf(close), 1);
    onClose && onClose();
    if (prevFocus && prevFocus.focus) prevFocus.focus();
  };
  back.addEventListener('mousedown', (e) => {
    if (e.target === back) close();
  });
  body.append(h('div', { class: 'sheet-head' }, h('h2', { text: title }), iconButton('close', 'Close', close, 'icon')));
  build(body, close);
  document.body.append(back);
  sheets.push(close);
  const first = body.querySelector('input, textarea, select');
  if (first) first.focus();
  return close;
}

export function anySheetOpen() {
  return sheets.length > 0;
}

document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape' && sheets.length) {
    e.preventDefault();
    sheets[sheets.length - 1]();
  }
});

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
