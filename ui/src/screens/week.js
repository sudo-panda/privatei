import { h, fill, toast, iconButton, n0, n1, dayLabel } from '../dom.js';
import { call, quiet, message } from '../api.js';
import { state, go } from '../state.js';

const MEALS = ['Breakfast', 'Lunch', 'Snack', 'Dinner'];
let els = {};
let mode = 'plan';
const open = new Set();

function mount(root) {
  const seg = h('div', { class: 'segment', role: 'group', 'aria-label': 'Week view' });
  const btn = (id, text) =>
    h('button', { type: 'button', 'aria-pressed': String(mode === id), 'data-mode': id, onclick: () => { mode = id; for (const b of seg.children) b.setAttribute('aria-pressed', String(b.dataset.mode === mode)); refresh(); } }, text);
  seg.append(btn('plan', 'The plan'), btn('history', 'What happened'));
  els.body = h('div', { class: 'screen' });
  root.append(seg, els.body);
}

async function refresh() {
  if (mode === 'plan') return renderPlan(await call('get_week'));
  return renderHistory(await call('get_history'));
}

function renderPlan(w) {
  fill(
    els.body,
    h(
      'div',
      { class: 'card' },
      h('div', { class: 'row spread' }, h('h2', { text: 'The plan' }), h('span', { class: 'small muted num', 'data-testid': 'plan-avg', text: w.plannedDays ? `avg ${n0(w.avgKcal)} kcal · ${n1(w.avgProt)} g` : 'nothing planned' })),
      h('button', {
        type: 'button',
        onclick: async () => {
          const n = await call('repeat_last_week');
          toast(n ? `Planned ${n} more day${n === 1 ? '' : 's'}` : 'Those days are already planned');
          refresh();
        },
      }, 'Repeat the last planned week'),
    ),
    h('div', { class: 'card' }, h('div', { class: 'list' }, w.days.map((d) => dayRow(d)))),
  );
}

function dayRow(d) {
  const isOpen = open.has(d.date);
  const head = h(
    'button',
    {
      type: 'button',
      class: 'item ghost',
      style: 'text-align:left;border-radius:0;min-height:44px',
      'aria-expanded': String(isOpen),
      'aria-label': `${dayLabel(d.date, state.today)}, ${d.meals.length} planned meals`,
      onclick: () => {
        isOpen ? open.delete(d.date) : open.add(d.date);
        refresh();
      },
    },
    h('span', { class: 'name' }, h('b', { text: dayLabel(d.date, state.today) }), d.isToday ? h('span', { class: 'badge', style: 'margin-left:6px', text: 'today' }) : null),
    h('span', { class: 'num small muted', text: d.meals.length ? `${n0(d.kcal)} kcal · ${n1(d.prot)} g` : '—' }),
  );
  if (!isOpen) return h('div', { class: 'item', style: 'padding:0', 'data-plan-day': d.date }, head);
  return h('div', { 'data-plan-day': d.date, style: 'border-top:1px solid var(--line)' }, head, planEditor(d));
}

function planEditor(d) {
  const meals = d.meals.map((m) => ({ ...m }));
  const save = async () => {
    await call('set_plan', { date: d.date, meals });
    refresh();
  };
  const slot = h('select', { 'aria-label': 'Meal slot' }, MEALS.map((m) => h('option', { value: m, text: m })));
  const name = h('input', { type: 'text', maxlength: 80, placeholder: 'Meal', 'aria-label': 'Planned meal name' });
  const kcal = h('input', { type: 'number', min: 0, max: 5000, placeholder: 'kcal', 'aria-label': 'Planned kcal' });
  const prot = h('input', { type: 'number', min: 0, max: 500, placeholder: 'prot g', 'aria-label': 'Planned protein' });
  const err = h('div', { class: 'error' });
  return h(
    'div',
    { style: 'padding:0 0 10px;display:flex;flex-direction:column;gap:8px' },
    meals.length
      ? h('div', { class: 'list' }, meals.map((m, i) =>
          h('div', { class: 'item' },
            h('span', { class: 'name' }, h('div', { text: m.name }), h('div', { class: 'meta', text: `${m.slot}${m.done ? ' · eaten' : ''}` })),
            h('span', { class: 'num small', text: `${n0(m.kcal)} · ${n1(m.prot)} g` }),
            iconButton('trash', `Remove ${m.name} from the plan`, () => { meals.splice(i, 1); save(); }))))
      : h('p', { class: 'muted small', text: 'No meals planned.' }),
    h('div', { class: 'grid2' }, slot, name),
    h('div', { class: 'grid2' }, kcal, prot),
    err,
    h('button', {
      type: 'button',
      onclick: async () => {
        err.textContent = '';
        if (!name.value.trim()) return (err.textContent = 'Name the meal.');
        const k = Number(kcal.value), p = Number(prot.value || 0);
        if (!(k >= 0 && k < 5000) || !(p >= 0 && p <= 500)) return (err.textContent = 'Check the kcal and protein.');
        meals.push({ slot: slot.value, name: name.value.trim(), kcal: k, prot: p, done: false });
        try {
          await quiet('set_plan', { date: d.date, meals });
          refresh();
        } catch (e) {
          meals.pop();
          err.textContent = message(e);
        }
      },
    }, 'Add to plan'),
  );
}

function renderHistory(hist) {
  const s = hist.summary;
  const used = s.daysUsed;
  fill(
    els.body,
    h(
      'div',
      { class: 'card', 'aria-label': 'Summary' },
      h('h2', { text: 'What happened' }),
      h('div', { class: 'small muted', text: used ? `Last ${hist.days.length} days, ${used} with something logged` : 'Nothing logged in the last 60 days.' }),
      h(
        'div',
        { class: 'stats' },
        stat('Avg kcal', n0(s.avgKcal)),
        stat('Avg protein', `${n1(s.avgProt)} g`),
        stat('Protein floor met', `${s.protDays}/${used}`),
        stat('Medicines taken', s.adherence === null ? '—' : `${n0(s.adherence)}%`),
        stat('Days trained', String(s.daysTrained)),
        stat('Days used', String(used)),
      ),
    ),
    h(
      'div',
      { class: 'card' },
      h('div', { class: 'list' }, hist.days.map((d) =>
        h('button', {
          type: 'button',
          class: 'item ghost day-row',
          style: 'text-align:left;border-radius:0;min-height:44px',
          'data-hist': d.date,
          'aria-label': `Open ${dayLabel(d.date, state.today)} on Today`,
          onclick: () => {
            state.date = d.date;
            go('today');
          },
        },
        h('span', { class: 'name' }, h('b', { text: dayLabel(d.date, state.today) }), h('div', { class: 'meta', text: [d.mins ? `${n0(d.mins)} min trained` : null, d.due ? `meds ${d.taken}/${d.due}` : null].filter(Boolean).join(' · ') || (d.used ? '' : 'not used') })),
        h('span', { class: `num small ${d.over ? 'error' : ''}`, text: `${n0(d.kcal)} kcal` }),
        h('span', { class: `num small ${d.protMet ? 'ok' : 'muted'}`, text: `${n1(d.prot)} g` }),
        ))),
    ),
  );
}

function stat(label, value) {
  return h('div', { class: 'stat' }, h('span', { class: 'small muted', text: label }), h('b', { text: value }));
}

export default { id: 'week', title: 'Week', icon: 'week', mount, refresh };
