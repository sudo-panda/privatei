import { h, fill, icon, iconButton, toast, openSheet, n0, n1, dayLabel, chipGroup } from '../dom.js';
import { call, quiet, message } from '../api.js';
import { state, go, changed } from '../state.js';

const KINDS = [
  ['gym', 'Gym'],
  ['run', 'Run'],
  ['walk', 'Walk'],
  ['cycle', 'Cycle'],
  ['swim', 'Swim'],
  ['other', 'Other'],
];

let els = {};
let day = null;

let opts = {};

/** opts.strip: show a 14-day calendar strip; opts.addPanel: a panel mounted above Food. */
function mount(root, o = {}) {
  opts = o;
  els.strip = h('div', { class: 'card day-strip hidden', 'aria-label': 'Jump to a day' });
  els.addSlot = h('div', { class: 'col' });
  els.nav = h('div', { class: 'card' });
  els.energy = h('div', { class: 'card', 'aria-label': 'Energy and protein' });
  els.plan = h('div', { class: 'card hidden', 'aria-label': 'Planned meals' });
  els.food = h('div', { class: 'card', 'aria-label': 'Food logged' });
  els.meds = h('div', { class: 'card hidden', 'aria-label': 'Medicines' });
  els.training = h('div', { class: 'card', 'aria-label': 'Training' });
  root.append(h('div', { class: 'cols' }, h('div', { class: 'col' }, els.strip, els.nav, els.energy, els.plan, els.meds), h('div', { class: 'col' }, els.addSlot, els.food, els.training)));
  if (opts.addPanel) opts.addPanel.mount(els.addSlot, { compact: true });
}

async function refresh(o = {}) {
  if (!state.date) state.date = state.today;
  const [d] = await Promise.all([call('get_day', { date: state.date }), opts.addPanel ? opts.addPanel.refresh(o) : null]);
  day = d;
  render();
  if (opts.strip) renderStrip();
}

function renderStrip() {
  els.strip.classList.remove('hidden');
  const end = day.today;
  const days = [];
  for (let i = 13; i >= 0; i--) days.push(shift(end, -i));
  const L = ['S', 'M', 'T', 'W', 'T', 'F', 'S'];
  fill(
    els.strip,
    h('div', { class: 'strip' }, days.map((d) => {
      const [y, m, dd] = d.split('-').map(Number);
      const dow = new Date(y, m - 1, dd).getDay();
      return h('button', { type: 'button', class: 'strip-day', 'aria-pressed': String(d === state.date), 'aria-label': dayLabel(d, day.today), onclick: () => setDate(d) },
        h('span', { class: 'small muted', text: L[dow] }), h('b', { class: 'num', text: String(dd) }));
    })),
  );
}

function setDate(d) {
  state.date = d;
  return refresh();
}

function shift(dateStr, n) {
  const [y, m, d] = dateStr.split('-').map(Number);
  const dt = new Date(y, m - 1, d + n, 12);
  const p = (x) => String(x).padStart(2, '0');
  return `${dt.getFullYear()}-${p(dt.getMonth() + 1)}-${p(dt.getDate())}`;
}

