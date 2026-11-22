import { h, fill, iconButton, toast, n0, n1, dayLabel, chipGroup } from '../dom.js';
import { call, quiet, message } from '../api.js';
import { state, go, changed } from '../state.js';
import { foodSearch } from '../foodsearch.js';

const MEALS = ['Breakfast', 'Lunch', 'Snack', 'Dinner'];
let els = {};
let draft = null; // { items, unknown, note, providerText, error }

let mealPreset = null; // set when you open Add food from a meal on the Log

function guessMeal() {
  if (mealPreset) return mealPreset;
  const d = new Date();
  const m = d.getHours() * 60 + d.getMinutes();
  return m < 630 ? 'Breakfast' : m < 870 ? 'Lunch' : m < 1050 ? 'Snack' : 'Dinner';
}

function mount(root, o = {}) {
  els.compact = !!o.compact;
  els.onLogged = o.onLogged || (() => {});
  els.target = h('div', { class: 'small muted', 'data-testid': 'log-target' });
  els.recent = h('div', { class: 'chips', 'aria-label': 'Recent foods' });
  const recentCard = h('div', { class: 'card' }, h('div', { class: 'row spread' }, h('h3', { text: 'Your usual foods' }), els.target), els.recent);

  // Describe it
  const text = h('textarea', { id: 'describe', 'aria-label': 'Describe what you ate', placeholder: 'e.g. 200g chicken breast, 2 eggs and 1 banana', maxlength: 2000 });
  els.status = h('div', { class: 'small muted', role: 'status' });
  const work = h('button', { type: 'button', class: 'primary', id: 'work-out' }, 'Work out the calories');
  work.addEventListener('click', async () => {
    if (!text.value.trim()) return toast('Describe what you ate first.', 'error');
    work.disabled = true;
    els.status.textContent = 'Working it out…';
    try {
      const v = await quiet('estimate_food', { text: text.value });
      setDraft(withPreset(v));
      els.status.textContent = '';
    } catch (e) {
      els.status.textContent = '';
      toast(message(e), 'error');
    } finally {
      work.disabled = false;
    }
  });
  const photoInput = h('input', { type: 'file', accept: 'image/*', class: 'sr-only', id: 'photo', 'aria-label': 'Choose a meal photo' });
  const photoBtn = h('button', { type: 'button', onclick: () => photoInput.click() }, 'Photo — uses AI');
  photoInput.addEventListener('change', async () => {
    const f = photoInput.files && photoInput.files[0];
    photoInput.value = '';
    if (!f) return;
    if (f.size > 15 * 1024 * 1024) return toast('That photo is too large (over 15 MB).', 'error');
    photoBtn.disabled = true;
    els.status.textContent = 'Looking at the photo…';
    try {
      const data = await readAsDataUrl(f);
      const ext = (f.name.split('.').pop() || 'jpg').toLowerCase();
      const v = await quiet('estimate_photo', { data, ext, note: text.value });
      setDraft(withPreset(v));
    } catch (e) {
      toast(message(e), 'error');
    } finally {
      els.status.textContent = '';
      photoBtn.disabled = false;
    }
  });
  const describeCard = h(
    'div',
    { class: 'card' },
    h('h2', { text: 'Describe it' }),
    text,
    h('div', { class: 'grid2' }, work, photoBtn),
    photoInput,
    els.status,
  );

  els.draft = h('div', { class: 'card hidden', id: 'draft', 'aria-label': 'Draft' });

  const search = (els.search = foodSearch({
    id: 'food-search',
    meal: guessMeal,
    confirmLabel: 'Add to log',
    onPick: async (f, grams, meal) => {
      try {
        await quiet('add_food', { date: state.date, meal: meal || '', reference: f.ref, grams });
      } catch (e) {
        toast(message(e), 'error');
        return false;
      }
      toast(`Logged ${f.name}`);
      changed();
      els.onLogged();
    },
  }));
  if (o.compact) {
    // Inline on a day: one "Add food" card; typing numbers by hand is tucked away.
    const manual = manualCard();
    manual.classList.remove('card');
    manual.querySelector('h2').remove();
    root.append(
      h('div', { class: 'card add-inline', 'aria-label': 'Add food' },
        h('div', { class: 'row spread' }, h('h2', { text: 'Add food' }), els.target),
        search,
        h('div', { class: 'small muted', text: 'Your usual foods — tap to add to the draft:' }),
        els.recent,
        h('div', { class: 'small muted', text: 'Or describe it — known foods are worked out on your Mac, the rest by AI:' }),
        text, h('div', { class: 'grid2' }, work, photoBtn), photoInput, els.status,
        h('details', { class: 'set-more' }, h('summary', { class: 'small', text: 'Type the numbers myself' }), manual)),
      els.draft,
    );
    return;
  }
  root.append(h('div', { class: 'cols' }, h('div', { class: 'col' }, h('div', { class: 'card' }, h('h2', { text: 'Search foods' }), search), describeCard, els.draft), h('div', { class: 'col' }, recentCard, manualCard())));
}

