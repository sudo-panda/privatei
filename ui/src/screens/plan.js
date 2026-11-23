import { h, fill, icon, iconButton, toast, openSheet, overlay, n0, n1, dayLabel, chipGroup } from '../dom.js';
import { call, quiet, message } from '../api.js';
import { state, go } from '../state.js';
import { planPanel } from './week.js';
import { foodSearch } from '../foodsearch.js';

const SLOTS = ['Breakfast', 'Lunch', 'Snack', 'Dinner'];
let els = {};
let span = 7;
let earlier = null;

function title(d) {
  if (d.offset === 0) return 'Today';
  if (d.offset === 1) return 'Tomorrow';
  return dayLabel(d.date, state.today);
}

function mount(root) {
  els.head = h('div', { class: 'card plan-head' });
  els.board = h('div', { class: 'plan-board' });
  els.earlier = h('div', { class: 'hidden' });
  root.append(els.head, els.board, els.earlier);
  // Like Add food on the Log: one panel to fill the coming days from the food databases.
  els.addOverlay = overlay('Add to plan');
  els.addOverlay.body.append(addPanel());
  root.classList.add('has-fab');
  root.append(h('button', { type: 'button', class: 'fab', id: 'open-plan-add', 'aria-label': 'Add food to the plan', title: 'Add food to the plan', onclick: () => openAddToPlan() }, icon('add')));
}

/** Open the Add to plan panel, on `date` if given. */
export async function openAddToPlan(date = null) {
  await els.addPanel.load(date);
  els.addOverlay.open('#plan-search');
}

/**
 * Search foods and recipes and add them to a day and meal of the next two weeks.
 * The panel stays open after each add, so a whole week can be filled in one go.
 */
function addPanel() {
  let days = [];
  let date = null;
  let slot = 'Lunch';
  const dayChips = h('div', { class: 'chips plan-day-chips', role: 'group', 'aria-label': 'Day' });
  const summary = h('div', { class: 'small muted', 'data-testid': 'plan-add-day' });
  const status = h('div', { class: 'small ok', role: 'status' });
  const firstOpenSlot = () => days.find((d) => d.date === date)?.emptySlots[0] || slot;
  const showDay = () => {
    for (const b of dayChips.children) b.setAttribute('aria-pressed', String(b.dataset.v === date));
    const d = days.find((x) => x.date === date);
    summary.textContent = d ? (d.meals.length ? `${title(d)}: ${d.meals.length} planned · ${n0(d.kcal)} kcal${d.emptySlots.length ? ` · still open: ${d.emptySlots.join(', ')}` : ''}` : `${title(d)}: nothing planned yet`) : '';
  };
  const search = foodSearch({
    id: 'plan-search',
    meal: () => (slot = firstOpenSlot()),
    confirmLabel: 'Add to plan',
    onPick: async (f, g, meal) => {
      try {
        await quiet('plan_food', { date, slot: meal || slot, reference: f.ref, grams: g });
      } catch (e) {
        toast(message(e), 'error');
        return false;
      }
      const d = days.find((x) => x.date === date);
      status.textContent = `Added ${f.name} to ${d ? title(d) : date} · ${meal || slot}`;
      refresh(); // also reloads this panel's day summary
      return true;
    },
  });
  const typeIt = h('button', { type: 'button', class: 'ghost small-btn', onclick: () => {
    const d = days.find((x) => x.date === date);
    if (d) editMeal(d, firstOpenSlot());
  } }, 'Type a meal yourself');
  const panel = h('div', { class: 'stack plan-add' },
    h('div', { class: 'field' }, h('span', { text: 'Day' }), dayChips),
    summary,
    search,
    status,
    typeIt);
  panel.load = async (want) => {
    const p = await call('get_plan_ahead', { days: 14 });
    days = p.days;
    // The day you asked for, else the one already picked, else the first with a gap.
    date = want || (days.some((d) => d.date === date) ? date : (days.find((d) => d.offset > 0 && d.emptySlots.length) || days[0]).date);
    fill(dayChips, days.map((d) => h('button', { type: 'button', class: 'chip', 'data-v': d.date, 'aria-pressed': String(d.date === date),
      onclick: () => { date = d.date; status.textContent = ''; showDay(); } },
      d.offset === 0 ? 'Today' : d.offset === 1 ? 'Tomorrow' : dayLabel(d.date, ''))));
    showDay();
  };
  els.addPanel = panel;
  return panel;
}

