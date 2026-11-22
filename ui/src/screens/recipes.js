import { h, fill, toast, openSheet, confirmSheet, iconButton, n0, n1 } from '../dom.js';
import { call, quiet, message } from '../api.js';
import { foodSearch } from '../foodsearch.js';

/** Recipes: dishes built from ingredients; they show up in food search like any food. */
export function recipesPanel() {
  let list;
  const p = {
    id: 'recipes', title: 'Recipes', icon: 'book',
    mount(root) {
      list = h('div', { class: 'masonry' });
      root.append(
        h('div', { class: 'toolbar' }, h('button', { type: 'button', class: 'primary', onclick: () => editRecipe(null, p.refresh) }, 'New recipe')),
        list,
      );
    },
    async refresh() {
      const rs = await call('get_recipes');
      fill(
        list,
        rs.length
          ? rs.map((r) =>
              h('button', { type: 'button', class: 'card recipe-card', 'aria-label': `Edit ${r.name}`, onclick: () => editRecipe(r, p.refresh) },
                h('div', { class: 'row spread' }, h('h2', { text: r.name }), h('span', { class: 'small muted', text: `${n0(r.servings)} serving${r.servings === 1 ? '' : 's'}` })),
                h('div', { class: 'num small', text: `${n0(r.perServing.kcal)} kcal · ${n1(r.perServing.prot)} g protein${r.perServing.fiber != null ? ` · ${n1(r.perServing.fiber)} g fiber` : ''} per serving (${n0(r.perServing.grams)} g)` }),
                h('div', { class: 'small muted ellipsis', text: r.ingredients.map((i) => i.name.split(',')[0]).join(', ') })))
          : h('div', { class: 'card' }, h('p', { class: 'muted', text: 'No recipes yet. Make one for dishes you cook often (dal, your usual bowl…) and log a serving in one tap.' })),
      );
    },
  };
  return p;
}

function editRecipe(r, done) {
  const state = {
    name: r ? r.name : '',
    servings: r ? r.servings : 4,
    cookedGrams: r ? r.cookedGrams : null,
    note: r ? r.note : '',
    ingredients: r ? r.ingredients.map((i) => ({ ...i })) : [],
  };
  openSheet(r ? `Edit ${r.name}` : 'New recipe', (body, close) => {
    const name = h('input', { type: 'text', id: 'rc-name', maxlength: 80, value: state.name, 'aria-label': 'Recipe name', placeholder: 'e.g. Masoor dal' });
    const servings = h('input', { type: 'number', id: 'rc-servings', min: 1, max: 100, value: state.servings, 'aria-label': 'Servings' });
    const cooked = h('input', { type: 'number', id: 'rc-cooked', min: 0, value: state.cookedGrams ?? '', 'aria-label': 'Cooked weight in grams', placeholder: 'optional' });
    const list = h('div', { class: 'list', id: 'rc-ingredients' });
    const totals = h('div', { class: 'recipe-totals num', 'data-testid': 'recipe-totals' });
    const err = h('div', { class: 'error', role: 'alert' });
    const input = () => ({ name: name.value, servings: Number(servings.value) || 0, cookedGrams: cooked.value === '' ? null : Number(cooked.value), ingredients: state.ingredients, note: state.note });
    let seq = 0;
    const preview = async () => {
      const my = ++seq;
      if (!state.ingredients.length) return fill(totals, h('span', { class: 'muted small', text: 'Add ingredients to see the totals.' }));
      const v = await quiet('recipe_preview', { recipe: { ...input(), name: name.value || 'Recipe', servings: Number(servings.value) || 1 } }).catch(() => null);
      if (!v || my !== seq) return;
      fill(totals,
        h('div', {}, h('span', { class: 'muted small', text: 'Per serving ' }), h('b', { text: `${n0(v.perServing.kcal)} kcal · ${n1(v.perServing.prot)} g protein${v.perServing.fiber != null ? ` · ${n1(v.perServing.fiber)} g fiber` : ''}` }), h('span', { class: 'muted small', text: ` (${n0(v.perServing.grams)} g)` })),
        h('div', { class: 'small muted', text: `Whole recipe: ${n0(v.total.kcal)} kcal · ${n1(v.total.prot)} g protein · ${n0(v.total.grams)} g` }));
    };
    const draw = () => {
      fill(list, state.ingredients.length
        ? state.ingredients.map((i, idx) =>
            h('div', { class: 'item' },
              h('span', { class: 'name' }, h('div', { text: i.name }), h('div', { class: 'meta', text: `${n0(i.grams)} g` })),
              h('span', { class: 'num small', text: `${n0(i.kcal)} kcal · ${n1(i.prot)} g` }),
              iconButton('close', `Remove ${i.name}`, () => { state.ingredients.splice(idx, 1); draw(); })))
        : h('p', { class: 'muted small', text: 'No ingredients yet.' }));
      preview();
    };
    for (const el of [servings, cooked]) el.addEventListener('input', preview);
    const search = foodSearch({
      id: 'rc-food',
      confirmLabel: 'Add ingredient',
      onPick: async (f, grams) => {
        try {
          state.ingredients.push(await quiet('ingredient', { reference: f.ref, grams }));
          draw();
        } catch (e) {
          toast(message(e), 'error');
          return false;
        }
      },
    });
    body.append(
      h('label', { class: 'field' }, 'Name', name),
      h('div', { class: 'grid2' }, h('label', { class: 'field' }, 'Servings', servings), h('label', { class: 'field' }, 'Cooked weight (g)', cooked)),
      h('div', { class: 'small muted', text: 'Weigh the finished pot if you can — water and cooking change the weight, and portions are worked out from it.' }),
      h('h3', { text: 'Ingredients' }),
      list,
      search,
      totals,
      err,
      h('div', { class: 'actions' },
        r ? h('button', { type: 'button', class: 'danger', onclick: async () => {
          if (!(await confirmSheet('Delete recipe?', `Delete ${r.name}? Food you already logged from it stays.`, 'Delete'))) return;
          await call('delete_recipe', { id: r.id });
          close();
          done();
        } }, 'Delete') : null,
        h('button', { type: 'button', class: 'primary', onclick: async () => {
          err.textContent = '';
          try {
            await quiet('save_recipe', { id: r ? r.id : null, recipe: input() });
          } catch (e) {
            return (err.textContent = message(e));
          }
          toast('Recipe saved');
          close();
          done();
        } }, 'Save recipe')),
    );
    draw();
  });
}
