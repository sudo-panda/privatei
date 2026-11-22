import { h, fill, n0, dayLabel } from '../dom.js';
import { call } from '../api.js';
import { state } from '../state.js';
import { openTraining } from './today.js';
import { barChart } from '../charts.js';

const KIND = { gym: 'Gym', run: 'Run', walk: 'Walk', cycle: 'Cycle', swim: 'Swim', other: 'Other' };

/** Training log: every session, newest first, with add and paste import. */
export function trainingPanel() {
  let list;
  const p = {
    id: 'training', title: 'Training', icon: 'run',
    mount(root) {
      list = h('div', { class: 'list' });
      root.append(
        h('div', { class: 'card', 'aria-label': 'Training log' },
          h('div', { class: 'card-head' }, h('h2', { text: 'Training log' }),
            h('button', { type: 'button', class: 'ghost', onclick: () => { state.date = state.today; openTraining(); } }, 'Add training')),
          list),
      );
    },
    async refresh() {
      const rows = await call('recent_workouts', { limit: 60 });
      let last = '';
      fill(
        list,
        rows.length
          ? rows.flatMap((w) => {
              const head = w.date !== last ? h('div', { class: 'item list-date' }, h('h3', { text: dayLabel(w.date, state.today) })) : null;
              last = w.date;
              return [head, h('button', { type: 'button', class: 'item ghost row-edit', 'aria-label': `Edit ${w.name}`, onclick: () => openTraining(w) },
                h('span', { class: 'name' }, h('div', { text: w.name }), h('div', { class: 'meta', text: `${KIND[w.kind] || w.kind} · ${n0(w.mins)} min` })),
                h('span', { class: 'num small', text: `${n0(w.kcal)} kcal` }),
                h('span', { class: 'edit-hint', 'aria-hidden': 'true', text: 'Edit' }))];
            })
          : h('p', { class: 'muted', text: 'No training logged yet.' }),
      );
    },
  };
  return p;
}

/** This week at a glance: sessions, minutes, burn, and the last four weeks. */
export function trainingSummaryPanel() {
  let box;
  return {
    id: 'training-summary', title: 'This week', icon: 'run',
    mount(root) { box = h('div', { class: 'stack' }); root.append(box); },
    async refresh() {
      const t = await call('training_summary');
      const last = t.lastDaysAgo === null ? 'No sessions yet' : t.lastDaysAgo === 0 ? 'Trained today' : t.lastDaysAgo === 1 ? 'Last session yesterday' : `Last session ${t.lastDaysAgo} days ago`;
      const total = t.byKind.reduce((a, [, m]) => a + m, 0) || 1;
      fill(
        box,
        h('div', { class: 'stat-grid' },
          h('div', { class: 'stat-tile' }, h('span', { class: 'small muted', text: 'Sessions, last 7 days' }), h('b', { text: String(t.weekSessions) })),
          h('div', { class: 'stat-tile' }, h('span', { class: 'small muted', text: 'Minutes' }), h('b', { text: n0(t.weekMins) })),
          h('div', { class: 'stat-tile' }, h('span', { class: 'small muted', text: 'Burned' }), h('b', { text: `${n0(t.weekKcal)} kcal` })),
          h('div', { class: `stat-tile ${t.lastDaysAgo !== null && t.lastDaysAgo >= 4 ? 'warn' : ''}` }, h('span', { class: 'small muted', text: 'Last session' }), h('b', { class: 'small-b', text: last }))),
        h('div', { class: 'card', 'aria-label': 'Last four weeks' },
          h('div', { class: 'row spread wrap' }, h('h2', { text: 'Last four weeks' }), h('span', { class: 'small muted', text: t.byKind.map(([k, m]) => `${KIND[k] || k} ${Math.round((m / total) * 100)}%`).join(' · ') })),
          barChart({ title: 'Training minutes, last 28 days', height: 120, data: t.daily.map(([d, m]) => ({ label: `${Number(d.slice(8))}.${Number(d.slice(5, 7))}`, v: m, tone: 'train', tip: `${dayLabel(d, state.today)}: ${m ? `${n0(m)} min` : 'rest'}` })) })),
      );
    },
  };
}
