import { h, fill, toast, openSheet, iconButton, n1 } from '../dom.js';
import { call, quiet, message } from '../api.js';
import { PLACES } from './stock.js';

const placeName = (c) => PLACES.find((p) => p[0] === c)?.[1] || c;
let els = {};
let trip = 1;

function mount(root) {
  const seg = h('div', { class: 'segment', role: 'group', 'aria-label': 'Trip' });
  for (const t of [1, 2]) {
    seg.append(h('button', { type: 'button', 'data-trip': t, 'aria-pressed': String(trip === t), onclick: () => { trip = t; for (const b of seg.children) b.setAttribute('aria-pressed', String(Number(b.dataset.trip) === trip)); refresh(); } }, `Trip ${t}`));
  }
  els.budget = h('div', { class: 'card', 'aria-label': 'Budget' });
  els.list = h('div', { class: 'screen' });
  els.inStock = h('div', { class: 'card hidden', 'aria-label': 'Already in stock' });
  els.actions = h('div', { class: 'grid2' });
  root.append(seg, els.budget, addCard(), els.actions, els.list, els.inStock);
}

function addCard() {
  const name = h('input', { type: 'text', id: 'shop-name', maxlength: 80, placeholder: 'Item', 'aria-label': 'Item name' });
  const qty = h('input', { type: 'text', id: 'shop-qty', maxlength: 30, placeholder: '1 kg, 4 pots', 'aria-label': 'Amount' });
  const price = h('input', { type: 'number', id: 'shop-price', min: 0, step: '0.05', placeholder: 'CHF', 'aria-label': 'Price in CHF' });
  const cat = h('select', { id: 'shop-cat', 'aria-label': 'Place' }, PLACES.map(([v, t]) => h('option', { value: v, text: t })));
  cat.value = 'chilled';
  const err = h('div', { class: 'error', role: 'alert' });
  const add = async () => {
    err.textContent = '';
    if (!name.value.trim()) return (err.textContent = 'Name the item.');
    const p = price.value === '' ? null : Number(price.value);
    try {
      await quiet('save_list_item', { id: null, item: { name: name.value.trim(), qty: qty.value.trim(), cat: cat.value, price: p, trip, checked: false, inStock: false } });
    } catch (e) {
      return (err.textContent = message(e));
    }
    name.value = qty.value = price.value = '';
    name.focus();
    refresh();
  };
  name.addEventListener('keydown', (e) => e.key === 'Enter' && add());
  return h('div', { class: 'card' }, h('div', { class: 'grid2' }, name, qty), h('div', { class: 'grid2' }, price, cat), err, h('button', { type: 'button', onclick: add }, 'Add to list'));
}

async function refresh() {
  const v = await call('get_shop', { trip });
  const pct = Math.min(100, (v.spend / v.budget) * 100);
  fill(
    els.budget,
    h('div', { class: 'row spread' }, h('h3', { text: 'Budget' }), h('span', { class: `num small ${v.overBudget ? 'error' : 'muted'}`, 'data-testid': 'spend', text: `CHF ${v.spend.toFixed(2)} / ${v.budget.toFixed(0)}` })),
    h('div', { class: `bar ${v.overBudget ? 'over' : ''}`, role: 'progressbar', 'aria-label': 'Spend against budget', 'aria-valuemin': 0, 'aria-valuemax': v.budget, 'aria-valuenow': v.spend }, h('span', { style: `width:${pct}%` })),
    h('div', { class: 'small muted', text: `This trip: CHF ${v.tripSpend.toFixed(2)}` }),
  );
  fill(
    els.actions,
    h('button', { type: 'button', onclick: checkHave }, 'Check what you already have'),
    h('button', {
      type: 'button',
      class: 'primary',
      disabled: v.checkedCount === 0,
      onclick: async () => {
        const n = await call('bought', { trip });
        toast(`Added ${n} item${n === 1 ? '' : 's'} to stock`);
        refresh();
      },
    }, 'Bought it — add to stock'),
  );
  fill(
    els.list,
    v.groups.length
      ? v.groups.map((g) =>
          h('div', { class: 'card', 'aria-label': placeName(g.cat) }, h('h3', { text: placeName(g.cat) }), h('div', { class: 'list' }, g.items.map((it) =>
            h('label', { class: 'item', 'data-item': it.id },
              h('input', { type: 'checkbox', checked: it.checked, 'aria-label': `Got ${it.name}`, onchange: async () => { try { await call('toggle_list_item', { id: it.id }); } finally { refresh(); } } }),
              h('span', { class: 'name' }, h('div', { text: it.name }), it.qty ? h('div', { class: 'meta', text: it.qty }) : null),
              it.price !== null && it.price !== undefined ? h('span', { class: 'num small', text: it.price.toFixed(2) }) : null,
              iconButton('trash', `Delete ${it.name}`, async (e) => { e.preventDefault(); await call('delete_list_item', { id: it.id }); refresh(); }))))))
      : h('div', { class: 'card' }, h('p', { class: 'muted', text: `Nothing on trip ${trip} yet.` })),
  );
  els.inStock.classList.toggle('hidden', v.inStock.length === 0);
  fill(
    els.inStock,
    h('h3', { text: 'Already in stock' }),
    h('div', { class: 'list' }, v.inStock.map((it) =>
      h('div', { class: 'item' }, h('span', { class: 'name', text: it.name }), h('button', { type: 'button', 'aria-label': `Move ${it.name} back to the list`, onclick: async () => { await call('move_back', { listId: it.id }); refresh(); } }, 'Move back')))),
  );
}

async function checkHave() {
  const matches = await call('check_have');
  if (!matches.length) return toast('Nothing on the list matches your stock.');
  openSheet('Check what you already have', (body, close) => {
    const list = h('div', { class: 'list' });
    const done = (row) => {
      row.remove();
      if (!list.children.length) {
        close();
        refresh();
      }
    };
    for (const m of matches) {
      const row = h('div', { class: 'item', style: 'flex-wrap:wrap' },
        h('span', { class: 'name' }, h('div', { text: m.listName }), h('div', { class: 'meta', text: `Stock: ${m.stockName} (${n1(m.stockQty)} ${m.stockUnit})`.trim() })),
        h('button', { type: 'button', onclick: async () => { await call('resolve_have', { listId: m.listId, stockId: m.stockId, stillThere: true }); done(row); } }, 'Still there'),
        h('button', { type: 'button', onclick: async () => { await call('resolve_have', { listId: m.listId, stockId: m.stockId, stillThere: false }); done(row); } }, 'Used up'));
      list.append(row);
    }
    body.append(h('p', { class: 'small muted', text: '“Still there” moves the item off this trip. “Used up” keeps it on the list and takes it out of stock.' }), list);
  }, { onClose: refresh });
}

export default { id: 'shop', title: 'Shopping', short: 'Shop', icon: 'shop', mount, refresh };
