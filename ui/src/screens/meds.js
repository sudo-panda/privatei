import { h, fill, toast, openSheet, confirmSheet, chipGroup, weekdayShort } from '../dom.js';
import { call, quiet, message } from '../api.js';

let els = {};

const FREQ = [
  ['daily', 'Every day'],
  ['weekly', 'Certain days'],
  ['monthly', 'Monthly'],
];

function schedule(m) {
  if (m.freq === 'weekly') return m.days.map((d) => weekdayShort[d]).join(', ');
  if (m.freq === 'monthly') return `Day ${m.dom} of the month`;
  return 'Every day';
}

function mount(root) {
  els.add = h('button', { type: 'button', class: 'primary', onclick: () => openEditor(null) }, 'Add medicine');
  els.list = h('div', { class: 'masonry' });
  root.append(h('div', { class: 'toolbar' }, els.add), els.list);
}

function section(title, meds, stopped = false) {
  if (!meds.length) return null;
  return h(
    'div',
    { class: 'card', 'aria-label': title },
    h('h3', { text: title }),
    h(
      'div',
      { class: 'list' },
      meds.map((m) =>
        h(
          'button',
          { type: 'button', class: 'item ghost', style: 'text-align:left;border-radius:0;min-height:48px', 'aria-label': `Edit ${m.name}`, onclick: () => openEditor(m, stopped) },
          h('span', { class: 'name' }, h('div', { text: m.name }), h('div', { class: 'meta', text: [m.dose, stopped ? `stopped ${m.until || ''}` : schedule(m)].filter(Boolean).join(' · ') })),
          m.time ? h('span', { class: 'num small', text: m.time }) : null,
        ),
      ),
    ),
  );
}

async function refresh() {
  const v = await call('get_meds');
  const any = v.daily.length + v.days.length + v.monthly.length + v.stopped.length;
  fill(
    els.list,
    any ? null : h('div', { class: 'card' }, h('p', { class: 'muted', text: 'No medicines yet. Add one and it shows up on Today when it is due.' })),
    section('Every day', v.daily),
    section('Certain days', v.days),
    section('Monthly', v.monthly),
    section('No longer taking', v.stopped, true),
  );
}

function openEditor(m, stopped = false) {
  const orig = m ? { freq: m.freq, days: [...m.days], dom: m.dom, time: m.time } : null;
  let freq = m ? m.freq : 'daily';
  let days = m ? [...m.days] : [];
  openSheet(m ? 'Edit medicine' : 'New medicine', (body, close) => {
    const name = h('input', { type: 'text', id: 'med-name', maxlength: 80, value: m ? m.name : '', 'aria-label': 'Name' });
    const dose = h('input', { type: 'text', id: 'med-dose', maxlength: 40, value: m ? m.dose : '', 'aria-label': 'Dose', placeholder: 'e.g. 1 tablet' });
    const dom = h('input', { type: 'number', id: 'med-dom', min: 1, max: 31, value: m && m.dom ? m.dom : '', 'aria-label': 'Day of month' });
    const time = h('input', { type: 'time', id: 'med-time', value: m ? m.time : '', 'aria-label': 'Time' });
    const note = h('div', { class: 'small badge warn hidden', style: 'white-space:normal;padding:8px 10px;border-radius:10px', 'data-testid': 'new-schedule-note' });
    const err = h('div', { class: 'error', role: 'alert' });
    const wk = h('div', { class: 'weekdays', role: 'group', 'aria-label': 'Weekdays' });
    weekdayShort.forEach((d, i) =>
      wk.append(
        h('button', {
          type: 'button',
          'aria-pressed': String(days.includes(i)),
          'aria-label': d,
          onclick: (e) => {
            days = days.includes(i) ? days.filter((x) => x !== i) : [...days, i].sort();
            e.currentTarget.setAttribute('aria-pressed', String(days.includes(i)));
            updateNote();
          },
        }, d),
      ),
    );
    const wkWrap = h('div', {}, wk);
    const domWrap = h('label', { class: 'field' }, 'Day of the month', dom);
    const showFreq = () => {
      wkWrap.classList.toggle('hidden', freq !== 'weekly');
      domWrap.classList.toggle('hidden', freq !== 'monthly');
    };
    const updateNote = () => {
      if (!orig || stopped) return note.classList.add('hidden');
      const d = dom.value ? Number(dom.value) : null;
      const changedSched = orig.freq !== freq || (freq === 'weekly' && orig.days.join() !== days.join()) || (freq === 'monthly' && orig.dom !== d) || (orig.time || '') !== time.value;
      note.textContent = 'Changing the schedule starts a new one from today. Earlier days keep the old schedule.';
      note.classList.toggle('hidden', !changedSched);
    };
    dom.addEventListener('input', updateNote);
    time.addEventListener('input', updateNote);
    const chips = chipGroup(FREQ, freq, (v) => { freq = v; showFreq(); updateNote(); }, 'Frequency');
    showFreq();

    const save = async () => {
      err.textContent = '';
      const input = {
        groupId: m ? m.groupId : null,
        name: name.value,
        dose: dose.value,
        freq,
        days: freq === 'weekly' ? days : [],
        dom: freq === 'monthly' && dom.value ? Number(dom.value) : null,
        time: time.value,
      };
      if (!input.name.trim()) return (err.textContent = 'Give the medicine a name.');
      try {
        const r = await quiet('save_med', { med: input });
        toast(r.startedNew ? 'Saved — new schedule from today' : 'Saved');
        close();
        refresh();
      } catch (e) {
        err.textContent = message(e);
      }
    };
    body.append(
      h('label', { class: 'field' }, 'Name', name),
      h('label', { class: 'field' }, 'Dose', dose),
      h('div', { class: 'field' }, h('span', { text: 'How often' }), chips),
      wkWrap,
      domWrap,
      h('label', { class: 'field' }, 'Time (optional)', time),
      note,
      err,
      h('button', { type: 'button', class: 'primary', onclick: save }, stopped ? 'Start taking again' : 'Save'),
      m
        ? h(
            'div',
            { class: 'actions' },
            stopped
              ? null
              : h('button', {
                  type: 'button',
                  onclick: async () => {
                    await call('stop_med', { groupId: m.groupId });
                    toast(`Stopped ${m.name}`);
                    close();
                    refresh();
                  },
                }, 'Stop taking'),
            h('button', {
              type: 'button',
              class: 'danger',
              onclick: async () => {
                const ok = await confirmSheet('Erase medicine?', `This deletes ${m.name}, its whole schedule history and every tick. It can't be undone.`, 'Erase');
                if (!ok) return;
                await call('erase_med', { groupId: m.groupId });
                toast(`Erased ${m.name}`);
                close();
                refresh();
              },
            }, 'Erase'),
          )
        : null,
    );
  });
}

export default { id: 'meds', title: 'Medicines', short: 'Meds', icon: 'meds', mount, refresh };
