import { h, fill, n0, n1, dayLabel } from '../dom.js';
import { call } from '../api.js';
import { state } from '../state.js';
import { barChart, pctBars } from '../charts.js';
import { fmtMins } from './today.js';

let els = {};
let range = 30;

const short = (d) => {
  const [, m, dd] = d.split('-').map(Number);
  return `${dd}.${m}`;
};

function mount(root) {
  const seg = h('div', { class: 'segment', role: 'group', 'aria-label': 'Period' });
  for (const r of [14, 30, 60]) {
    seg.append(h('button', { type: 'button', 'data-range': r, 'aria-pressed': String(r === range), onclick: () => { range = r; for (const b of seg.children) b.setAttribute('aria-pressed', String(Number(b.dataset.range) === r)); refresh(); } }, `${r} days`));
  }
  els.body = h('div', { class: 'screen' });
  root.append(h('div', { class: 'toolbar' }, seg), els.body);
}

function stat(label, value, sub, tone = '') {
  return h('div', { class: `stat-tile ${tone}` }, h('span', { class: 'small muted', text: label }), h('b', { text: value }), sub ? h('span', { class: 'small muted', text: sub }) : null);
}

function chartCard(title, sub, chart) {
  return h('div', { class: 'card', 'aria-label': title }, h('div', { class: 'row spread wrap' }, h('h2', { text: title }), h('span', { class: 'small muted', text: sub })), chart);
}

let seq = 0;

