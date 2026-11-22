// Search the food databases and pick an amount. Used on the Log and in the Plan editor.
import { h, fill, n0, n1, chipGroup } from './dom.js';
import { quiet } from './api.js';

const SOURCE = { table: 'Built-in', mine: 'Learned', recipe: 'Recipe', usda: 'USDA', cofid: 'UK', swiss: 'Swiss', bls: 'German', off: 'Product' };

/**
 * opts.onPick(food, grams, meal) — called when you confirm an amount.
 * opts.confirmLabel — button text; opts.meal — show meal chips with this default.
 */
export function foodSearch(opts) {
  const input = h('input', { type: 'search', class: 'food-q', id: opts.id || 'food-search', placeholder: 'Search foods — e.g. lentils, gruyère, chapati', autocomplete: 'off', 'aria-label': 'Search foods' });
  const results = h('div', { class: 'food-results hidden', role: 'listbox', 'aria-label': 'Food results' });
  const picked = h('div', { class: 'food-picked hidden' });
  let seq = 0;
  let timer = null;
  const run = async () => {
    const my = ++seq;
    const q = input.value.trim();
    if (q.length < 2) {
      results.classList.add('hidden');
      return;
    }
    const hits = await quiet('search_foods', { query: q }).catch(() => []);
    if (my !== seq) return;
    results.classList.remove('hidden');
    fill(
      results,
      hits.length
        ? hits.slice(0, 12).map(hitButton)
        : h('p', { class: 'small muted', text: 'Nothing found in your databases. Try fewer words, look it up online below, or describe it and the AI will work it out.' }),
      opts.online === false ? null : onlineButton(q),
    );
  };

  /** Look packaged products up on Open Food Facts (online; the results are kept for offline use). */
  function onlineButton(q) {
    const status = h('span', { class: 'small muted', role: 'status' });
    const btn = h('button', { type: 'button', class: 'ghost', id: `${opts.id || 'food-search'}-online` }, /^\d{8,}$/.test(q) ? `Look up barcode ${q} online` : 'Search packaged products online');
    btn.addEventListener('click', async () => {
      btn.disabled = true;
      status.textContent = 'Asking Open Food Facts…';
      try {
        const found = await quiet('search_off', { query: q });
        if (!found.length) {
          status.textContent = 'No packaged products found.';
          btn.disabled = false;
          return;
        }
        fill(results, found.map(hitButton), h('p', { class: 'small muted', text: 'From Open Food Facts (ODbL). Saved so they work offline next time.' }));
      } catch (e) {
        status.textContent = String(e && e.message ? e.message : e);
        btn.disabled = false;
      }
    });
    return h('div', { class: 'row wrap online-row' }, btn, status);
  }

  function hitButton(f) {
    return h('button', { type: 'button', class: 'food-hit', role: 'option', 'data-ref': f.ref, 'aria-label': `${f.name}, ${SOURCE[f.source] || f.sourceName}`, onclick: () => pick(f) },
      h('span', { class: 'name' }, h('span', { text: f.name }), h('span', { class: 'meta', text: [f.category, `${f.known} nutrients`].filter(Boolean).join(' · ') })),
      h('span', { class: `badge src-${f.source}`, text: SOURCE[f.source] || f.sourceName }),
      h('span', { class: 'num small muted', text: `${n0(f.kcal100)} kcal · ${n1(f.prot100)} g /100 g` }));
  }
  input.addEventListener('input', () => {
    clearTimeout(timer);
    timer = setTimeout(run, 120);
  });

  function pick(f) {
    results.classList.add('hidden');
    picked.classList.remove('hidden');
    let meal = opts.meal || null;
    const piece = f.portions.find((p) => p.label === '1 piece');
    const grams = h('input', { type: 'number', id: `${opts.id || 'food-search'}-grams`, min: 1, max: 5000, value: piece ? piece.grams : 100, 'aria-label': 'Grams' });
    const preview = h('div', { class: 'num small', 'data-testid': 'food-preview' });
    // Display only: the real numbers are computed by the core when you add it.
    const show = () => {
      const k = (Number(grams.value) || 0) / 100;
      preview.textContent = `${n0(f.kcal100 * k)} kcal · ${n1(f.prot100 * k)} g protein${f.fiber100 != null ? ` · ${n1(f.fiber100 * k)} g fiber` : ''}`;
    };
    grams.addEventListener('input', show);
    show();
    const portions = [{ label: '100 g', grams: 100 }, ...f.portions].slice(0, 7);
    fill(
      picked,
      h('div', { class: 'row spread wrap' }, h('b', { text: f.name }), h('span', { class: `badge src-${f.source}`, text: f.sourceName })),
      h('div', { class: 'chips', role: 'group', 'aria-label': 'Portions' }, portions.map((p) =>
        h('button', { type: 'button', class: 'chip', onclick: () => { grams.value = p.grams; show(); } }, h('span', { text: `${p.label}${p.label.endsWith(' g') ? '' : ` (${n0(p.grams)} g)`}` })))),
      h('div', { class: 'row' }, h('label', { class: 'field grow' }, 'Grams', grams), preview),
      opts.meal ? chipGroup(['Breakfast', 'Lunch', 'Snack', 'Dinner'].map((m) => [m, m]), meal, (v) => (meal = v), 'Meal') : null,
      h('div', { class: 'grid2' },
        h('button', { type: 'button', onclick: reset }, 'Cancel'),
        h('button', { type: 'button', class: 'primary', onclick: async () => { const g = Number(grams.value); if (!(g > 0 && g <= 5000)) return grams.focus(); if ((await opts.onPick(f, g, meal)) !== false) reset(); } }, opts.confirmLabel || 'Add')),
    );
    grams.focus();
    grams.select();
  }

  function reset() {
    picked.classList.add('hidden');
    fill(picked);
    input.value = '';
    results.classList.add('hidden');
  }

  return h('div', { class: 'food-search' }, input, results, picked);
}
