import { h, fill, toast, openSheet, confirmSheet, chipGroup, n1 } from '../dom.js';
import { call, quiet, message } from '../api.js';
import { state } from '../state.js';

export const PLACES = [
  ['cooked', 'Cooked'],
  ['chilled', 'Fridge'],
  ['produce', 'Produce'],
  ['shelf', 'Shelf'],
  ['freezer', 'Freezer'],
];
const placeName = (c) => PLACES.find((p) => p[0] === c)?.[1] || c;

let els = {};
let view = null;
let soonOnly = false;

function mount(root) {
  els.search = h('input', { type: 'search', id: 'stock-search', placeholder: 'Search stock', 'aria-label': 'Search stock', autocomplete: 'off' });
  els.search.addEventListener('input', render);
  els.banner = h('button', { type: 'button', class: 'banner hidden', id: 'soon-banner', onclick: () => { soonOnly = !soonOnly; render(); } });
  els.list = h('div', { class: 'masonry' });
  root.append(
    h('div', { class: 'row toolbar' }, h('div', { class: 'grow' }, els.search), h('button', { type: 'button', class: 'primary', onclick: () => openEditor(null, '') }, 'Add')),
    els.banner,
    els.list,
  );
}

async function refresh() {
  view = await call('get_stock');
  render();
}

function badge(it) {
  if (it.eatToday) return h('span', { class: 'badge over', text: 'eat today' });
  if (it.daysLeft === null || it.daysLeft === undefined) return null;
  if (it.daysLeft < 0) return h('span', { class: 'badge over', text: 'past use-by' });
  if (it.daysLeft === 0) return h('span', { class: 'badge warn', text: 'use today' });
  if (it.soon) return h('span', { class: 'badge warn', text: `use in ${it.daysLeft} d` });
  return null;
}

function render() {
  if (!view) return;
  const q = els.search.value.trim().toLowerCase();
  els.banner.classList.toggle('hidden', view.soonCount === 0);
  els.banner.textContent = soonOnly
    ? `Showing ${view.soonCount} item${view.soonCount === 1 ? '' : 's'} to use soon — show all`
    : `${view.soonCount} item${view.soonCount === 1 ? '' : 's'} to use within 7 days`;
  els.banner.setAttribute('aria-pressed', String(soonOnly));
  const sections = view.sections
    .map((s) => ({ ...s, items: s.items.filter((i) => (!q || i.name.toLowerCase().includes(q)) && (!soonOnly || i.soon)) }))
    .filter((s) => s.items.length);
  fill(
    els.list,
    sections.map((s) =>
      h(
        'div',
        { class: 'card', 'aria-label': placeName(s.cat) },
        h('h3', { text: placeName(s.cat) }),
        h(
          'div',
          { class: 'list' },
          s.items.map((it) =>
            h(
              'button',
              { type: 'button', class: 'item ghost', style: 'text-align:left;border-radius:0;min-height:48px', 'aria-label': `Edit ${it.name}`, onclick: () => openEditor(it) },
              h('span', { class: 'name' }, h('div', { text: it.name }), h('div', { class: 'meta', text: it.useBy ? `use by ${it.useBy}` : `added ${it.added}` })),
              badge(it),
              h('span', { class: 'num small', text: `${n1(it.qty)} ${it.unit}`.trim() }),
            ),
          ),
        ),
      ),
    ),
    sections.length === 0 && q
      ? h('button', { type: 'button', class: 'block', id: 'add-from-search', onclick: () => openEditor(null, els.search.value.trim()) }, `Add “${els.search.value.trim()}” to stock`)
      : null,
    sections.length === 0 && !q ? h('div', { class: 'card' }, h('p', { class: 'muted', text: soonOnly ? 'Nothing to use soon.' : 'Nothing in stock yet.' })) : null,
  );
}

function stepFor(unit) {
  return ['g', 'ml'].includes(unit.trim().toLowerCase()) ? 50 : 1;
}

/** Edits a draft: nothing is written until Save; closing a new item leaves no row. */
function openEditor(it, presetName = '') {
  const d = it
    ? { name: it.name, qty: it.qty, unit: it.unit, cat: it.cat, added: it.added, useBy: it.useBy || '' }
    : { name: presetName, qty: 1, unit: '', cat: 'chilled', added: state.today, useBy: '' };
  openSheet(it ? 'Edit stock' : 'Add to stock', (body, close) => {
    const name = h('input', { type: 'text', id: 'st-name', maxlength: 80, value: d.name, 'aria-label': 'Name' });
    const qty = h('input', { type: 'number', id: 'st-qty', min: 0, step: 'any', value: d.qty, 'aria-label': 'Amount', style: 'text-align:center' });
    const unit = h('input', { type: 'text', id: 'st-unit', maxlength: 20, value: d.unit, list: 'units', 'aria-label': 'Unit', placeholder: 'g, ml, pots…' });
    const useBy = h('input', { type: 'date', id: 'st-useby', value: d.useBy, 'aria-label': 'Use by' });
    const err = h('div', { class: 'error', role: 'alert' });
    const step = (sign) => {
      const s = stepFor(unit.value);
      qty.value = String(Math.max(0, Math.round(((Number(qty.value) || 0) + sign * s) * 100) / 100));
    };
    body.append(
      h('label', { class: 'field' }, 'Name', name),
      h(
        'div',
        { class: 'grid2' },
        h('div', { class: 'field' }, h('span', { text: 'Amount' }), h('div', { class: 'stepper' },
          h('button', { type: 'button', 'aria-label': 'Less', onclick: () => step(-1) }, '−'),
          qty,
          h('button', { type: 'button', 'aria-label': 'More', onclick: () => step(1) }, '+'))),
        h('label', { class: 'field' }, 'Unit', unit),
      ),
      h('datalist', { id: 'units' }, ['g', 'ml', 'pcs', 'pots', 'packs', 'tins', 'kg', 'l'].map((u) => h('option', { value: u }))),
      h('div', { class: 'field' }, h('span', { text: 'Place' }), chipGroup(PLACES, d.cat, (v) => (d.cat = v), 'Place')),
      h('label', { class: 'field' }, 'Use by (optional)', useBy),
      err,
      h('button', {
        type: 'button',
        class: 'primary',
        onclick: async () => {
          err.textContent = '';
          const item = { name: name.value.trim(), qty: Number(qty.value), unit: unit.value.trim(), cat: d.cat, added: d.added, useBy: useBy.value || null };
          if (!item.name) return (err.textContent = 'Give the item a name.');
          if (!(item.qty >= 0)) return (err.textContent = 'The amount must be 0 or more.');
          try {
            await quiet('save_stock', { id: it ? it.id : null, item });
          } catch (e) {
            return (err.textContent = message(e));
          }
          toast('Saved');
          close();
          refresh();
        },
      }, 'Save'),
      it
        ? h('button', {
            type: 'button',
            class: 'danger',
            onclick: async () => {
              if (!(await confirmSheet('Remove from stock?', `Remove ${it.name} from stock?`, 'Remove'))) return;
              await call('delete_stock', { id: it.id });
              toast('Removed');
              close();
              refresh();
            },
          }, 'Remove from stock')
        : null,
    );
  });
}

export default { id: 'stock', title: 'Stock', icon: 'stock', mount, refresh };