async function refresh() {
  const my = ++seq;
  const hist = await call('get_history', { range });
  if (my !== seq) return; // switched period again while this one loaded
  const s = hist.summary;
  const days = [...hist.days].reverse(); // oldest first for charts
  const food = s.foodDays;
  const tone = (ok) => (ok ? 'good' : 'warn');

  const insights = h(
    'div',
    { class: 'card insights', 'aria-label': 'What’s missing' },
    h('h2', { text: 'What’s missing' }),
    h('div', { class: 'list' }, hist.insights.map((i) =>
      h('div', { class: `item insight ${i.level}` },
        h('span', { class: `badge ${i.level === 'warn' ? 'warn' : i.level === 'good' ? 'good' : ''}`, text: i.level === 'warn' ? 'Needs attention' : i.level === 'good' ? 'Going well' : 'Note' }),
        h('span', { class: 'name' }, h('b', { text: i.title }), h('div', { class: 'meta', text: i.detail }))))),
  );

  const tiles = h(
    'div',
    { class: 'stat-grid' },
    stat('Avg calories', n0(s.avgKcal), `target ${n0(hist.kcalTarget)}`, tone(s.avgKcal <= hist.kcalTarget)),
    stat('Avg protein', `${n1(s.avgProt)} g`, `${s.protDays}/${food} days ≥ ${n0(hist.protFloor)} g`, tone(s.avgProt >= hist.protFloor)),
    stat('Avg fiber', s.avgFiber === null ? '—' : `${n1(s.avgFiber)} g`, `${s.fiberDays}/${food} days ≥ ${n0(hist.fiberFloor)} g`, s.avgFiber === null ? '' : tone(s.avgFiber >= hist.fiberFloor)),
    stat('Medicines', s.adherence === null ? '—' : `${n0(s.adherence)}%`, 'of doses taken', s.adherence === null ? '' : tone(s.adherence >= 90)),
    stat('Avg sleep', s.avgSleepMins === null ? '—' : `${n1(s.avgSleepMins / 60)} h`, s.avgSleepMins === null ? 'log it on the Log' : `bed ${s.avgBed} · up ${s.avgWake}`, s.avgSleepMins === null ? '' : tone(s.avgSleepMins >= 420)),
    stat('Trained', `${s.daysTrained} days`, `of ${hist.days.length}`),
    stat('Days logged', String(s.daysUsed), `of ${hist.days.length}`),
  );

  const kcal = barChart({
    title: 'Calories per day',
    data: days.map((d) => ({ label: short(d.date), v: d.kcal, tone: d.kcal === 0 ? 'muted' : d.over ? 'over' : '', tip: `${dayLabel(d.date, state.today)}: ${n0(d.kcal)} kcal of ${n0(d.target)}` })),
    ref: hist.kcalTarget,
    refLabel: `${n0(hist.kcalTarget)} kcal target`,
  });
  const prot = barChart({
    title: 'Protein per day',
    data: days.map((d) => ({ label: short(d.date), v: d.prot, tone: d.prot === 0 ? 'muted' : d.protMet ? 'prot' : 'prot short', tip: `${dayLabel(d.date, state.today)}: ${n1(d.prot)} g protein` })),
    ref: hist.protFloor,
    refLabel: `${n0(hist.protFloor)} g floor`,
  });
  const fiber = barChart({
    title: 'Fiber per day',
    data: days.map((d) => ({ label: short(d.date), v: d.fiber || 0, tone: d.fiber === null ? 'muted' : d.fiberMet ? 'fiber' : 'fiber short', tip: d.fiber === null ? `${dayLabel(d.date, state.today)}: no fiber data` : `${dayLabel(d.date, state.today)}: ${n1(d.fiber)} g fiber` })),
    ref: hist.fiberFloor,
    refLabel: `${n0(hist.fiberFloor)} g floor`,
  });
  const train = barChart({
    title: 'Training minutes per day',
    height: 110,
    data: days.map((d) => ({ label: short(d.date), v: d.mins, tone: 'train', tip: `${dayLabel(d.date, state.today)}: ${d.mins ? `${n0(d.mins)} min` : 'no training'}` })),
  });

  const sleepChart = barChart({
    title: 'Hours slept per night',
    height: 120,
    data: days.map((d) => ({ label: short(d.date), v: d.sleep ? d.sleep.mins / 60 : 0, tone: d.sleep ? (d.sleep.mins >= 420 ? 'sleep' : 'sleep short') : 'muted', tip: d.sleep ? `${dayLabel(d.date, state.today)}: ${fmtMins(d.sleep.mins)} (${d.sleep.bed}–${d.sleep.wake})` : `${dayLabel(d.date, state.today)}: not logged` })),
    ref: 7,
    refLabel: '7 h',
  });
  const nutrients = nutrientGroups(s.nutrients, 'No nutrient data yet. Foods you add from the food database carry 30–40 nutrients each.');

  const meds = hist.meds.length
    ? pctBars(hist.meds.map((m) => ({ label: m.name, value: m.due ? (m.taken / m.due) * 100 : 0, text: `${m.taken}/${m.due}`, tone: m.missed === 0 ? 'ok' : m.missed * 5 > m.due ? 'short' : 'mid' })), { max: 100, title: 'Medicines taken' })
    : h('p', { class: 'muted small', text: 'No medicines due in this period.' });

  const dayList = h('details', { class: 'card' },
    h('summary', {}, h('b', { text: 'Day by day' }), h('span', { class: 'small muted', text: ' · tap a day to see all its nutrients' })),
    h('div', { class: 'list' }, hist.days.map(dayRow)));

  fill(
    els.body,
    tiles,
    h('div', { class: 'cols' },
      h('div', { class: 'col' }, insights, h('div', { class: 'card', 'aria-label': 'Nutrients' }, h('div', { class: 'row spread wrap' }, h('h2', { text: 'Nutrients' }), h('span', { class: 'small muted', text: 'daily average vs reference · ≈ some foods lack data · food only' })), nutrients), h('div', { class: 'card', 'aria-label': 'Medicines taken' }, h('h2', { text: 'Medicines taken' }), meds)),
      h('div', { class: 'col' },
        chartCard('Calories', 'red = over target', kcal),
        chartCard('Protein', `line = ${n0(hist.protFloor)} g floor · faded = under it`, prot),
        chartCard('Fiber', `line = ${n0(hist.fiberFloor)} g floor · faded = under it`, fiber),
        chartCard('Training', 'minutes per day', train),
        chartCard('Sleep', 'hours per night · line = 7 h', sleepChart))),
    dayList,
  );
}

