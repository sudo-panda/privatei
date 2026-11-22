import { h, fill, icon, iconButton, toast, openSheet, overlay, n0, n1, dayLabel, chipGroup } from '../dom.js';
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
  els.strip = h('div', { class: 'hidden' });
  els.addSlot = h('div', { class: 'col' });
  els.nav = h('div', { class: opts.strip ? 'card day-nav-strip' : 'card' });
  els.energy = h('div', { class: 'card', 'aria-label': 'Energy and protein' });
  els.plan = h('div', { class: 'card hidden', 'aria-label': 'Planned meals' });
  els.food = h('div', { class: 'card', 'aria-label': 'Food logged' });
  els.meds = h('div', { class: 'card hidden', 'aria-label': 'Medicines' });
  els.training = h('div', { class: 'card', 'aria-label': 'Training' });
  root.append(h('div', { class: 'cols' }, h('div', { class: 'col' }, els.strip, els.nav, els.energy, els.plan, els.meds), h('div', { class: 'col' }, els.food, els.training)));
  if (opts.addPanel) {
    // Adding food happens in an overlay, so the Log stays a clean record of the day.
    els.addOverlay = overlay('Add food');
    opts.addPanel.mount(els.addOverlay.body, { compact: true, onLogged: () => els.addOverlay.close() });
    // Always within reach, however far down the day you've scrolled.
    root.classList.add('has-fab');
    root.append(h('button', { type: 'button', class: 'fab', id: 'open-add', 'aria-label': 'Add food', title: 'Add food (⌘N)', onclick: () => openAdd() }, icon('add')));
  }
}

export function openAdd() {
  if (els.addOverlay) els.addOverlay.open('#food-search');
}

async function refresh(o = {}) {
  if (!state.date) state.date = state.today;
  const [d] = await Promise.all([call('get_day', { date: state.date }), opts.addPanel ? opts.addPanel.refresh(o) : null]);
  day = d;
  render();
  if (opts.strip) renderStrip();
}

