import { h, fill, iconButton, n0, n1, dayLabel } from '../dom.js';
import { call } from '../api.js';
import { state, changed } from '../state.js';
import { openTraining } from './today.js';
import add from './add.js';

const KIND = { gym: 'Gym', run: 'Run', walk: 'Walk', cycle: 'Cycle', swim: 'Swim', other: 'Other' };

let els = {};
let count = 7;

function shift(d, n) {
  const [y, m, dd] = d.split('-').map(Number);
  const t = new Date(y, m - 1, dd + n, 12);
  const p = (x) => String(x).padStart(2, '0');
  return `${t.getFullYear()}-${p(t.getMonth() + 1)}-${p(t.getDate())}`;
}

function mount(root) {
  els.add = h('div', { class: 'col' });
  els.days = h('div', { class: 'journal' });
  els.more = h('button', { type: 'button', class: 'block', onclick: () => { count += 7; refresh(); } }, 'Show older days');
  root.append(h('div', { class: 'journal-wrap' }, els.add, els.days, els.more));
  add.mount(els.add, { compact: true });
}

async function refresh(o = {}) {
  state.date = state.today; // the box at the top always logs to today
  const dates = Array.from({ length: count }, (_, i) => shift(state.today, -i));
  const [days] = await Promise.all([Promise.all(dates.map((date) => call('get_day', { date }))), add.refresh(o)]);
  fill(els.days, days.map(dayCard));
}

function dayCard(d) {
  const kPct = Math.min(100, (d.kcal / Math.max(1, d.target)) * 100);
  const pPct = Math.min(100, (d.prot / Math.max(1, d.protFloor || 1)) * 100);
  const food = d.meals.flatMap((g) => g.rows.map((r) => ({ ...r, meal: g.meal })));
  const empty = !food.length && !d.workouts.length && !d.meds.length && !d.plan.length;
  return h(
    'article',
    { class: `card journal-day ${d.isToday ? 'is-today' : ''}`, 'data-day': d.date, 'aria-label': dayLabel(d.date, d.today) },
    h('div', { class: 'row spread wrap' },
      h('h2', { text: dayLabel(d.date, d.today) }),
      h('div', { class: 'row small num' },
        h('span', { class: d.over ? 'error' : '', text: `${n0(d.kcal)} / ${n0(d.target)} kcal` }),
        h('span', { class: d.protMet ? 'ok' : 'muted', text: `${n1(d.prot)} g protein` }))),
    h('div', { class: 'grid2' },
      h('div', { class: `bar ${d.over ? 'over' : ''}`, 'aria-label': 'Energy' }, h('span', { style: `width:${kPct}%` })),
      h('div', { class: `bar prot ${d.protMet ? 'good' : ''}`, 'aria-label': 'Protein' }, h('span', { style: `width:${pPct}%` }))),
    empty ? h('p', { class: 'muted small', text: 'Nothing logged.' }) : null,
    d.plan.length
      ? h('div', { class: 'list' }, d.plan.map((m) =>
          h('label', { class: 'item' },
            h('input', { type: 'checkbox', checked: m.done, 'aria-label': `Ate ${m.name}`, onchange: async () => { try { await call('toggle_plan', { date: d.date, idx: m.idx }); } finally { changed(); } } }),
            h('span', { class: 'name' }, h('div', { text: m.name }), h('div', { class: 'meta', text: `Planned · ${m.slot}` })),
            h('span', { class: 'num small', text: `${n0(m.kcal)} · ${n1(m.prot)} g` }))))
      : null,
    food.filter((r) => !r.fromPlan).length
      ? h('div', { class: 'list' }, food.filter((r) => !r.fromPlan).map((r) =>
          h('div', { class: 'item' },
            h('span', { class: 'name' }, h('div', { text: r.name }), h('div', { class: 'meta', text: r.meal })),
            h('span', { class: 'num small', text: `${n0(r.kcal)} · ${n1(r.prot)} g` }),
            iconButton('trash', `Delete ${r.name}`, async () => { await call('delete_log', { id: r.id }); changed(); }))))
      : null,
    d.meds.length
      ? h('div', { class: 'chips' }, d.meds.map((m) =>
          h('label', { class: `chip med-chip ${m.status}` },
            h('input', { type: 'checkbox', checked: m.taken, 'aria-label': `Took ${m.name}`, onchange: async () => { try { await call('toggle_dose', { date: d.date, medId: m.medId }); } finally { changed(); } } }),
            h('span', { text: `${m.name}${m.time ? ` · ${m.time}` : ''}${m.status === 'overdue' ? ' · overdue' : m.status === 'missed' ? ' · missed' : ''}` }))))
      : null,
    d.workouts.length
      ? h('div', { class: 'small muted' }, d.workouts.map((w) => `${KIND[w.kind] || w.kind} ${n0(w.mins)} min · ${n0(w.kcal)} kcal`).join('  ·  '))
      : null,
    d.isToday ? h('div', { class: 'row' }, h('button', { type: 'button', class: 'ghost', onclick: () => { state.date = d.date; openTraining(); } }, 'Add training')) : null,
  );
}

export default { id: 'today', title: 'Journal', icon: 'today', mount, refresh };