function readAsDataUrl(file) {
  return new Promise((res, rej) => {
    const r = new FileReader();
    r.onload = () => res(String(r.result));
    r.onerror = () => rej(new Error('That photo could not be read.'));
    r.readAsDataURL(file);
  });
}

/** Opened from a meal on the Log: everything you describe goes into that meal. */
function withPreset(v) {
  if (mealPreset && v && v.items) for (const it of v.items) it.meal = mealPreset;
  return v;
}

function setDraft(v) {
  draft = v;
  renderDraft();
}

function renderDraft() {
  if (!draft) {
    els.draft.classList.add('hidden');
    return;
  }
  els.draft.classList.remove('hidden');
  const total = draft.items.reduce((a, i) => [a[0] + i.kcal, a[1] + i.prot], [0, 0]);
  fill(
    els.draft,
    h('div', { class: 'card-head' }, h('h2', { text: 'Draft' }), draft.providerText ? h('span', { class: 'badge', 'data-testid': 'provider', text: draft.providerText }) : null),
    draft.items.length
      ? h(
          'div',
          { class: 'list' },
          draft.items.map((it, i) =>
            h(
              'div',
              { class: 'item', 'data-draft': i },
              h('span', { class: 'name' }, h('div', { text: it.name }), h('div', { class: 'meta', text: `${it.meal}${it.est ? ' · estimated weight' : ''}` })),
              h('span', { class: 'num small', text: `${n0(it.kcal)} · ${n1(it.prot)} g` }),
              draft.provider
                ? h('button', { type: 'button', class: 'ghost small-btn', 'aria-label': `Find ${it.name} as a packaged product`, title: 'Look it up on Open Food Facts', onclick: () => els.search.searchOnline(it.name.split(' · ')[0]) }, 'Packaged?')
                : null,
              iconButton('close', `Remove ${it.name}`, () => {
                draft.items.splice(i, 1);
                renderDraft();
              }),
            ),
          ),
        )
      : h('p', { class: 'muted', text: 'Nothing placed yet.' }),
    draft.unknown.length
      ? h('div', { class: 'small', 'data-testid': 'unknown' }, h('b', { text: "Couldn't place: " }), draft.unknown.join(', '),
          h('div', { class: 'chips', style: 'margin-top:6px' }, draft.unknown.map((u) =>
            h('button', { type: 'button', class: 'chip', 'aria-label': `Find ${u} as a packaged product`, onclick: () => els.search.searchOnline(u) }, h('span', { text: `Find “${u}” as a packaged product` })))))
      : null,
    draft.error ? h('p', { class: 'small error', text: draft.error }) : null,
    draft.note ? h('p', { class: 'small muted', text: draft.note }) : null,
    h('div', { class: 'row spread' }, h('b', { text: 'Total' }), h('span', { class: 'num', 'data-testid': 'draft-total', text: `${n0(total[0])} kcal · ${n1(total[1])} g protein` })),
    h(
      'div',
      { class: 'grid2' },
      h('button', { type: 'button', onclick: () => setDraft(null) }, 'Discard'),
      h('button', {
        type: 'button',
        class: 'primary',
        disabled: draft.items.length === 0,
        onclick: async () => {
          const n = await call('save_items', { date: state.date, items: draft.items });
          toast(`Saved ${n} item${n === 1 ? '' : 's'}`);
          setDraft(null);
          document.getElementById('describe').value = '';
          changed();
          els.onLogged();
        },
      }, 'Save'),
    ),
  );
}

