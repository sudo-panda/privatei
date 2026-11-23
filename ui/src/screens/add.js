import { h, fill, iconButton, toast, openSheet, n0, n1, dayLabel, chipGroup } from '../dom.js';
import { call, quiet, message } from '../api.js';
import { state, go, changed } from '../state.js';
import { foodSearch } from '../foodsearch.js';

const MEALS = ['Breakfast', 'Lunch', 'Snack', 'Dinner'];
let els = {};
let draft = null; // { items, unknown, note, providerText, error }

function guessMeal() {
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
      setDraft(v);
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
      setDraft(v);
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
  const total = h('span', { class: 'num', 'data-testid': 'draft-total' });
  const showTotal = () => {
    // Display only: each row's numbers come from the core (database or AI estimate).
    const t = draft.items.reduce((a, i) => [a[0] + i.kcal, a[1] + i.prot, a[2] + (i.fiber || 0)], [0, 0, 0]);
    total.textContent = `${n0(t[0])} kcal · ${n1(t[1])} g protein · ${n1(t[2])} g fiber`;
  };
  fill(
    els.draft,
    h('div', { class: 'card-head' }, h('h2', { text: 'Draft' }), draft.providerText ? h('span', { class: 'badge', 'data-testid': 'provider', text: draft.providerText }) : null),
    draft.items.length
      ? h('div', { class: 'draft-rows' }, h('div', { class: 'draft-hint small muted', text: 'Check each line: change the food, the amount or the meal before saving.' }), draft.items.map((it, i) => draftRow(it, i, showTotal)))
      : h('p', { class: 'muted', text: 'Nothing placed yet.' }),
    draft.unknown.length
      ? h('div', { class: 'small', 'data-testid': 'unknown' }, h('b', { text: "Couldn't place: " }), draft.unknown.join(', '),
          h('div', { class: 'chips', style: 'margin-top:6px' }, draft.unknown.map((u) =>
            h('button', { type: 'button', class: 'chip', 'aria-label': `Find ${u} as a packaged product`, onclick: () => els.search.searchOnline(u) }, h('span', { text: `Find “${u}” as a packaged product` })))))
      : null,
    draft.error ? h('p', { class: 'small error', text: draft.error }) : null,
    draft.note ? h('p', { class: 'small muted', text: draft.note }) : null,
    h('div', { class: 'row spread draft-total' }, h('b', { text: 'Total' }), total),
    h(
      'div',
      { class: 'grid2' },
      h('button', { type: 'button', onclick: () => setDraft(null) }, 'Discard'),
      h('button', {
        type: 'button',
        class: 'primary',
        disabled: draft.items.length === 0,
        onclick: async () => {
          const items = draft.items.map(({ alts, portions, sourceName, ...item }) => item);
          const n = await call('save_items', { date: state.date, items });
          toast(`Saved ${n} item${n === 1 ? '' : 's'}`);
          setDraft(null);
          document.getElementById('describe').value = '';
          changed();
          els.onLogged();
        },
      }, 'Save'),
    ),
  );
  showTotal();
}

/**
 * One draft line, like a food diary entry: the food (tap to change it), amount and unit
 * (grams or the food's own portions), meal, and its numbers. Changing anything asks the
 * core for new numbers and updates just this line, so the boxes keep their focus.
 */
function draftRow(it, i, onChange) {
  const row = h('div', { class: 'draft-row', 'data-draft': i });
  const base = (name) => name.split(' · ')[0];
  const numbers = h('span', { class: 'num small draft-num' });
  const err = h('div', { class: 'small error', role: 'alert' });
  const amount = h('input', { type: 'number', class: 'draft-amount', min: 0, step: 'any', inputmode: 'decimal', 'aria-label': `Amount of ${base(it.name)}` });
  const unit = h('select', { class: 'draft-unit', 'aria-label': `Unit for ${base(it.name)}` });
  const mealSel = h('select', { class: 'draft-meal', 'aria-label': `Meal for ${base(it.name)}` }, MEALS.map((m) => h('option', { value: m, text: m })));
  const foodBtn = h('button', { type: 'button', class: 'ghost draft-food', 'aria-label': `Change food: ${base(it.name)}`, title: 'Change the food', onclick: () => chooseFood(i) });
  const source = h('span', { class: 'badge draft-source' });
  const guessed = h('span', { class: 'small muted draft-guessed' });
  const show = () => {
    const cur = draft.items[i];
    foodBtn.replaceChildren(h('span', { text: base(cur.name) }));
    source.textContent = cur.foodRef ? cur.sourceName || 'Database' : 'AI estimate';
    source.classList.toggle('warn', !cur.foodRef);
    numbers.textContent = `${n0(cur.kcal)} kcal · ${n1(cur.prot)} g protein${cur.fiber != null ? ` · ${n1(cur.fiber)} g fiber` : ''}`;
    // Units: grams, plus the food's own portions (1 cup, 1 piece…).
    const units = [['g', 'g', 1], ...(cur.portions || []).map((p, k) => [`p${k}`, `${p.label} (${n0(p.grams)} g)`, p.grams])];
    let keep = unit.value;
    if (!keep && cur.grams) {
      // First time: "2 eggs" reads better as 2 × 1 piece than 120 g.
      const fit = units.slice(1).find(([, , g]) => { const n = cur.grams / g; return n >= 0.5 && n <= 20 && Math.abs(n * 2 - Math.round(n * 2)) < 0.01; });
      if (fit) keep = fit[0];
    }
    fill(unit, units.map(([v, t]) => h('option', { value: v, text: t })));
    unit.value = units.some(([v]) => v === keep) ? keep : 'g';
    unit.units = units;
    if (document.activeElement !== amount) {
      const per = units.find(([v]) => v === unit.value)[2];
      amount.value = cur.grams ? String(Math.round((cur.grams / per) * 100) / 100) : '';
    }
    mealSel.value = cur.meal;
    guessed.textContent = cur.est ? 'amount guessed — check it' : '';
    onChange();
  };
  const regrams = async () => {
    err.textContent = '';
    const per = unit.units.find(([v]) => v === unit.value)[2];
    const g = Number(amount.value) * per;
    if (!(g > 0 && g <= 5000)) return (err.textContent = 'Amount must come to between 1 and 5000 g.');
    const cur = draft.items[i];
    try {
      const next = cur.foodRef
        ? await quiet('draft_item', { reference: cur.foodRef, grams: g, meal: cur.meal })
        : { ...cur, ...(await quiet('rescale_item', { item: plain(cur), grams: g })) };
      // You set the amount yourself now, so it's no longer a guess.
      draft.items[i] = { ...next, est: false, alts: cur.alts, meal: cur.meal };
    } catch (e) {
      return (err.textContent = message(e));
    }
    show();
  };
  amount.addEventListener('change', regrams);
  unit.addEventListener('change', () => {
    // Switching unit: 1 of a portion, or the current grams in g.
    const cur = draft.items[i];
    amount.value = unit.value === 'g' ? String(cur.grams || 100) : '1';
    regrams();
  });
  mealSel.addEventListener('change', () => {
    draft.items[i].meal = mealSel.value;
  });
  row.append(
    h('div', { class: 'draft-line1' }, foodBtn, source, numbers,
      iconButton('close', `Remove ${base(it.name)}`, () => {
        draft.items.splice(i, 1);
        renderDraft();
      })),
    h('div', { class: 'draft-line2' }, amount, unit, mealSel, guessed),
    err,
  );
  row.refresh = show;
  show();
  return row;
}

