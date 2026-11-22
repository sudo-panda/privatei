// Search the food databases and pick an amount. Used on the Log and in the Plan editor.
import { h, fill, n0, n1, chipGroup } from './dom.js';
import { quiet } from './api.js';

const SOURCE = { table: 'Built-in', mine: 'Learned', usda: 'USDA', swiss: 'Swiss' };

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
        ? hits.slice(0, 12).map((f) =>
            h('button', { type: 'button', class: 'food-hit', role: 'option', 'data-ref': f.ref, 'aria-label': `${f.name}, ${SOURCE[f.source] || f.sourceName}`, onclick: () => pick(f) },
              h('span', { class: 'name' }, h('span', { text: f.name }), h('span', { class: 'meta', text: [f.category, `${f.known} nutrients`].filter(Boolean).join(' · ') })),
              h('span', { class: `badge src-${f.source}`, text: SOURCE[f.source] || f.sourceName }),
              h('span', { class: 'num small muted', text: `${n0(f.kcal100)} kcal · ${n1(f.prot100)} g /100 g` })))
        : h('p', { class: 'small muted', text: 'Nothing found. Try fewer or different words, or describe it below and the AI will work it out.' }),
    );
  };
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