function manualCard() {
  let meal = guessMeal();
  let edited = { kcal: false, prot: false };
  const list = h('datalist', { id: 'food-names' });
  const name = h('input', { type: 'text', id: 'm-name', list: 'food-names', maxlength: 80, autocomplete: 'off', 'aria-label': 'Food name' });
  const grams = h('input', { type: 'number', id: 'm-grams', min: 0, inputmode: 'decimal', 'aria-label': 'Grams' });
  const kcal = h('input', { type: 'number', id: 'm-kcal', min: 0, max: 5000, inputmode: 'decimal', 'aria-label': 'kcal' });
  const prot = h('input', { type: 'number', id: 'm-prot', min: 0, max: 500, inputmode: 'decimal', 'aria-label': 'Protein (g)' });
  const hint = h('div', { class: 'small muted' });
  const err = h('div', { class: 'error', role: 'alert' });
  kcal.addEventListener('input', () => (edited.kcal = kcal.value !== ''));
  prot.addEventListener('input', () => (edited.prot = prot.value !== ''));
  let seq = 0;
  const autofill = async () => {
    const my = ++seq;
    const g = grams.value === '' ? null : Number(grams.value);
    const q = name.value.trim() ? await quiet('quick_fill', { name: name.value, grams: g }).catch(() => null) : null;
    if (my !== seq) return;
    if (!q) {
      hint.textContent = '';
      return;
    }
    hint.textContent = `From the food table: ${q.matched}, ${n1(q.grams)} g`;
    if (!edited.kcal) kcal.value = n0(q.kcal);
    if (!edited.prot) prot.value = n1(q.prot);
  };
  name.addEventListener('input', autofill);
  grams.addEventListener('input', autofill);
  const chips = chipGroup(MEALS.map((m) => [m, m]), meal, (v) => (meal = v), 'Meal');
  els.manualMeal = (v) => { meal = v; chips.setValue(v); };
  const save = h('button', {
    type: 'button',
    class: 'primary',
    onclick: async () => {
      err.textContent = '';
      const k = Number(kcal.value);
      const p = prot.value === '' ? 0 : Number(prot.value);
      if (!name.value.trim()) return (err.textContent = 'Give the food a name.');
      if (!(k > 0 && k < 5000)) return (err.textContent = 'kcal must be above 0 and below 5000.');
      if (!(p >= 0 && p <= 500)) return (err.textContent = 'Protein must be between 0 and 500 g.');
      const g = grams.value === '' ? null : Number(grams.value);
      try {
        await quiet('add_log', { date: state.date, entry: { meal, name: name.value.trim(), grams: g, kcal: k, prot: p, est: false } });
      } catch (e) {
        err.textContent = message(e);
        return;
      }
      toast(`Logged ${name.value.trim()}`);
      for (const i of [name, grams, kcal, prot]) i.value = '';
      edited = { kcal: false, prot: false };
      hint.textContent = '';
      changed();
      els.onLogged();
    },
  }, 'Log it');
  els.names = list;
  return h(
    'div',
    { class: 'card', id: 'manual' },
    h('h2', { text: 'Type the numbers myself' }),
    h('label', { class: 'field' }, 'Name', name),
    list,
    h('div', { class: 'grid3' }, h('label', { class: 'field' }, 'Grams', grams), h('label', { class: 'field' }, 'kcal', kcal), h('label', { class: 'field' }, 'Protein g', prot)),
    hint,
    chips,
    err,
    save,
  );
}

async function refresh(opts = {}) {
  if (!state.date) state.date = state.today;
  els.target.textContent = `Logging to ${dayLabel(state.date, state.today)}`;
  const [recent, names] = await Promise.all([call('recent_foods'), opts.background ? null : call('food_names')]);
  fill(
    els.recent,
    recent.length
      ? recent.map((r) =>
          h('button', {
            type: 'button',
            class: 'chip',
            'aria-label': `Add ${r.name} to the draft`,
            onclick: () => {
              // Into the draft, next to anything you described; Save logs it all at once.
              const d = draft || { items: [], unknown: [], note: '', provider: null, providerText: null, error: null };
              d.items.push({ name: r.name, grams: r.grams ?? null, kcal: r.kcal, prot: r.prot, est: r.est, meal: guessMeal(), fiber: r.fiber ?? null, nut: r.nut ?? null });
              setDraft(d);
              els.draft.scrollIntoView({ block: 'nearest' });
            },
          }, h('span', { text: r.name })),
        )
      : h('span', { class: 'muted small', text: 'Foods you log show up here.' }),
  );
  if (names) fill(els.names, names.map((n) => h('option', { value: n })));
}

/** Add to this meal (from a meal on the Log), or null to guess from the time of day. */
function setMeal(m) {
  mealPreset = MEALS.includes(m) ? m : null;
  if (els.manualMeal) els.manualMeal(guessMeal());
}

export default { id: 'add', title: 'Add food', short: 'Add', icon: 'add', mount, refresh, go, setMeal };