function render() {
  const d = day;
  fill(
    els.nav,
    h(
      'div',
      { class: 'row spread' },
      iconButton('left', 'Previous day', () => setDate(shift(d.date, -1)), 'icon'),
      h('div', { class: 'grow', style: 'text-align:center' }, h('h2', { id: 'day-label', text: dayLabel(d.date, d.today) }), d.isToday ? null : h('div', { class: 'small muted', text: d.date })),
      h('button', { class: 'icon', type: 'button', 'aria-label': 'Next day', title: 'Next day', disabled: d.isToday, onclick: () => setDate(shift(d.date, 1)) }, icon('right')),
    ),
    d.isToday ? null : h('button', { type: 'button', class: 'block', onclick: () => setDate(d.today) }, 'Back to today'),
  );

  const left = d.target - d.kcal;
  const kPct = Math.min(100, (d.kcal / Math.max(1, d.target)) * 100);
  const pPct = Math.min(100, (d.prot / Math.max(1, d.protFloor || 1)) * 100);
  fill(
    els.energy,
    h('div', { class: 'row spread' }, h('h3', { text: 'Energy' }), h('span', { class: `small ${d.over ? 'error' : 'muted'}`, text: d.over ? `${n0(-left)} kcal over` : `${n0(left)} kcal left` })),
    h('div', { class: 'row' }, h('span', { class: 'big', 'data-testid': 'kcal', text: n0(d.kcal) }), h('span', { class: 'muted num', text: `/ ${n0(d.target)} kcal` })),
    h('div', { class: `bar ${d.over ? 'over' : ''}`, role: 'progressbar', 'aria-label': 'Energy against target', 'aria-valuemin': 0, 'aria-valuemax': n0(d.target), 'aria-valuenow': n0(d.kcal) }, h('span', { style: `width:${kPct}%` })),
    h('div', { class: 'row spread' }, h('h3', { text: 'Protein' }), h('span', { class: `small ${d.protMet ? 'ok' : 'muted'}`, text: d.protMet ? 'Floor reached' : `${n1(d.protFloor - d.prot)} g to go` })),
    h('div', { class: 'row' }, h('span', { class: 'big', 'data-testid': 'prot', text: n1(d.prot) }), h('span', { class: 'muted num', text: `/ ${n0(d.protFloor)} g` })),
    h('div', { class: `bar prot ${d.protMet ? 'good' : ''}`, role: 'progressbar', 'aria-label': 'Protein against floor', 'aria-valuemin': 0, 'aria-valuemax': n0(d.protFloor), 'aria-valuenow': n1(d.prot) }, h('span', { style: `width:${pPct}%` })),
    h('div', { class: 'row spread' }, h('h3', { text: 'Fiber' }), h('span', { class: `small ${d.fiberMet ? 'ok' : 'muted'}`, text: !d.fiberKnown ? 'no fiber data yet' : d.fiberMet ? 'Floor reached' : `${n1(d.fiberFloor - d.fiber)} g to go` })),
    h('div', { class: 'row' }, h('span', { class: 'big', 'data-testid': 'fiber', text: n1(d.fiber) }), h('span', { class: 'muted num', text: `/ ${n0(d.fiberFloor)} g` })),
    h('div', { class: `bar fiber ${d.fiberMet ? 'good' : ''}`, role: 'progressbar', 'aria-label': 'Fiber against floor', 'aria-valuemin': 0, 'aria-valuemax': n0(d.fiberFloor), 'aria-valuenow': n1(d.fiber) }, h('span', { style: `width:${Math.min(100, (d.fiber / Math.max(1, d.fiberFloor)) * 100)}%` })),
    d.micros.some((m) => m.pct !== null)
      ? h('div', { class: 'micro-row', 'aria-label': 'Vitamins and minerals today' }, d.micros.map((m) =>
          h('span', { class: `micro ${m.pct >= 100 ? 'ok' : m.pct >= 50 ? 'mid' : 'short'}`, title: `${m.label}: ${n1(m.amount)} ${m.unit} of ${m.reference} ${m.unit}` },
            h('span', { class: 'micro-name', text: m.label.replace('Vitamin ', 'Vit ') }), h('b', { class: 'num', text: `${n0(m.pct)}%` }))))
      : null,
    d.burn > 0
      ? h(
          'label',
          { class: 'row small' },
          h('input', {
            type: 'checkbox',
            checked: d.addBurn,
            'aria-label': 'Add training burn to target',
            onchange: async (e) => {
              try {
                await call('set_add_burn', { on: e.target.checked });
              } finally {
                refresh();
              }
            },
          }),
          h('span', { text: `Add ${n0(d.burn)} kcal training burn to target` }),
        )
      : null,
  );

  // Plan
  els.plan.classList.toggle('hidden', d.plan.length === 0);
  fill(
    els.plan,
    h('h3', { text: 'Planned meals' }),
    h(
      'div',
      { class: 'list' },
      d.plan.map((m) =>
        h(
          'label',
          { class: 'item' },
          h('input', {
            type: 'checkbox',
            checked: m.done,
            'aria-label': `Ate ${m.name}`,
            onchange: async () => {
              try {
                await call('toggle_plan', { date: d.date, idx: m.idx });
              } finally {
                refresh();
              }
            },
          }),
          h('span', { class: 'name' }, h('div', { text: m.name }), h('div', { class: 'meta', text: m.slot })),
          h('span', { class: 'num small', text: `${n0(m.kcal)} kcal · ${n1(m.prot)} g` }),
        ),
      ),
    ),
  );

  // Food
  const groups = d.meals.map((g) =>
    h(
      'div',
      { class: 'list', 'data-meal': g.meal },
      h('div', { class: 'row spread', style: 'padding-top:6px' }, h('h3', { text: g.meal }), h('span', { class: 'num small muted', text: `${n0(g.kcal)} kcal · ${n1(g.prot)} g` })),
      g.rows.map((r) =>
        h(
          'div',
          { class: 'item', 'data-log': r.id },
          h('span', { class: 'name' }, h('div', { text: r.name }), r.est ? h('div', { class: 'meta', text: 'estimated weight' }) : null),
          h('span', { class: 'num small', text: `${n0(r.kcal)} · ${n1(r.prot)} g` }),
          iconButton('trash', `Delete ${r.name}`, async () => {
            await call('delete_log', { id: r.id });
            toast('Deleted');
            refresh();
          }),
        ),
      ),
    ),
  );
  fill(
    els.food,
    h('div', { class: 'card-head' }, h('h2', { text: 'Food' }), opts.addPanel ? null : h('button', { type: 'button', class: 'ghost', onclick: () => go('add') }, icon('add'), 'Add food')),
    groups.length ? groups : h('p', { class: 'muted', text: 'Nothing logged yet.' }),
    d.canCopyYesterday
      ? h('button', {
          type: 'button',
          onclick: async () => {
            const n = await call('copy_yesterday', { date: d.date });
            toast(`Copied ${n} item${n === 1 ? '' : 's'} from the day before`);
            refresh();
          },
        }, "Copy yesterday's food")
      : null,
  );

  // Meds
  els.meds.classList.toggle('hidden', d.meds.length === 0);
  fill(
    els.meds,
    h('h3', { text: 'Medicines' }),
    h(
      'div',
      { class: 'list' },
      d.meds.map((m) =>
        h(
          'label',
          { class: 'item', 'data-med': m.medId },
          h('input', {
            type: 'checkbox',
            checked: m.taken,
            'aria-label': `Took ${m.name}`,
            onchange: async () => {
              try {
                await call('toggle_dose', { date: d.date, medId: m.medId });
              } finally {
                refresh();
              }
            },
          }),
          h('span', { class: 'name' }, h('div', { text: m.name }), h('div', { class: 'meta', text: [m.dose, m.time].filter(Boolean).join(' · ') })),
          m.status === 'overdue' ? h('span', { class: 'badge warn', text: 'overdue' }) : null,
          m.status === 'missed' ? h('span', { class: 'badge over', text: 'missed' }) : null,
        ),
      ),
    ),
  );

  // Training
  fill(
    els.training,
    h('div', { class: 'card-head' }, h('h2', { text: 'Training' }), h('button', { type: 'button', class: 'ghost', onclick: openTraining, disabled: d.date > d.today }, icon('add'), 'Add training')),
    d.workouts.length
      ? h(
          'div',
          { class: 'list' },
          d.workouts.map((w) =>
            h(
              'div',
              { class: 'item' },
              h('span', { class: 'name' }, h('div', { text: w.name }), h('div', { class: 'meta', text: `${KINDS.find((k) => k[0] === w.kind)?.[1] || w.kind} · ${n0(w.mins)} min` })),
              h('span', { class: 'num small', text: `${n0(w.kcal)} kcal` }),
              iconButton('trash', `Delete ${w.name}`, async () => {
                await call('delete_workout', { id: w.id });
                refresh();
              }),
            ),
          ),
        )
      : h('p', { class: 'muted', text: 'No training logged.' }),
  );
}

