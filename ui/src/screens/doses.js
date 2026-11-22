import { h, fill } from '../dom.js';
import { call } from '../api.js';
import { state, changed } from '../state.js';

/** Today's doses with ticks (the schedules live below them on the Meds tab). */
export function dosesPanel() {
  let box;
  return {
    id: 'doses', title: 'Today', icon: 'meds',
    mount(root) { box = h('div', { class: 'card', 'aria-label': 'Doses today' }); root.append(box); },
    async refresh() {
      const d = await call('get_day', { date: state.today });
      const taken = d.meds.filter((m) => m.taken).length;
      fill(
        box,
        h('div', { class: 'row spread' }, h('h2', { text: 'Today' }), h('span', { class: 'small muted', text: d.meds.length ? `${taken} of ${d.meds.length} taken` : '' })),
        d.meds.length
          ? h('div', { class: 'list' }, d.meds.map((m) =>
              h('label', { class: 'item', 'data-med': m.medId },
                h('input', { type: 'checkbox', checked: m.taken, 'aria-label': `Took ${m.name}`, onchange: async () => { try { await call('toggle_dose', { date: d.date, medId: m.medId }); } finally { changed(); } } }),
                h('span', { class: 'name' }, h('div', { text: m.name }), h('div', { class: 'meta', text: [m.dose, m.time].filter(Boolean).join(' · ') })),
                m.status === 'overdue' ? h('span', { class: 'badge warn', text: 'overdue' }) : null)))
          : h('p', { class: 'muted', text: 'Nothing due today.' }),
      );
    },
  };
}