/** Nutrient bars by group (target, limit), as % of the daily reference. Fiber has its own chart. */
function nutrientGroups(list, empty) {
  const groups = ['Carbs', 'Fats', 'Vitamins', 'Minerals', 'Other'];
  const known = list.filter((m) => m.pct !== null && m.kind !== 'info' && m.key !== 'fiber');
  if (!known.length) return h('p', { class: 'muted small', text: empty });
  return h('div', { class: 'nut-groups' }, groups.map((g) => {
    const rows = known.filter((m) => m.group === g);
    if (!rows.length) return null;
    return h('div', { class: 'nut-group' }, h('h3', { text: g }),
      pctBars(rows.map((m) => ({
        label: m.label,
        value: m.pct,
        text: `${m.coverage < 0.8 ? '≈' : ''}${n0(m.pct)}% · ${n1(m.amount)} ${m.unit}`,
        tone: m.kind === 'limit' ? (m.pct > 100 ? 'short' : m.pct > 80 ? 'mid' : 'ok') : m.pct < 60 ? 'short' : m.pct < 100 ? 'mid' : 'ok',
      })), { title: g }));
  }));
}

/** One day: its totals, and tapping it opens that day's nutrients right underneath. */
function dayRow(d) {
  const panel = h('div', { class: 'day-nutrients hidden', id: `day-nut-${d.date}`, role: 'region', 'aria-label': `Nutrients on ${dayLabel(d.date, state.today)}` });
  let loaded = false;
  const btn = h('button', {
    type: 'button', class: 'item ghost day-row', 'data-hist': d.date, 'aria-expanded': 'false', 'aria-controls': panel.id,
    // The label is all a screen reader hears, so it carries the numbers too.
    'aria-label': [dayLabel(d.date, state.today), `${n0(d.kcal)} kcal`, `${n1(d.prot)} g protein`, d.fiber === null ? null : `${n1(d.fiber)} g fiber`, d.sleep ? `slept ${fmtMins(d.sleep.mins)}` : null, d.mins ? `${n0(d.mins)} min trained` : null, d.due ? `meds ${d.taken} of ${d.due}` : null].filter(Boolean).join(', '),
    onclick: async () => {
      const open = panel.classList.toggle('hidden') === false;
      btn.setAttribute('aria-expanded', String(open));
      if (!open || loaded) return;
      fill(panel, h('p', { class: 'small muted', text: 'Loading…' }));
      const day = await call('get_day', { date: d.date });
      loaded = true;
      fill(panel,
        d.sleep ? h('p', { class: 'small day-sleep', text: `Sleep: in bed ${d.sleep.bed}, up ${d.sleep.wake} · ${fmtMins(d.sleep.mins)}` }) : null,
        nutrientGroups(day.nutrients, 'No nutrient data for this day. Foods from the food database carry 30–40 nutrients each; typed-in numbers only have calories, protein and fiber.'));
    },
  },
  h('span', { class: 'day-chevron', 'aria-hidden': 'true' }),
  h('span', { class: 'name' }, h('b', { text: dayLabel(d.date, state.today) }), h('div', { class: 'meta', text: [d.sleep ? `slept ${fmtMins(d.sleep.mins)}` : null, d.mins ? `${n0(d.mins)} min trained` : null, d.due ? `meds ${d.taken}/${d.due}` : null].filter(Boolean).join(' · ') })),
  h('span', { class: `num small ${d.over ? 'error' : ''}`, text: `${n0(d.kcal)} kcal` }),
  h('span', { class: `num small ${d.protMet ? 'ok' : 'muted'}`, text: `${n1(d.prot)} g` }),
  h('span', { class: `num small ${d.fiberMet ? 'ok' : 'muted'}`, text: d.fiber === null ? '—' : `${n1(d.fiber)} g fiber` }));
  return h('div', { class: 'day-entry' }, btn, panel);
}

export default { id: 'history', title: 'History', icon: 'chart', mount, refresh };