/** One bar: ‹  scrolling days  [Today, pinned]  ›. The selected day is always scrolled into view. */
function renderStrip() {
  const d = day;
  const L = ['S', 'M', 'T', 'W', 'T', 'F', 'S'];
  const MON = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
  // Four weeks back, or further if you've stepped back past that (plus a week before it).
  const span = Math.max(27, Math.round((new Date(d.today) - new Date(d.date)) / 864e5) + 7);
  const days = [];
  for (let i = span; i >= 1; i--) days.push(shift(d.today, -i));
  const dayBtn = (x, isToday = false) => {
    const [y, m, dd] = x.split('-').map(Number);
    const dow = new Date(y, m - 1, dd).getDay();
    return h('button', {
      type: 'button',
      class: `strip-day ${isToday ? 'is-today' : ''} ${dd === 1 ? 'month-start' : ''}`,
      'data-date': x,
      'aria-pressed': String(x === d.date),
      'aria-label': isToday ? 'Today' : dayLabel(x, d.today),
      onclick: () => setDate(x),
    },
    h('span', { class: 'strip-dow', text: isToday ? 'Today' : dd === 1 ? MON[m - 1] : L[dow] }),
    h('b', { class: 'num', text: String(dd) }));
  };
  const track = h('div', { class: 'strip-track', role: 'group', 'aria-label': 'Days' }, days.map((x) => dayBtn(x)));
  // Remember which day sat at the left edge, so redrawing doesn't move the bar.
  const old = els.nav.querySelector('.strip-track');
  let anchor = null;
  if (old) {
    const first = [...old.children].find((c) => c.offsetLeft + c.offsetWidth > old.scrollLeft);
    if (first) anchor = { date: first.dataset.date, delta: first.offsetLeft - old.scrollLeft };
  }
  fill(
    els.nav,
    h('div', { class: 'strip-bar' },
      iconButton('left', 'Previous day', () => setDate(shift(d.date, -1)), 'icon'),
      track,
      dayBtn(d.today, true),
      h('button', { class: 'icon', type: 'button', 'aria-label': 'Next day', title: 'Next day', disabled: d.isToday, onclick: () => setDate(shift(d.date, 1)) }, icon('right'))),
  );
  // Keep the bar where it was; scroll only if the selected day is out of view.
  const ref = anchor && track.querySelector(`[data-date="${anchor.date}"]`);
  track.scrollLeft = ref ? ref.offsetLeft - anchor.delta : track.scrollWidth;
  const sel = track.querySelector('[aria-pressed="true"]');
  if (sel) {
    const pad = 4;
    if (sel.offsetLeft < track.scrollLeft) track.scrollTo({ left: sel.offsetLeft - pad, behavior: 'smooth' });
    else if (sel.offsetLeft + sel.offsetWidth > track.scrollLeft + track.clientWidth) {
      track.scrollTo({ left: sel.offsetLeft + sel.offsetWidth - track.clientWidth + pad, behavior: 'smooth' });
    }
  }
  // Fade an edge when days are hidden past it (e.g. between the visible days and Today).
  const edges = () => {
    const max = track.scrollWidth - track.clientWidth;
    track.classList.toggle('more-left', track.scrollLeft > 2);
    track.classList.toggle('more-right', track.scrollLeft < max - 2);
  };
  edges();
  track.addEventListener('scroll', edges, { passive: true });
  const chip = document.getElementById('date-chip');
  if (chip) {
    const [y, m, dd] = d.date.split('-').map(Number);
    chip.textContent = new Date(y, m - 1, dd).toLocaleDateString(undefined, { weekday: 'long', day: 'numeric', month: 'long', ...(y !== new Date().getFullYear() ? { year: 'numeric' } : {}) }) + (d.isToday ? '' : ' · not today');
  }
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
  if (!opts.strip) fill(
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
  // One compact line each: name, amount / goal, what's left, and a thin bar underneath.
  const macro = (cls, label, testid, value, goal, note, noteCls, pct, barCls, aria) =>
    h('div', { class: `macro ${cls}` },
      h('span', { class: 'mlabel', text: label }),
      h('span', { class: 'mval num' }, h('b', { 'data-testid': testid, text: value }), h('span', { class: 'muted', text: ` / ${goal}` })),
      h('span', { class: `mnote small ${noteCls}`, text: note }),
      h('div', { class: `bar thin ${barCls}`, role: 'progressbar', 'aria-label': aria[0], 'aria-valuemin': 0, 'aria-valuemax': aria[1], 'aria-valuenow': aria[2] }, h('span', { style: `width:${pct}%` })));
  fill(
    els.energy,
    macro('energy', 'Energy', 'kcal', n0(d.kcal), `${n0(d.target)} kcal`, d.over ? `${n0(-left)} over` : `${n0(left)} left`, d.over ? 'error' : 'muted', kPct, d.over ? 'over' : '', ['Energy against target', n0(d.target), n0(d.kcal)]),
    macro('prot', 'Protein', 'prot', n1(d.prot), `${n0(d.protFloor)} g`, d.protMet ? 'floor reached' : `${n1(d.protFloor - d.prot)} g to go`, d.protMet ? 'ok' : 'muted', pPct, `prot ${d.protMet ? 'good' : ''}`, ['Protein against floor', n0(d.protFloor), n1(d.prot)]),
    macro('fiber', 'Fiber', 'fiber', n1(d.fiber), `${n0(d.fiberFloor)} g`, !d.fiberKnown ? 'no data yet' : d.fiberMet ? 'floor reached' : `${n1(d.fiberFloor - d.fiber)} g to go`, d.fiberMet ? 'ok' : 'muted', Math.min(100, (d.fiber / Math.max(1, d.fiberFloor)) * 100), `fiber ${d.fiberMet ? 'good' : ''}`, ['Fiber against floor', n0(d.fiberFloor), n1(d.fiber)]),
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

  // Food: a table — name and amount on the left, aligned kcal / protein / fiber columns.
  const split = (name) => {
    const i = name.lastIndexOf(' · ');
    return i > 0 ? [name.slice(0, i), name.slice(i + 3)] : [name, ''];
  };
  const cell = (v, cls = '') => h('span', { class: `fc num ${cls}`, text: v });
  const fx = (x) => (Math.round((Number(x) || 0) * 10) / 10).toFixed(1); // always one decimal so columns line up
  const groups = d.meals.map((g) =>
    h(
      'div',
      { class: 'food-group', 'data-meal': g.meal },
      h('div', { class: 'food-row food-head' },
        h('span', { class: 'fn', text: g.meal }),
        cell(g.rows.length ? n0(g.kcal) : ''),
        cell(g.rows.length ? fx(g.prot) : ''),
        cell('')),
      g.rows.length ? null : h('div', { class: 'food-empty small muted', text: 'Nothing yet' }),
      g.rows.map((r) => {
        const [name, amount] = split(r.name);
        return h(
          'button',
          { type: 'button', class: 'food-row row-edit', 'data-log': r.id, 'aria-label': `Edit ${r.name}`, onclick: () => openFoodEditor(r) },
          h('span', { class: 'fn' },
            h('span', { class: 'fname', text: name }),
            h('span', { class: 'famount', text: [amount, r.est ? 'estimated' : null, r.fromPlan ? 'from plan' : null].filter(Boolean).join(' · ') })),
          cell(n0(r.kcal)),
          cell(fx(r.prot)),
          cell(r.fiber == null ? '–' : fx(r.fiber), 'muted'),
        );
      }),
    ),
  );
  const head = h('div', { class: 'food-row food-cols', 'aria-hidden': 'true' }, h('span', { class: 'fn' }), cell('kcal'), cell('protein'), cell('fiber'));
  fill(
    els.food,
    h('div', { class: 'card-head' },
      h('h2', { text: 'Food' }),
      opts.addPanel ? null : h('button', { type: 'button', class: 'ghost', onclick: () => go('add') }, icon('add'), 'Add food')),
    h('div', { class: 'food-table' }, head, groups),
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
    h('div', { class: 'card-head' }, h('h2', { text: 'Training' }), h('button', { type: 'button', class: 'ghost', onclick: () => openTraining(), disabled: d.date > d.today }, icon('add'), 'Add training')),
    d.workouts.length
      ? h(
          'div',
          { class: 'list' },
          d.workouts.map((w) =>
            h(
              'button',
              { type: 'button', class: 'item ghost row-edit', 'aria-label': `Edit ${w.name}`, onclick: () => openTraining(w) },
              h('span', { class: 'name' }, h('div', { text: w.name }), h('div', { class: 'meta', text: `${KINDS.find((k) => k[0] === w.kind)?.[1] || w.kind} · ${n0(w.mins)} min` })),
              h('span', { class: 'num small', text: `${n0(w.kcal)} kcal` }),
              h('span', { class: 'edit-hint', 'aria-hidden': 'true', text: 'Edit' }),
            ),
          ),
        )
      : h('p', { class: 'muted', text: 'No training logged.' }),
  );
}

/** Add a session, or edit `w` ({ id, kind, mins, kcal, name }) with Save and Delete. */
export function openTraining(w = null) {
  const date = state.date;
  openSheet(w ? 'Edit training' : 'Add training', (body, close) => {
    let kind = w ? w.kind : 'gym';
    const mins = h('input', { type: 'number', inputmode: 'numeric', min: 1, max: 1440, id: 'tr-mins', 'aria-label': 'Minutes', value: w ? w.mins : '' });
    const kcal = h('input', { type: 'number', inputmode: 'numeric', min: 0, max: 5000, id: 'tr-kcal', 'aria-label': 'Calories (optional)', value: w ? w.kcal : '' });
    const note = h('input', { type: 'text', maxlength: 80, id: 'tr-note', 'aria-label': 'Note', value: w ? w.name : '' });
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
            if (w) await quiet('update_workout', { id: w.id, kind, mins: m, kcal: k, name: note.value });
            else await quiet('add_workout', { date, kind, mins: m, kcal: k, name: note.value });
            toast('Training saved');
            close();
            changed();
          } catch (e) {
            err.textContent = message(e);
          }
        },
      }, 'Save training'),
      w
        ? h('button', { type: 'button', class: 'danger', onclick: async () => { await call('delete_workout', { id: w.id }); toast('Deleted'); close(); changed(); } }, 'Delete session')
        : h('details', {},
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

/** Edit or delete a logged food. */
export function openFoodEditor(r) {
  openSheet('Edit food', (body, close) => {
    let meal = r.meal;
    const name = h('input', { type: 'text', id: 'ed-name', maxlength: 80, value: r.name, 'aria-label': 'Food name' });
    const grams = h('input', { type: 'number', id: 'ed-grams', min: 0, value: r.grams ?? '', 'aria-label': 'Grams' });
    const kcal = h('input', { type: 'number', id: 'ed-kcal', min: 0, max: 5000, value: r.kcal, 'aria-label': 'kcal' });
    const prot = h('input', { type: 'number', id: 'ed-prot', min: 0, max: 500, value: r.prot, 'aria-label': 'Protein g' });
    const fiber = h('input', { type: 'number', id: 'ed-fiber', min: 0, max: 300, value: r.fiber ?? '', 'aria-label': 'Fiber g' });
    const err = h('div', { class: 'error', role: 'alert' });
    // Changing the grams scales the numbers, like a food scale.
    const base = { g: Number(r.grams) || 0, kcal: r.kcal, prot: r.prot, fiber: r.fiber };
    grams.addEventListener('input', () => {
      const g = Number(grams.value);
      if (!(base.g > 0 && g > 0)) return;
      const f = g / base.g;
      kcal.value = Math.round(base.kcal * f);
      prot.value = Math.round(base.prot * f * 10) / 10;
      if (base.fiber != null) fiber.value = Math.round(base.fiber * f * 10) / 10;
    });
    body.append(
      h('label', { class: 'field' }, 'Name', name),
      h('div', { class: 'grid2' }, h('label', { class: 'field' }, 'Grams', grams), h('label', { class: 'field' }, 'kcal', kcal)),
      h('div', { class: 'grid2' }, h('label', { class: 'field' }, 'Protein g', prot), h('label', { class: 'field' }, 'Fiber g', fiber)),
      h('div', { class: 'field' }, h('span', { text: 'Meal' }), chipGroup(['Breakfast', 'Lunch', 'Snack', 'Dinner'].map((m) => [m, m]), meal, (v) => (meal = v), 'Meal')),
      r.fromPlan ? h('p', { class: 'small muted', text: 'This came from your plan; the planned meal changes too.' }) : null,
      err,
      h('div', { class: 'actions' },
        h('button', { type: 'button', class: 'danger', onclick: async () => { await call('delete_log', { id: r.id }); toast('Deleted'); close(); changed(); } }, 'Delete'),
        h('button', {
          type: 'button',
          class: 'primary',
          onclick: async () => {
            err.textContent = '';
            const k = Number(kcal.value), p = Number(prot.value || 0);
            if (!name.value.trim()) return (err.textContent = 'Give the food a name.');
            if (!(k > 0 && k < 5000)) return (err.textContent = 'kcal must be above 0 and below 5000.');
            if (!(p >= 0 && p <= 500)) return (err.textContent = 'Protein must be between 0 and 500 g.');
            try {
              await quiet('update_log', { id: r.id, entry: { meal, name: name.value.trim(), grams: grams.value === '' ? null : Number(grams.value), kcal: k, prot: p, est: r.est, fiber: fiber.value === '' ? null : Number(fiber.value) } });
            } catch (e) {
              return (err.textContent = message(e));
            }
            toast('Saved');
            close();
            changed();
          },
        }, 'Save')),
    );
  });
}

export default { id: 'today', title: 'Today', icon: 'today', mount, refresh, setDate, openAdd, get els() { return els; } };