export function openTraining() {
  const date = state.date;
  openSheet('Add training', (body, close) => {
    let kind = 'gym';
    const mins = h('input', { type: 'number', inputmode: 'numeric', min: 1, max: 1440, id: 'tr-mins', 'aria-label': 'Minutes' });
    const kcal = h('input', { type: 'number', inputmode: 'numeric', min: 0, max: 5000, id: 'tr-kcal', 'aria-label': 'Calories (optional)' });
    const note = h('input', { type: 'text', maxlength: 80, id: 'tr-note', 'aria-label': 'Note' });
    const est = h('div', { class: 'small muted', 'data-testid': 'burn-estimate' });
    const err = h('div', { class: 'error', role: 'alert' });
    const update = async () => {
      const m = Number(mins.value);
      if (!(m > 0)) {
        est.textContent = '';
        kcal.placeholder = '';
        return;
      }
      const e = await quiet('burn_estimate', { kind, mins: m }).catch(() => null);
      if (e !== null) {
        est.textContent = `Estimate: ${n0(e)} kcal`;
        kcal.placeholder = n0(e);
      }
    };
    mins.addEventListener('input', update);
    const paste = h('textarea', { id: 'tr-paste', 'aria-label': 'Paste workouts', placeholder: '2026-11-20, Run, 30, 320\n21.11.2026; Gym; 1:05' });
    body.append(
      chipGroup(KINDS, kind, (v) => { kind = v; update(); }, 'Sport'),
      h('div', { class: 'grid2' }, h('label', { class: 'field' }, 'Minutes', mins), h('label', { class: 'field' }, 'kcal (optional)', kcal)),
      est,
      h('label', { class: 'field' }, 'Note', note),
      err,
      h('button', {
        type: 'button',
        class: 'primary',
        onclick: async () => {
          err.textContent = '';
          const m = Number(mins.value);
          if (!(m > 0 && m <= 1440)) return (err.textContent = 'Enter minutes between 1 and 1440.');
          const k = kcal.value === '' ? null : Number(kcal.value);
          if (k !== null && !(k >= 0 && k <= 5000)) return (err.textContent = 'kcal must be between 0 and 5000.');
          try {
            await quiet('add_workout', { date, kind, mins: m, kcal: k, name: note.value });
            toast('Training saved');
            close();
            changed();
          } catch (e) {
            err.textContent = message(e);
          }
        },
      }, 'Save training'),
      h('details', {},
        h('summary', { class: 'small', style: 'font-weight:600' }, 'Paste from another app'),
        h('div', { class: 'small muted', text: 'One session per line: date, sport, minutes (or H:MM), kcal. Separate with commas, semicolons or tabs.' }),
        paste,
        h('button', {
          type: 'button',
          onclick: async () => {
            const r = await call('import_workouts', { text: paste.value });
            toast(`Added ${r.added} session${r.added === 1 ? '' : 's'}${r.skipped ? `, skipped ${r.skipped} line${r.skipped === 1 ? '' : 's'}` : ''}`);
            if (r.added) {
              close();
              changed();
            }
          },
        }, 'Import pasted sessions'),
      ),
    );
  });
}

export default { id: 'today', title: 'Today', icon: 'today', mount, refresh, setDate, get els() { return els; } };