async function refresh() {
  const p = await call('get_plan_ahead', { days: span });
  const unplanned = p.days.filter((d) => d.meals.length === 0).length;
  const seg = h('div', { class: 'segment', role: 'group', 'aria-label': 'How far ahead' },
    [7, 14].map((n) => h('button', { type: 'button', 'aria-pressed': String(span === n), onclick: () => { span = n; refresh(); } }, `${n} days`)));
  fill(
    els.head,
    h('div', { class: 'row spread wrap' },
      h('div', {},
        h('h2', { text: `Next ${span} days` }),
        h('div', { class: 'small muted', 'data-testid': 'plan-summary', text: p.plannedDays
          ? `${p.plannedDays} planned${unplanned ? `, ${unplanned} still open` : ''} · avg ${n0(p.avgKcal)} kcal · ${n1(p.avgProt)} g protein`
          : 'Nothing planned yet. Add meals to a day below.' })),
      h('div', { class: 'row wrap plan-tools' },
        seg,
        h('button', {
          type: 'button',
          onclick: async () => {
            try {
              const n = await quiet('repeat_last_week');
              toast(n ? `Copied last week into ${n} day${n === 1 ? '' : 's'}` : 'Those days already have a plan');
            } catch (e) {
              toast(message(e), 'error');
            }
            refresh();
          },
        }, 'Repeat last week'),
        p.hasEarlier ? h('button', { type: 'button', class: 'ghost', onclick: toggleEarlier }, els.earlier.classList.contains('hidden') ? 'Earlier days' : 'Hide earlier days') : null)),
  );
  fill(els.board, p.days.map((d) => dayCard(d, p)));
  if (els.addOverlay?.isOpen()) els.addPanel.load();
  if (!els.earlier.classList.contains('hidden') && earlier) await earlier.refresh();
}

function toggleEarlier() {
  const hidden = els.earlier.classList.toggle('hidden');
  if (!hidden && !earlier) {
    earlier = planPanel();
    fill(els.earlier, h('h2', { class: 'set-section-title', text: 'Earlier days' }));
    earlier.mount(els.earlier);
  }
  refresh();
}

function dayCard(d, p) {
  const pct = Math.min(100, (d.kcal / Math.max(1, p.kcalTarget)) * 100);
  return h(
    'article',
    { class: `card plan-day ${d.isToday ? 'is-today' : ''} ${d.meals.length ? '' : 'empty'}`, 'data-plan-day': d.date, 'aria-label': title(d) },
    h('div', { class: 'row spread' },
      h('div', {}, h('h2', { class: 'plan-day-title', text: title(d) }), d.offset > 1 ? null : h('div', { class: 'small muted', text: dayLabel(d.date, '') })),
      d.meals.length ? h('span', { class: `num small ${d.over ? 'error' : ''}`, text: `${n0(d.kcal)} kcal` }) : h('span', { class: 'badge warn', text: 'Open' })),
    d.meals.length
      ? h('div', { class: 'stack' },
          h('div', { class: `bar thin ${d.over ? 'over' : ''}`, 'aria-label': 'Planned calories against target' }, h('span', { style: `width:${pct}%` })),
          h('div', { class: 'row small muted spread' },
            h('span', { class: d.protMet ? 'ok' : '', text: `${n1(d.prot)} g protein` }),
            h('span', { text: `${n1(d.fiber)} g fiber` })))
      : null,
    d.shop ? shopTile(d) : null,
    h('div', { class: 'plan-slots' }, SLOTS.map((slot) => {
      const meals = d.meals.filter((m) => m.slot === slot);
      return h('div', { class: 'plan-slot' },
        h('div', { class: 'row spread' },
          h('span', { class: 'slot-name', text: slot }),
          iconButton('plus', `Add ${slot.toLowerCase()} on ${title(d)}`, () => editMeal(d, slot))),
        meals.map((m) => h('button', { type: 'button', class: 'plan-meal ghost row-edit', 'aria-label': `Edit ${m.name}`, onclick: () => editMeal(d, slot, m) },
          h('span', { class: 'name' }, h('span', { text: m.name }), m.done ? h('span', { class: 'badge good', text: 'eaten' }) : null),
          h('span', { class: 'num small muted', text: `${n0(m.kcal)} kcal · ${n1(m.prot)} g` }),
          h('span', { class: 'edit-hint', 'aria-hidden': 'true', text: 'Edit' }))));
    })),
    // Not a shopping day yet: a quiet button at the foot of the card.
    d.shop ? null : h('button', { type: 'button', class: 'ghost small-btn plan-shop-add', 'aria-label': `Make ${title(d)} a shopping day`, onclick: () => pickTrip(d) }, icon('shop'), 'Shopping day'),
  );
}

/** A planned shopping day: the trip, what's left to buy, and a way to open it. */
function shopTile(d) {
  const s = d.shop;
  return h('div', { class: 'shop-tile', 'data-shop-day': d.date },
    h('span', { class: 'shop-tile-icon', 'aria-hidden': 'true' }, icon('shop')),
    h('span', { class: 'name' },
      h('b', { text: `Shopping · Trip ${s.trip}` }),
      h('div', { class: 'meta', text: s.items ? `${s.items} item${s.items === 1 ? '' : 's'} to buy${s.cost ? ` · CHF ${s.cost.toFixed(2)}` : ''}` : 'List is empty' })),
    h('button', { type: 'button', class: 'small-btn', onclick: () => go('kitchen', { part: 'Shopping', trip: s.trip }) }, 'Open trip'),
    iconButton('close', `No shopping on ${title(d)}`, async () => { await call('set_shop_day', { date: d.date, trip: null }); refresh(); }));
}

