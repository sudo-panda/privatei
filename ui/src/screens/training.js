import { h, fill, iconButton, n0, dayLabel } from '../dom.js';
import { call } from '../api.js';
import { state, changed } from '../state.js';
import { openTraining } from './today.js';

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
              return [head, h('div', { class: 'item' },
                h('span', { class: 'name' }, h('div', { text: w.name }), h('div', { class: 'meta', text: `${KIND[w.kind] || w.kind} · ${n0(w.mins)} min` })),
                h('span', { class: 'num small', text: `${n0(w.kcal)} kcal` }),
                iconButton('trash', `Delete ${w.name}`, async () => { await call('delete_workout', { id: w.id }); changed(); }))];
            })
          : h('p', { class: 'muted', text: 'No training logged yet.' }),
      );
    },
  };
  return p;
}
