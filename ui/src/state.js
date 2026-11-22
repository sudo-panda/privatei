// Shared UI state: which day is open, and a tiny change bus.
export const state = { today: '', date: '' };

const listeners = new Set();
export function onChange(fn) {
  listeners.add(fn);
}
/** Something changed in the data; visible screens refresh their read-only parts. */
export function changed(what = 'all') {
  for (const fn of listeners) fn(what);
}

let goImpl = () => {};
export function setGo(fn) {
  goImpl = fn;
}
export function go(tab, opts) {
  goImpl(tab, opts);
}