/** An item as the core expects it (no draft-only fields). */
function plain({ alts, portions, sourceName, ...item }) {
  return item;
}

/** Pick another food for a draft line: the other matches, a search, or a packaged product. */
function chooseFood(i) {
  const cur = draft.items[i];
  const base = cur.name.split(' · ')[0];
  openSheet(`Choose the food for “${base}”`, (body, close) => {
    const list = h('div', { class: 'food-results' });
    const status = h('div', { class: 'small muted', role: 'status' });
    const pick = async (f) => {
      try {
        const row = await quiet('draft_item', { reference: f.ref, grams: cur.grams || 100, meal: cur.meal });
        draft.items[i] = { ...row, est: cur.est, alts: cur.alts && cur.alts.length ? cur.alts : [] };
      } catch (e) {
        status.textContent = message(e);
        return;
      }
      close();
      renderDraft();
    };
    const choice = (f) => h('button', { type: 'button', class: 'food-hit', 'aria-pressed': String(f.ref === cur.foodRef), onclick: () => pick(f) },
      h('span', { class: 'name' }, h('span', { text: f.name })),
      h('span', { class: `badge src-${f.source || ''}`, text: f.sourceName }));
    const q = h('input', { type: 'search', id: 'draft-food-q', value: base, 'aria-label': 'Search the food databases', autocomplete: 'off' });
    let seq = 0;
    const search = async () => {
      const my = ++seq;
      const hits = q.value.trim().length < 2 ? [] : await quiet('search_foods', { query: q.value }).catch(() => []);
      if (my !== seq) return;
      fill(list, hits.length ? hits.slice(0, 15).map(choice) : h('p', { class: 'small muted', text: 'Nothing found. Try fewer words, or look it up as a packaged product.' }));
    };
    q.addEventListener('input', search);
    const online = h('button', { type: 'button', class: 'ghost small-btn' }, 'Look it up as a packaged product');
    online.addEventListener('click', async () => {
      status.textContent = 'Asking Open Food Facts…';
      try {
        const found = await quiet('search_off', { query: q.value });
        status.textContent = found.length ? '' : 'No packaged products found.';
        if (found.length) fill(list, found.map(choice));
      } catch (e) {
        status.textContent = message(e);
      }
    });
    body.append(
      cur.alts && cur.alts.length ? h('div', { class: 'stack' }, h('h3', { text: 'Other matches' }), h('div', { class: 'food-results' }, cur.alts.map(choice))) : null,
      h('h3', { text: 'Search' }),
      q,
      list,
      h('div', { class: 'row wrap' }, online, status),
    );
    search();
  });
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
            onclick: async () => {
              // Into the draft, next to anything you described; Save logs it all at once.
              const d = draft || { items: [], unknown: [], note: '', provider: null, providerText: null, error: null };
              const fromDb = r.foodRef && r.grams ? await quiet('draft_item', { reference: r.foodRef, grams: r.grams, meal: guessMeal() }).catch(() => null) : null;
              d.items.push(fromDb ? { ...fromDb, alts: [] } : { name: r.name, grams: r.grams ?? null, kcal: r.kcal, prot: r.prot, est: r.est, meal: guessMeal(), fiber: r.fiber ?? null, nut: r.nut ?? null, foodRef: null, alts: [], portions: [] });
              setDraft(d);
              els.draft.scrollIntoView({ block: 'nearest' });
            },
          }, h('span', { text: r.name })),
        )
      : h('span', { class: 'muted small', text: 'Foods you log show up here.' }),
  );
  if (names) fill(els.names, names.map((n) => h('option', { value: n })));
}

export default { id: 'add', title: 'Add food', short: 'Add', icon: 'add', mount, refresh, go };