/** Which trip are you shopping for on this day? */
function pickTrip(d) {
  openSheet(`Shopping on ${title(d)}`, (body, close) => {
    const pick = async (trip) => {
      await call('set_shop_day', { date: d.date, trip });
      toast(`${title(d)} is a shopping day`);
      close();
      refresh();
    };
    body.append(
      h('p', { class: 'small muted', text: 'Which list are you shopping for? The day shows it on the Plan, and on the Log when the day comes.' }),
      h('div', { class: 'actions' },
        h('button', { type: 'button', onclick: () => pick(1) }, 'Trip 1'),
        h('button', { type: 'button', onclick: () => pick(2) }, 'Trip 2')));
  });
}

/** Add a meal to a slot, or edit/delete `existing`. */
function editMeal(d, slot, existing = null) {
  openSheet(existing ? `Edit ${existing.name}` : `Plan ${slot.toLowerCase()} · ${title(d)}`, (body, close) => {
    let s = slot;
    const name = h('input', { type: 'text', id: 'pm-name', maxlength: 80, list: 'food-names', 'aria-label': 'Meal', placeholder: 'e.g. Tofu bowl', value: existing ? existing.name : '' });
    const kcal = h('input', { type: 'number', id: 'pm-kcal', min: 0, max: 5000, 'aria-label': 'kcal', value: existing ? existing.kcal : '' });
    const prot = h('input', { type: 'number', id: 'pm-prot', min: 0, max: 500, 'aria-label': 'Protein g', value: existing ? existing.prot : '' });
    const fiber = h('input', { type: 'number', id: 'pm-fiber', min: 0, max: 300, 'aria-label': 'Fiber g', value: existing && existing.fiber != null ? existing.fiber : '' });
    const hint = h('div', { class: 'small muted' });
    const err = h('div', { class: 'error', role: 'alert' });
    let edited = !!existing;
    for (const i of [kcal, prot, fiber]) i.addEventListener('input', () => (edited = true));
    name.addEventListener('input', async () => {
      const q = name.value.trim() ? await quiet('quick_fill', { name: name.value, grams: null }).catch(() => null) : null;
      hint.textContent = q ? `From the food table: ${q.matched}, ${n1(q.grams)} g` : '';
      if (q && !edited) {
        kcal.value = n0(q.kcal);
        prot.value = n1(q.prot);
        fiber.value = q.fiber === null || q.fiber === undefined ? '' : n1(q.fiber);
      }
    });
    const search = foodSearch({
      id: 'plan-food',
      confirmLabel: 'Use this',
      onPick: (f, g) => {
        const k = g / 100;
        name.value = `${f.name} · ${Math.round(g)} g`;
        kcal.value = Math.round(f.kcal100 * k);
        prot.value = Math.round(f.prot100 * k * 10) / 10;
        fiber.value = f.fiber100 == null ? '' : Math.round(f.fiber100 * k * 10) / 10;
        edited = true;
        hint.textContent = `From ${f.sourceName}`;
      },
    });
    body.append(
      h('div', { class: 'field' }, h('span', { text: 'Meal' }), chipGroup(SLOTS.map((x) => [x, x]), s, (v) => (s = v), 'Meal')),
      existing ? null : search,
      h('label', { class: 'field' }, 'What', name),
      h('div', { class: 'grid3' }, h('label', { class: 'field' }, 'kcal', kcal), h('label', { class: 'field' }, 'Protein g', prot), h('label', { class: 'field' }, 'Fiber g', fiber)),
      hint,
      err,
      h('button', {
        type: 'button',
        class: 'primary',
        onclick: async () => {
          err.textContent = '';
          const k = Number(kcal.value), pr = Number(prot.value || 0);
          if (!name.value.trim()) return (err.textContent = 'Name the meal.');
          if (!(k > 0 && k < 5000)) return (err.textContent = 'kcal must be above 0 and below 5000.');
          if (!(pr >= 0 && pr <= 500)) return (err.textContent = 'Protein must be between 0 and 500 g.');
          const meal = { slot: s, name: name.value.trim(), kcal: k, prot: pr, fiber: fiber.value === '' ? null : Number(fiber.value), done: false };
          try {
            if (existing) await quiet('update_plan_meal', { date: d.date, idx: existing.idx, name: existing.name, meal });
            else await quiet('add_plan_meal', { date: d.date, meal });
          } catch (e) {
            return (err.textContent = message(e));
          }
          close();
          refresh();
        },
      }, existing ? 'Save' : 'Add to plan'),
      existing
        ? h('button', { type: 'button', class: 'danger', onclick: async () => { await call('remove_plan_meal', { date: d.date, idx: existing.idx, name: existing.name }); toast('Removed from the plan'); close(); refresh(); } }, 'Remove from plan')
        : null,
    );
  });
}

export default { id: 'plan', title: 'Plan', icon: 'week', mount, refresh };
