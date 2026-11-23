// A small in-memory stand-in for the Rust commands, for UI tests only.
// It is bundled only in `vite build --mode mock`. The real logic lives in privatei-core
// and has its own tests; this only needs to be believable enough to drive every screen.
import { mockIPC } from '@tauri-apps/api/mocks';
import CATALOG from './fixtures/catalog.json';
import USDA_SAMPLE from './fixtures/foods-sample.json';

const params = new URLSearchParams(location.search);
const TODAY = params.get('today') || '2026-11-22';
const HHMM = params.get('hhmm') || '09:00';
const MEALS = ['Breakfast', 'Lunch', 'Snack', 'Dinner'];
const CATS = ['cooked', 'chilled', 'produce', 'shelf', 'freezer'];
const MET = { gym: 5, run: 9.8, walk: 3.5, cycle: 7.5, swim: 7, other: 4 };
const TABLE = { 'chicken breast': [165, 31, 0], egg: [143, 12.6, 55], banana: [89, 1.1, 120], skyr: [63, 11, 0], oats: [370, 13, 0] };

// Fiber and micros per 100 g: [fiber, vitC mg, vitD µg, B12 µg, iron mg, calcium mg]
const EXTRA = {
  skyr: [0, 0, 0, 0.5, 0.1, 120], oats: [10.1, 0, 0, 0, 4.3, 54], 'frozen raspberries': [6.5, 26, 0, 0, 0.7, 25], egg: [0, 0, 2, 0.9, 1.8, 56],
  'wholegrain bread': [6, 0, 0, 0, 2.5, 100], 'chicken breast': [0, 0, 0.1, 0.3, 0.5, 15], 'rice cooked': [0.4, 0, 0, 0, 0.2, 10], 'smoked tofu': [1.5, 0, 0, 0, 2.5, 200],
  'chickpeas drained': [7.6, 1.3, 0, 0, 1.5, 40], 'bell pepper roasted dry': [2, 100, 0, 0, 0.5, 9], 'cottage cheese': [0, 0, 0, 0.4, 0.1, 83], banana: [2.6, 8.7, 0, 0, 0.3, 5],
  apple: [2.4, 4.6, 0, 0, 0.1, 6], 'protein pudding': [0, 0, 0, 0.3, 0.2, 120], erdnussmus: [6, 0, 0, 0, 1.9, 45], 'cooked prawns': [0, 0, 0, 1.5, 0.3, 70],
  'chicken thigh roasted skin-off': [0, 0, 0.1, 0.4, 1, 10], 'firm tofu roasted dry': [2.5, 0, 0, 0, 3, 250], 'potato roasted dry': [2.2, 10, 0, 0, 1, 12],
  'frozen spinach': [3, 2.2, 0, 0, 1.9, 150], roti: [4.5, 0, 0, 0, 2.5, 30], 'mystery dal': [5, 3, 0, 0, 2.5, 40],
};
function nutr(name, grams) {
  const base = name.split(' · ')[0].replace(' (AI)', '').toLowerCase();
  const g = grams || Number((name.match(/· (\d+) g/) || [])[1]) || 300;
  const e = EXTRA[base];
  if (!e) return {};
  const f = g / 100;
  return { fiber: r1(e[0] * f), nut: { vitC: r1(e[1] * f), vitD: r1(e[2] * f), b12: r1(e[3] * f), iron: r1(e[4] * f), calcium: Math.round(e[5] * f), sodium: Math.round(40 * f), satFat: r1(1.2 * f) } };
}
// Same shape as NutView in privatei-core: every catalog nutrient, unknown = pct null.
function nutViews(sum, known, total, fiberRef) {
  return CATALOG.map((d) => {
    const k = known[d.key] || 0;
    const reference = d.key === 'fiber' ? fiberRef : d.reference;
    const amount = sum[d.key] || 0;
    return { ...d, amount: r1(amount), reference, pct: k && reference > 0 ? Math.round((amount / reference) * 100) : null, coverage: total ? Math.round((k / total) * 100) / 100 : 0 };
  });
}
const rowNut = (l) => ({ ...(l.nut || {}), ...(l.fiber != null ? { fiber: l.fiber } : {}) });

let seq = 0;
const id = () => `m${Date.now().toString(36)}${(seq++).toString(36)}`;
const r1 = (x) => Math.round(x * 10) / 10;

function addDays(d, n) {
  const [y, m, dd] = d.split('-').map(Number);
  const t = new Date(Date.UTC(y, m - 1, dd + n));
  return t.toISOString().slice(0, 10);
}
const dow = (d) => new Date(d + 'T12:00:00Z').getUTCDay();
const dom = (d) => Number(d.slice(8, 10));
const cmpDate = (a, b) => (a < b ? -1 : a > b ? 1 : 0);

const db = {
  log: {}, plan: {}, stock: {}, list: {}, meds: {}, medlog: {}, workouts: {}, foods: {},
  recipes: {}, sleep: {},
  rules: '', settings: { addBurn: false, weightKg: 82, kcalTarget: 1500, protFloor: 90, fiberFloor: 30 },
  ai: { order: ['claude', 'gemini'], geminiKey: '', geminiModel: 'gemini-flash-latest', claudeModel: '', claudePath: '' },
  aiAvailable: true,
  online: true,
  swiss: [],
  off: [],
  sources: [{ id: 'cofid', name: 'UK CoFID (McCance and Widdowson)', version: '2021', licence: 'Open Government Licence v3.0', attribution: "Contains public sector information licensed under the Open Government Licence v3.0. McCance and Widdowson's Composition of Foods Integrated Dataset, Public Health England / OHID.", url: 'https://www.gov.uk/government/publications/composition-of-foods-integrated-dataset-cofid', importedAt: 0, foods: 2882 }, { id: 'usda', name: 'USDA FoodData Central', version: 'SR Legacy 2018-04 + Foundation 2026-10', licence: 'Public domain (CC0 1.0)', attribution: 'U.S. Department of Agriculture, Agricultural Research Service. FoodData Central (SR Legacy, Foundation Foods). fdc.nal.usda.gov', url: 'https://fdc.nal.usda.gov/download-datasets/', importedAt: 0, foods: 8237 }],
  ts: 1,
  calls: [],
};

function seed() {
  const s = params.get('seed');
  if (s === 'demo') {
    const y = addDays(TODAY, -1);
    db.log.a = { date: y, meal: 'Breakfast', name: 'Skyr · 200 g', grams: 200, kcal: 126, prot: 22, est: false, ts: db.ts++ };
    db.log.b = { date: y, meal: 'Lunch', name: 'Chicken breast · 150 g', grams: 150, kcal: 248, prot: 46.5, est: false, ts: db.ts++ };
    db.plan[TODAY] = { meals: [{ slot: 'Lunch', name: 'Tofu bowl', kcal: 520, prot: 38, done: false }] };
    db.meds.m1 = { groupId: 'g1', name: 'Vitamin D', dose: '1000 IU', freq: 'daily', days: [], dom: null, time: '08:00', from: addDays(TODAY, -10), until: null };
    db.meds.m2 = { groupId: 'g2', name: 'Magnesium', dose: '1', freq: 'daily', days: [], dom: null, time: '21:00', from: addDays(TODAY, -10), until: null };
    db.stock.s1 = { name: 'Cooked rice', qty: 600, unit: 'g', cat: 'cooked', added: addDays(TODAY, -3), useBy: addDays(TODAY, 1) };
    db.stock.s2 = { name: 'Skyr', qty: 2, unit: 'pots', cat: 'chilled', added: addDays(TODAY, -1), useBy: addDays(TODAY, 10) };
    db.stock.s3 = { name: 'Oats', qty: 1, unit: 'kg', cat: 'shelf', added: addDays(TODAY, -20), useBy: null };
    db.list.l1 = { name: 'Skyr', qty: '4 pots', cat: 'chilled', price: 6.4, trip: 1, checked: false, inStock: false };
    db.list.l2 = { name: 'Bananas', qty: '6', cat: 'produce', price: 2.2, trip: 1, checked: false, inStock: false };
    db.workouts.w1 = { date: y, kind: 'run', mins: 30, kcal: 422, name: 'Lake run', ts: db.ts++ };
    db.foods.f1 = { name: 'Protein pudding', kcal100: 80, prot100: 10, piece: 200, src: 'claude' };
    db.rules = 'No cheese after 20:00';
  }
  if (s === 'rich') seedRich();
  const rows = Number(params.get('rows') || 0);
  for (let i = 0; i < rows; i++) {
    const d = addDays(TODAY, -(i % 60));
    db.log[`r${i}`] = { date: d, meal: MEALS[i % 4], name: `Food ${i % 50}`, grams: 100, kcal: 100 + (i % 7) * 10, prot: 5 + (i % 5), est: i % 3 === 0, ts: db.ts++ };
  }
}

// Three weeks of believable data for looking at layouts.
function seedRich() {
  const B = [['Skyr · 200 g', 126, 22, 'Breakfast'], ['Oats · 50 g', 185, 6.5, 'Breakfast'], ['Frozen raspberries · 100 g', 52, 1.2, 'Breakfast'], ['Egg · 110 g', 157, 13.9, 'Breakfast'], ['Wholegrain bread · 80 g', 195, 6.8, 'Breakfast']];
  const L = [['Chicken breast · 180 g', 297, 55.8, 'Lunch'], ['Rice cooked · 150 g', 195, 4.1, 'Lunch'], ['Smoked tofu · 150 g', 285, 28.5, 'Lunch'], ['Chickpeas drained · 120 g', 180, 10.2, 'Lunch'], ['Bell pepper roasted dry · 160 g', 64, 2.4, 'Lunch'], ['Cottage cheese · 200 g', 196, 22, 'Lunch']];
  const S = [['Banana · 120 g', 107, 1.3, 'Snack'], ['Apple · 180 g', 94, 0.5, 'Snack'], ['Protein pudding · 200 g', 160, 20, 'Snack'], ['Erdnussmus · 15 g', 90, 3.8, 'Snack']];
  const D = [['Cooked prawns · 150 g', 149, 31.5, 'Dinner'], ['Chicken thigh roasted skin-off · 200 g', 370, 48, 'Dinner'], ['Firm tofu roasted dry · 180 g', 342, 37.8, 'Dinner'], ['Potato roasted dry · 250 g', 300, 6.8, 'Dinner'], ['Frozen spinach · 150 g', 38, 4.4, 'Dinner'], ['Roti · 80 g', 240, 8, 'Dinner'], ['Mystery dal (AI)', 410, 21, 'Dinner']];
  const pick = (arr, i, k) => arr[(i * 7 + k * 3) % arr.length];
  for (let i = 1; i <= 21; i++) {
    const d = addDays(TODAY, -i);
    const n = 2 + (i % 3);
    const meals = [pick(B, i, 0), pick(B, i, 1), pick(L, i, 0), pick(L, i, 2), pick(S, i, 0), pick(D, i, 0), pick(D, i, 1)];
    if (i % 4 === 0) meals.push(pick(S, i, 2));
    meals.slice(0, 5 + n).forEach(([name, kcal, prot, meal]) => {
      db.log[`d${i}-${name}`] = { date: d, meal, name, grams: null, kcal, prot, est: name.includes('AI'), ts: db.ts++, ...nutr(name) };
    });
    if (i % 2 === 0) { const k = (i / 2) % 4; db.workouts[`w${i}`] = { date: d, kind: ['gym', 'run', 'walk', 'cycle'][k], mins: [55, 35, 70, 45][k], kcal: [478, 563, 351, 564][k], name: ['Upper body', 'Lake run', 'Uetliberg hike', 'Spin class'][k], ts: db.ts++ }; }
  }
  // Today so far
  db.log.t1 = { date: TODAY, meal: 'Breakfast', name: 'Skyr · 200 g', grams: 200, kcal: 126, prot: 22, est: false, ts: db.ts++, ...nutr('Skyr · 200 g') };
  db.log.t2 = { date: TODAY, meal: 'Breakfast', name: 'Oats · 50 g', grams: 50, kcal: 185, prot: 6.5, est: false, ts: db.ts++, ...nutr('Oats · 50 g') };
  // Plans: last week and the coming days
  const plans = [
    [{ slot: 'Breakfast', name: 'Skyr, oats, raspberries', kcal: 360, prot: 31, fiber: 9 }, { slot: 'Lunch', name: 'Tofu bowl', kcal: 520, prot: 38, fiber: 8 }, { slot: 'Dinner', name: 'Chicken, potatoes, spinach', kcal: 610, prot: 52, fiber: 7 }],
    [{ slot: 'Breakfast', name: 'Eggs on toast', kcal: 390, prot: 21, fiber: 5 }, { slot: 'Lunch', name: 'Chickpea salad', kcal: 450, prot: 22, fiber: 12 }, { slot: 'Snack', name: 'Protein pudding', kcal: 160, prot: 20, fiber: 0 }, { slot: 'Dinner', name: 'Prawns and rice', kcal: 520, prot: 36, fiber: 2 }],
    [{ slot: 'Lunch', name: 'Cottage cheese wrap', kcal: 430, prot: 34, fiber: 6 }, { slot: 'Dinner', name: 'Dal with roti', kcal: 650, prot: 29, fiber: 14 }],
  ];
  for (let i = -6; i <= 4; i++) db.plan[addDays(TODAY, i)] = { meals: plans[(i + 6) % 3].map((m) => ({ ...m, done: false })) };
  db.plan[addDays(TODAY, 2)].shop = 1; // a shopping day this week
  // Medicines
  const from = addDays(TODAY, -30);
  db.meds.m1 = { groupId: 'g1', name: 'Vitamin D3', dose: '1000 IU', freq: 'daily', days: [], dom: null, time: '08:00', from, until: null };
  db.meds.m2 = { groupId: 'g2', name: 'Magnesium', dose: '300 mg', freq: 'daily', days: [], dom: null, time: '21:00', from, until: null };
  db.meds.m3 = { groupId: 'g3', name: 'Vitamin B12', dose: '1 tablet', freq: 'weekly', days: [1, 4], dom: null, time: '', from, until: null };
  db.meds.m4 = { groupId: 'g4', name: 'Iron', dose: '1 tablet', freq: 'monthly', days: [], dom: 1, time: '', from, until: null };
  db.meds.m5 = { groupId: 'g5', name: 'Antihistamine', dose: '10 mg', freq: 'daily', days: [], dom: null, time: '', from: addDays(TODAY, -40), until: addDays(TODAY, -12) };
  for (let i = 1; i <= 21; i++) {
    const d = addDays(TODAY, -i);
    for (const [mid, m] of Object.entries(db.meds)) if (isDue(m, d) && (i * 5 + mid.length) % 7 !== 0) db.medlog[`${d}_${mid}`] = { date: d, medId: mid, at: m.time, ts: db.ts++ };
  }
  db.medlog[`${TODAY}_m1`] = { date: TODAY, medId: 'm1', at: '08:04', ts: db.ts++ };
  // Sleep: two weeks of nights, a couple of late ones.
  for (let i = 1; i <= 14; i++) db.sleep[addDays(TODAY, -i)] = { bed: ['23:15', '22:45', '00:30', '23:40', '23:05', '01:10', '22:50'][i % 7], wake: ['06:50', '06:30', '07:15', '07:00', '06:45', '08:20', '06:40'][i % 7], ts: db.ts++ };
  // Kitchen
  const st = [
    ['Cooked rice', 450, 'g', 'cooked', -3, 1], ['Dal', 600, 'g', 'cooked', -1, 3], ['Roasted peppers', 300, 'g', 'cooked', -4, 0],
    ['Skyr', 3, 'pots', 'chilled', -2, 9], ['Cottage cheese', 1, 'tub', 'chilled', -1, 6], ['Smoked tofu', 2, 'packs', 'chilled', -5, 4], ['Eggs', 8, 'pcs', 'chilled', -6, 14],
    ['Bananas', 4, 'pcs', 'produce', -2, 3], ['Apples', 6, 'pcs', 'produce', -4, 12], ['Cucumber', 1, 'pcs', 'produce', -3, 5],
    ['Oats', 900, 'g', 'shelf', -20, null], ['Chickpeas', 3, 'tins', 'shelf', -15, 400], ['Tinned tomatoes', 4, 'tins', 'shelf', -15, 500], ['Erdnussmus', 1, 'jar', 'shelf', -30, 120], ['Ground flaxseed', 250, 'g', 'shelf', -10, 60],
    ['Frozen spinach', 750, 'g', 'freezer', -12, 150], ['Frozen raspberries', 500, 'g', 'freezer', -8, 200], ['Chicken breast', 4, 'portions', 'freezer', -6, 80],
  ];
  st.forEach(([name, qty, unit, cat, added, use], i) => (db.stock[`s${i}`] = { name, qty, unit, cat, added: addDays(TODAY, added), useBy: use === null ? null : addDays(TODAY, use) }));
  const li = [
    ['Skyr', '4 pots', 'chilled', 6.4, 1], ['Chicken breast', '600 g', 'chilled', 14.5, 1], ['Cottage cheese', '2 tubs', 'chilled', 4.2, 1], ['Bell peppers', '3', 'produce', 3.9, 1],
    ['Bananas', '6', 'produce', 2.2, 1], ['Onions', '1 kg', 'produce', 1.95, 1], ['Rice', '1 kg', 'shelf', 2.6, 1],
    ['Prawns', '400 g', 'freezer', 9.9, 2], ['Firm tofu', '3 packs', 'chilled', 8.7, 2], ['Lemons', '4', 'produce', 2.4, 2], ['Thomy Inferno mustard', '1', 'shelf', 2.95, 2], ['Frozen raspberries', '2 bags', 'freezer', 7.0, 2],
  ];
  li.forEach(([name, qty, cat, price, trip], i) => (db.list[`l${i}`] = { name, qty, cat, price, trip, checked: i === 3, inStock: false }));
  db.foods.f1 = { name: 'Protein pudding', kcal100: 80, prot100: 10, piece: 200, src: 'claude' };
  db.foods.f2 = { name: 'Mystery dal', kcal100: 137, prot100: 7, piece: 0, src: 'claude' };
  db.foods.f3 = { name: 'Haferflocken', kcal100: 375, prot100: 13, piece: 0, src: 'gemini' };
  db.foods.f4 = { name: 'Migros protein bar', kcal100: 360, prot100: 33, piece: 55, src: 'claude' };
  db.rules = 'No cheese after 20:00\nMax 2 eggs a day\nDal counts as 1 roti unless I say otherwise';
  db.settings.addBurn = true;
}

// Food database stand-in: a public-domain USDA sample, plus a few made-up "Swiss" rows
// (the real Swiss database is downloaded by the app and isn't redistributed here).
const SWISS_SAMPLE = [
  { ref: 'swiss:sample-1', name: 'Rösti, prepared (sample row)', source: 'swiss', sourceName: 'Swiss Food Composition Database', category: 'Potatoes', kcal100: 150, prot100: 2, nut100: { fiber: 2, sodium: 300, potassium: 380 }, portions: [] },
  { ref: 'swiss:sample-2', name: 'Birchermüesli (sample row)', source: 'swiss', sourceName: 'Swiss Food Composition Database', category: 'Cereals', kcal100: 120, prot100: 4, nut100: { fiber: 2.5, calcium: 90, vitC: 6 }, portions: [] },
  { ref: 'swiss:sample-3', name: 'Appenzeller cheese (sample row)', source: 'swiss', sourceName: 'Swiss Food Composition Database', category: 'Hard cheese', kcal100: 390, prot100: 25, nut100: { calcium: 800, sodium: 700, satFat: 20 }, portions: [] },
];
// Made-up packaged products standing in for Open Food Facts answers.
const OFF_SAMPLE = [
  { ref: 'off:7610000000011', name: 'Skyr nature (sample brand)', source: 'off', sourceName: 'Open Food Facts', category: 'Packaged product', kcal100: 62, prot100: 11, nut100: { sugar: 4, addedSugar: 0, fat: 0.2, satFat: 0.1, sodium: 40, calcium: 120 }, portions: [{ label: '1 serving', grams: 150 }] },
  { ref: 'off:7610000000028', name: 'Skyr vanilla (sample brand)', source: 'off', sourceName: 'Open Food Facts', category: 'Packaged product', kcal100: 85, prot100: 9.5, nut100: { sugar: 9, addedSugar: 5, fat: 0.2, sodium: 44 }, portions: [{ label: '1 serving', grams: 150 }] },
  { ref: 'off:7610000000035', name: 'Protein bar peanut (sample brand)', source: 'off', sourceName: 'Open Food Facts', category: 'Packaged product', kcal100: 360, prot100: 33, nut100: { sugar: 3, addedSugar: 2, fiber: 15, satFat: 4.5, sodium: 280 }, portions: [{ label: '1 serving', grams: 55 }] },
];
const norm = (t) => t.toLowerCase().normalize('NFD').replace(/[\u0300-\u036f]/g, '');
function builtin() {
  const out = Object.entries(TABLE).map(([name, [k, p, piece]]) => ({ ref: `table:${name}`, name: name[0].toUpperCase() + name.slice(1), source: 'table', sourceName: 'Built-in table', category: '', kcal100: k, prot100: p, nut100: { ...(nutr(name, 100).nut || {}), ...(nutr(name, 100).fiber != null ? { fiber: nutr(name, 100).fiber } : {}) }, portions: piece ? [{ label: '1 piece', grams: piece }] : [] }));
  for (const [fid, f] of Object.entries(db.foods)) out.push({ ref: `mine:${fid}`, name: f.name, source: 'mine', sourceName: 'Learned foods', category: '', kcal100: f.kcal100, prot100: f.prot100, nut100: {}, portions: f.piece ? [{ label: '1 piece', grams: f.piece }] : [] });
  return out;
}
function allFoods() {
  return [...builtin(), ...USDA_SAMPLE, ...db.swiss, ...db.off];
}
function recipeView(rid, r) {
  const raw = r.ingredients.reduce((a, i) => a + i.grams, 0);
  const grams = r.cookedGrams || raw;
  const kcal = r.ingredients.reduce((a, i) => a + i.kcal, 0), prot = r.ingredients.reduce((a, i) => a + i.prot, 0);
  const hasF = r.ingredients.some((i) => i.fiber != null);
  const fiber = hasF ? r.ingredients.reduce((a, i) => a + (i.fiber || 0), 0) : null;
  const sv = r.servings || 1;
  return { id: rid, name: r.name, servings: r.servings, cookedGrams: r.cookedGrams ?? null, note: r.note || '', ingredients: r.ingredients,
    total: { grams: Math.round(grams), kcal: Math.round(kcal), prot: r1(prot), fiber: hasF ? r1(fiber) : null },
    perServing: { grams: Math.round(grams / sv), kcal: Math.round(kcal / sv), prot: r1(prot / sv), fiber: hasF ? r1(fiber / sv) : null }, known: 0 };
}
function recipeFoods() {
  return Object.entries(db.recipes).map(([rid, r]) => {
    const v = recipeView(rid, r);
    const k = 100 / (v.total.grams || 1);
    const nut = {};
    for (const i of r.ingredients) for (const [kk, vv] of Object.entries(i.nut || {})) nut[kk] = (nut[kk] || 0) + vv * k;
    if (v.total.fiber != null) nut.fiber = v.total.fiber * k;
    return { ref: `recipe:${rid}`, name: r.name, source: 'recipe', sourceName: 'My recipes', category: `${r.servings} servings`, kcal100: v.total.kcal * k, prot100: v.total.prot * k, nut100: nut, portions: [{ label: '1 serving', grams: v.perServing.grams }, { label: 'whole recipe', grams: v.total.grams }] };
  });
}
function cleanRecipe(r) {
  if (!r.name || !r.name.trim()) throw 'Give the recipe a name.';
  if (!(r.servings > 0 && r.servings <= 100)) throw 'Servings must be between 1 and 100.';
  if (!r.ingredients || !r.ingredients.length) throw 'Add at least one ingredient.';
  return { name: r.name.trim(), servings: r.servings, cookedGrams: r.cookedGrams || null, ingredients: r.ingredients, note: r.note || '' };
}
function foodByRef(ref) {
  const f = [...recipeFoods(), ...allFoods()].find((x) => x.ref === ref);
  return f ? { ...f, fiber100: f.nut100.fiber ?? null, known: Object.keys(f.nut100).length } : null;
}
function searchFoods(q) {
  const words = norm(q).split(/[^a-z0-9]+/).filter(Boolean);
  if (!words.length) return [];
  const hit = (name) => { const ws = norm(name).split(/[^a-z0-9]+/); return words.every((w) => ws.some((n) => n.startsWith(w))); };
  const score = (f) => { const ws = norm(f.name).split(/[^a-z0-9]+/); return words.filter((w) => ws.includes(w)).length * 2 + (norm(f.name).startsWith(words[0]) ? 3 : 0) - f.name.length / 60; };
  const local = [...recipeFoods(), ...builtin()].filter((f) => hit(f.name)).slice(0, 6);
  const dbHits = [...USDA_SAMPLE, ...db.swiss, ...db.off].filter((f) => hit(f.name) || hit(f.category)).sort((a, b) => score(b) - score(a));
  return [...local, ...dbHits].slice(0, 30).map((f) => ({ ...f, fiber100: f.nut100.fiber ?? null, known: Object.keys(f.nut100).length }));
}

const live = (t) => Object.entries(db[t]);
const meal = (m) => (MEALS.includes(m) ? m : guess(HHMM));
function guess(hhmm) {
  const [h, m] = hhmm.split(':').map(Number);
  const x = h * 60 + m;
  return x < 630 ? 'Breakfast' : x < 870 ? 'Lunch' : x < 1050 ? 'Snack' : 'Dinner';
}
const burn = (kind, mins) => Math.min(5000, Math.round((MET[kind] || 4) * 3.5 * db.settings.weightKg / 200 * mins));
const isDue = (m, d) => m.from <= d && (!m.until || d <= m.until) && (m.freq === 'weekly' ? m.days.includes(dow(d)) : m.freq === 'monthly' ? m.dom === dom(d) : true);
const planIds = (d) => new Set((db.plan[d]?.meals || []).map((m) => m.logId).filter(Boolean));

function getDay({ date }) {
  const ids = planIds(date);
  const logs = live('log').filter(([, l]) => l.date === date).sort((a, b) => a[1].ts - b[1].ts);
  const groups = {};
  let kcal = 0, prot = 0, fiber = 0, fiberKnown = false;
  const msum = {}, known = {};
  for (const [lid, l] of logs) {
    const g = (groups[l.meal] ||= { meal: l.meal, kcal: 0, prot: 0, rows: [] });
    g.kcal += l.kcal; g.prot += l.prot; kcal += l.kcal; prot += l.prot;
    if (l.fiber != null) { fiber += l.fiber; fiberKnown = true; }
    for (const [k, v] of Object.entries(rowNut(l))) { msum[k] = (msum[k] || 0) + v; known[k] = (known[k] || 0) + 1; }
    g.rows.push({ id: lid, name: l.name, grams: l.grams, kcal: l.kcal, prot: l.prot, fiber: l.fiber ?? null, meal: l.meal, est: l.est, fromPlan: ids.has(lid) });
  }
  const ws = live('workouts').filter(([, w]) => w.date === date);
  const b = ws.reduce((a, [, w]) => a + w.kcal, 0);
  const s = db.settings;
  const target = s.kcalTarget + (s.addBurn && b > 0 ? b : 0);
  const y = addDays(date, -1);
  const yIds = planIds(y);
  const own = logs.filter(([lid]) => !ids.has(lid)).length;
  const meds = live('meds').filter(([, m]) => isDue(m, date)).map(([mid, m]) => {
    const taken = !!db.medlog[`${date}_${mid}`];
    let status = taken ? 'taken' : date < TODAY ? 'missed' : date === TODAY && m.time && HHMM > m.time ? 'overdue' : 'due';
    return { medId: mid, groupId: m.groupId, name: m.name, dose: m.dose, time: m.time, status, taken };
  }).sort((a, b) => (a.time || '99').localeCompare(b.time || '99'));
  return {
    date, today: TODAY, isToday: date === TODAY, kcal: Math.round(kcal), prot: r1(prot), target, kcalTarget: s.kcalTarget, protFloor: s.protFloor,
    burn: b, addBurn: s.addBurn, over: kcal > target, protMet: prot >= s.protFloor,
    fiber: r1(fiber), fiberFloor: s.fiberFloor, fiberKnown, fiberMet: fiberKnown && fiber >= s.fiberFloor, nutrients: nutViews(msum, known, logs.length, s.fiberFloor),
    plan: (db.plan[date]?.meals || []).map((m, idx) => ({ idx, slot: m.slot, name: m.name, kcal: m.kcal, prot: m.prot, done: !!m.done })),
    meals: MEALS.map((m) => groups[m] || { meal: m, kcal: 0, prot: 0, rows: [] }).map((g) => { const known = g.rows.filter((r) => r.fiber != null); return { ...g, kcal: Math.round(g.kcal), prot: r1(g.prot), fiber: known.length ? r1(known.reduce((a, r) => a + r.fiber, 0)) : null }; }),
    meds, workouts: ws.map(([wid, w]) => ({ id: wid, kind: w.kind, mins: w.mins, kcal: w.kcal, name: w.name })),
    canCopyYesterday: own === 0 && live('log').some(([lid, l]) => l.date === y && !yIds.has(lid)),
    sleep: sleepView(db.sleep[date]),
    shop: shopDay(date),
  };
}

// A planned shopping day, as in the core: the trip and what's still to buy on it.
function shopDay(date) {
  const trip = db.plan[date]?.shop;
  if (!trip) return null;
  const left = Object.values(db.list).filter((l) => l.trip === trip && !l.checked && !l.inStock);
  return { trip, items: left.length, cost: Math.round(left.reduce((a, l) => a + (l.price || 0), 0) * 100) / 100 };
}

// Sleep, as in the core's sleep.rs.
const hm = (t) => { const m = /^(\d{1,2}):(\d{2})$/.exec(String(t).trim()); return m && +m[1] < 24 && +m[2] < 60 ? +m[1] * 60 + +m[2] : null; };
const fmtHm = (m) => `${String(Math.floor(m / 60) % 24).padStart(2, '0')}:${String(m % 60).padStart(2, '0')}`;
const sleepMins = (bed, wake) => { const b = hm(bed), w = hm(wake); if (b == null || w == null) return null; const d = w > b ? w - b : w + 1440 - b; return d > 0 && d <= 1200 ? d : null; };
function sleepView(s) { const mins = s && sleepMins(s.bed, s.wake); return mins ? { bed: s.bed, wake: s.wake, mins } : null; }
const avgClock = (ts, pivot) => (ts.length ? fmtHm(Math.round(ts.map((t) => (t < pivot ? t + 1440 : t)).reduce((a, b) => a + b, 0) / ts.length) % 1440) : null);

function place(part) {
  const t = part.toLowerCase();
  const g = t.match(/(\d+(?:\.\d+)?)\s*g\b/);
  const n = t.match(/^(\d+)\s/);
  for (const [name, [k, p, piece]] of Object.entries(TABLE)) {
    if (!t.includes(name)) continue;
    const grams = g ? Number(g[1]) : piece ? (n ? Number(n[1]) : 1) * piece : null;
    if (!grams) return null;
    const f = foodByRef(`table:${name}`);
    return { name: `${name[0].toUpperCase()}${name.slice(1)} · ${grams} g`, grams, kcal: Math.round(k * grams / 100), prot: r1(p * grams / 100), est: !g, meal: guess(HHMM), ...nutr(name, grams), foodRef: f.ref, sourceName: f.sourceName, alts: [], portions: f.portions };
  }
  return null;
}

function estimate({ text }) {
  if (!text.trim()) throw 'Describe what you ate first.';
  const items = [], unknown = [];
  const hasAmount = (p) => /\d|\b(a|an|one|two|half|ein|zwei)\b/i.test(p) || /^(some|bowl|plate|cup|glass|handful|portion|serving|piece|slice|bit|little|few|etwas)\b/i.test(p);
  for (const chunk of text.split(/[,;+\n]/)) {
    const bits = chunk.split(/\b(and|with|und|mit)\b/i).map((x) => x.trim()).filter(Boolean);
    const groups = [];
    for (let i = 0; i < bits.length; i++) {
      if (/^(and|with|und|mit)$/i.test(bits[i])) continue;
      const conn = i > 0 ? bits[i - 1] : '';
      if (!groups.length || hasAmount(bits[i])) groups.push([bits[i]]);
      else groups[groups.length - 1].push(`${conn} ${bits[i]}`);
    }
    for (const g of groups) {
      const placed = g.map((p) => place(p.replace(/^(and|with|und|mit)\s+/i, '')));
      if (placed.every(Boolean)) items.push(...placed);
      else unknown.push(g.join(' '));
    }
  }
  const v = { items, unknown, note: '', provider: null, providerText: null, error: null, meal: guess(HHMM) };
  if (unknown.length) {
    if (db.aiAvailable) {
      for (const u of unknown) v.items.push(fakeAi(u));
      v.unknown = [];
      v.provider = 'claude';
      v.providerText = 'Claude worked it out';
    } else {
      v.error = 'No AI is set up. Install Claude Code or add a Gemini key in Settings.';
    }
  }
  return v;
}

// Stands in for the AI: reads the amount, names the food plainly, and the app (like the
// core's match_draft) looks it up; only foods with no match keep the "AI" numbers.
const FILLER = new Set(['a', 'an', 'the', 'of', 'some', 'bowl', 'plate', 'cup', 'cups', 'glass', 'handful', 'portion', 'serving', 'piece', 'pieces', 'slice', 'slices', 'small', 'big', 'large', 'little', 'my', 'and', 'with', 'g', 'gram', 'grams']);
function fakeAi(u) {
  const t = u.toLowerCase();
  const g = t.match(/(\d+(?:\.\d+)?)\s*g\b/);
  const n = t.match(/^(\d+)\s/);
  const grams = g ? Number(g[1]) : (n ? Number(n[1]) : 1) * (/\bbowl|plate\b/.test(t) ? 300 : 150);
  const words = t.replace(/[^a-z\u00e0-\u00ff ]+/g, ' ').split(/\s+/).filter((w) => w && !FILLER.has(w));
  // Like the core: all the words, then drop words from the end until something matches.
  for (let len = words.length; len >= 1; len--) {
    const hits = searchFoods(words.slice(0, len).join(' ')).slice(0, 8);
    if (hits.length) {
      const row = draftItem({ reference: hits[0].ref, grams, meal: guess(HHMM) });
      return { ...row, est: !g, alts: hits.map((x) => ({ ref: x.ref, name: x.name, sourceName: x.sourceName })) };
    }
  }
  const name = u.trim().slice(0, 80);
  return { name, grams, kcal: Math.round(grams * 1.6), prot: r1(grams * 0.07), est: true, meal: guess(HHMM), fiber: r1(grams * 0.02), foodRef: null, sourceName: null, alts: [], portions: [] };
}

function draftItem({ reference, grams, meal: m }) {
  if (!(grams > 0 && grams <= 5000)) throw 'Grams must be between 1 and 5000.';
  const f = foodByRef(reference);
  if (!f) throw "That food isn't in your databases any more.";
  const k = grams / 100;
  const nut = Object.fromEntries(Object.entries(f.nut100).filter(([key]) => key !== 'fiber').map(([key, v]) => [key, Math.round(v * k * 1000) / 1000]));
  return { name: `${f.name} · ${Math.round(grams)} g`, grams: Math.round(grams), kcal: Math.round(f.kcal100 * k), prot: r1(f.prot100 * k), est: false, meal: meal(m), fiber: f.nut100.fiber != null ? r1(f.nut100.fiber * k) : null, nut, foodRef: f.ref, sourceName: f.sourceName, alts: [], portions: f.portions };
}

function rescaleItem({ item, grams }) {
  if (!(grams > 0 && grams <= 5000)) throw 'Grams must be between 1 and 5000.';
  if (!(item.grams > 0)) throw 'This row has no weight to scale from.';
  const f = grams / item.grams;
  const m = item.name.match(/^(.*) · [\d.]+ g$/);
  return { ...item, name: m ? `${m[1]} · ${Math.round(grams)} g` : item.name, grams: Math.round(grams), kcal: Math.round(item.kcal * f), prot: r1(item.prot * f), fiber: item.fiber != null ? r1(item.fiber * f) : null, nut: item.nut ? Object.fromEntries(Object.entries(item.nut).map(([k, v]) => [k, v * f])) : null };
}

function medsView() {
  const groups = {};
  for (const [mid, m] of live('meds')) (groups[m.groupId] ||= []).push([mid, m]);
  const v = { daily: [], days: [], monthly: [], stopped: [] };
  for (const recs of Object.values(groups)) {
    recs.sort((a, b) => cmpDate(a[1].from, b[1].from));
    const cur = [...recs].reverse().find(([, m]) => !m.until || m.until >= TODAY);
    const [mid, m] = cur || recs[recs.length - 1];
    const mv = { id: mid, ...m };
    if (!cur) v.stopped.push(mv);
    else (m.freq === 'weekly' ? v.days : m.freq === 'monthly' ? v.monthly : v.daily).push(mv);
  }
  for (const l of [v.daily, v.days, v.monthly]) l.sort((a, b) => (a.time || '99').localeCompare(b.time || '99'));
  return v;
}

function saveMed({ med }) {
  if (!med.name.trim()) throw 'Give the medicine a name.';
  if (med.freq === 'weekly' && !med.days.length) throw 'Pick at least one day.';
  if (med.freq === 'monthly' && !(med.dom >= 1 && med.dom <= 31)) throw 'Pick a day of the month (1–31).';
  const rec = { groupId: med.groupId || id(), name: med.name.trim(), dose: med.dose, freq: med.freq, days: med.days, dom: med.dom, time: med.time, from: TODAY, until: null };
  if (!med.groupId) {
    db.meds[id()] = rec;
    return { groupId: rec.groupId, startedNew: false };
  }
  const cur = Object.entries(db.meds).filter(([, m]) => m.groupId === med.groupId && (!m.until || m.until >= TODAY)).sort((a, b) => cmpDate(b[1].from, a[1].from))[0];
  if (!cur) {
    db.meds[id()] = rec;
    return { groupId: rec.groupId, startedNew: true };
  }
  const [cid, c] = cur;
  const sched = c.freq !== rec.freq || c.days.join() !== rec.days.join() || c.dom !== rec.dom || c.time !== rec.time;
  if (!sched) {
    Object.assign(c, { name: rec.name, dose: rec.dose });
    return { groupId: rec.groupId, startedNew: false };
  }
  if (c.from >= TODAY) {
    db.meds[cid] = { ...rec, from: c.from };
    return { groupId: rec.groupId, startedNew: false };
  }
  c.until = addDays(TODAY, -1);
  const nid = id();
  db.meds[nid] = rec;
  const tick = db.medlog[`${TODAY}_${cid}`];
  if (tick) {
    delete db.medlog[`${TODAY}_${cid}`];
    db.medlog[`${TODAY}_${nid}`] = { ...tick, medId: nid };
  }
  return { groupId: rec.groupId, startedNew: true };
}

function stockView() {
  const secs = {};
  let soon = 0, total = 0;
  for (const [sid, s] of live('stock')) {
    const age = Math.round((new Date(TODAY) - new Date(s.added)) / 864e5);
    const left = s.useBy ? Math.round((new Date(s.useBy) - new Date(TODAY)) / 864e5) : null;
    const isSoon = left !== null && left <= 7;
    soon += isSoon; total++;
    (secs[s.cat] ||= []).push({ id: sid, ...s, ageDays: age, daysLeft: left, soon: isSoon, eatToday: s.cat === 'cooked' && age >= 3 });
  }
  return { sections: CATS.filter((c) => secs[c]).map((c) => ({ cat: c, items: secs[c].sort((a, b) => a.name.localeCompare(b.name)) })), soonCount: soon, total };
}

function shopView({ trip }) {
  const groups = {};
  const inStock = [];
  let spend = 0, tripSpend = 0, checked = 0;
  for (const [lid, l] of live('list')) {
    const row = { id: lid, ...l };
    if (l.inStock) { if (l.trip === trip) inStock.push(row); continue; }
    spend += l.price || 0;
    if (l.trip !== trip) continue;
    tripSpend += l.price || 0;
    checked += l.checked ? 1 : 0;
    (groups[l.cat] ||= []).push(row);
  }
  spend = Math.round(spend * 100) / 100;
  return { trip, budget: 80, spend, tripSpend: Math.round(tripSpend * 100) / 100, overBudget: spend > 80, groups: CATS.filter((c) => groups[c]).map((c) => ({ cat: c, items: groups[c] })), inStock, checkedCount: checked };
}

function parseQty(q) {
  const m = String(q || '').trim().match(/^(\d+(?:[.,]\d+)?)\s*(.*)$/);
  return m ? [Number(m[1].replace(',', '.')), m[2].trim()] : [1, ''];
}

function weekView() {
  const days = [];
  let sk = 0, sp = 0, n = 0;
  for (let i = -7; i <= 7; i++) {
    const d = addDays(TODAY, i);
    const meals = db.plan[d]?.meals || [];
    const kcal = meals.reduce((a, m) => a + m.kcal, 0), prot = meals.reduce((a, m) => a + m.prot, 0);
    if (meals.length) { sk += kcal; sp += prot; n++; }
    days.push({ date: d, dow: dow(d), isToday: d === TODAY, meals, kcal, prot });
  }
  return { days, avgKcal: n ? Math.round(sk / n) : 0, avgProt: n ? r1(sp / n) : 0, plannedDays: n };
}

function history({ range = 60 } = {}) {
  const st = db.settings;
  const food = {}, mins = {}, burns = {}, ticks = new Set();
  for (const [, l] of live('log')) {
    const e = (food[l.date] ||= { kcal: 0, prot: 0, fiber: 0, fk: false, m: {} });
    e.kcal += l.kcal; e.prot += l.prot;
    if (l.fiber != null) { e.fiber += l.fiber; e.fk = true; }
    for (const [k, v] of Object.entries(rowNut(l))) e.m[k] = (e.m[k] || 0) + v;
  }
  for (const [, w] of live('workouts')) { mins[w.date] = (mins[w.date] || 0) + w.mins; burns[w.date] = (burns[w.date] || 0) + w.kcal; }
  for (const k of Object.keys(db.medlog)) ticks.add(k);
  const used = (d) => !!food[d] || !!mins[d] || [...ticks].some((t) => t.startsWith(d)) || !!sleepView(db.sleep[d]);
  let first = addDays(TODAY, 1 - range);
  while (first <= TODAY && !used(first)) first = addDays(first, 1);
  const days = [], medHist = {};
  for (let d = first; d <= TODAY; d = addDays(d, 1)) {
    const f = food[d] || { kcal: 0, prot: 0, fiber: 0, fk: false, m: {} };
    const due = live('meds').filter(([, m]) => isDue(m, d));
    let taken = 0;
    for (const [mid, m] of due) {
      const t = ticks.has(`${d}_${mid}`);
      taken += t;
      if (used(d)) { const h = (medHist[m.groupId] ||= { name: m.name, taken: 0, due: 0 }); h.taken += t; h.due++; }
    }
    const target = st.kcalTarget + (st.addBurn ? burns[d] || 0 : 0);
    days.push({ date: d, kcal: Math.round(f.kcal), prot: r1(f.prot), fiber: f.fk ? r1(f.fiber) : null, nut: Object.keys(f.m).length ? f.m : null, mins: mins[d] || 0, taken, due: due.length, used: used(d),
      protMet: f.prot >= st.protFloor && f.prot > 0, fiberMet: f.fk && f.fiber >= st.fiberFloor, over: f.kcal > target, target: Math.round(target), sleep: sleepView(db.sleep[d]) });
  }
  const u = days.filter((d) => d.used);
  const fd = days.filter((d) => d.kcal > 0);
  const avg = (a) => (a.length ? a.reduce((x, y) => x + y, 0) / a.length : 0);
  const fib = fd.filter((d) => d.fiber != null).map((d) => d.fiber);
  const nsum = {}, nk = {};
  for (const d of fd) for (const [k, v] of Object.entries(d.nut || {})) { nsum[k] = (nsum[k] || 0) + v; nk[k] = (nk[k] || 0) + 1; }
  const navg = Object.fromEntries(Object.entries(nsum).map(([k, v]) => [k, v / nk[k]]));
  const due = u.reduce((a, d) => a + d.due, 0), taken = u.reduce((a, d) => a + d.taken, 0);
  const summary = {
    avgKcal: Math.round(avg(fd.map((d) => d.kcal))), avgProt: r1(avg(fd.map((d) => d.prot))), avgFiber: fib.length ? r1(avg(fib)) : null,
    protDays: u.filter((d) => d.protMet).length, fiberDays: u.filter((d) => d.fiberMet).length, foodDays: fd.length,
    adherence: due ? Math.round((taken * 100) / due) : null, daysTrained: days.filter((d) => d.mins > 0).length, daysUsed: u.length,
    nutrients: nutViews(navg, nk, fd.length, st.fiberFloor),
  };
  const slept = days.map((d) => d.sleep).filter(Boolean);
  Object.assign(summary, {
    avgSleepMins: slept.length ? Math.round(avg(slept.map((x) => x.mins))) : null,
    avgBed: avgClock(slept.map((x) => hm(x.bed)), 720), avgWake: avgClock(slept.map((x) => hm(x.wake)), 1080), sleepNights: slept.length,
  });
  const meds = Object.values(medHist).map((m) => ({ ...m, missed: m.due - m.taken }));
  const ins = [];
  const short = fd.filter((d) => d.prot < st.protFloor).length;
  if (short) ins.push({ level: short * 3 > fd.length ? 'warn' : 'info', title: `Protein under ${st.protFloor} g on ${short} of ${fd.length} days`, detail: `Average ${summary.avgProt} g. Highest per 100 g in your table: Soy granulate dry, Chicken breast, Erdnussmus, Cooked prawns.` });
  if (summary.avgFiber != null && summary.avgFiber < st.fiberFloor) ins.push({ level: 'warn', title: `Fiber averaged ${summary.avgFiber} g, under your ${st.fiberFloor} g floor`, detail: 'Richest in your table: Ground flaxseed, Soy granulate dry, Oats, Chickpeas drained.' });
  for (const m of summary.nutrients.filter((m) => m.key !== 'fiber' && m.coverage >= 0.5 && m.pct != null)) {
    if (m.kind === 'target' && m.pct < 60) ins.push({ level: 'warn', title: `${m.label} at ${m.pct}% of the daily amount`, detail: `About ${m.amount} ${m.unit} a day against ${m.reference} ${m.unit}, from food only (supplements not counted).` });
    if (m.kind === 'limit' && m.pct > 100) ins.push({ level: 'warn', title: `${m.label} above your limit`, detail: `About ${m.amount} ${m.unit} a day against a limit of ${m.reference} ${m.unit}.` });
  }
  for (const m of meds.filter((m) => m.missed)) ins.push({ level: m.missed * 5 > m.due ? 'warn' : 'info', title: `${m.name} missed ${m.missed} of ${m.due} times`, detail: 'Counting only days you used the app.' });
  if (slept.length >= 3) {
    const a = summary.avgSleepMins, h = `${Math.floor(a / 60)} h${a % 60 ? ` ${a % 60} min` : ''}`, t = ` Usually in bed by ${summary.avgBed}, up at ${summary.avgWake}.`;
    ins.push(a < 420 ? { level: 'warn', title: `Sleeping ${h} a night on average`, detail: `Most adults need 7–9 hours.${t}` } : { level: 'good', title: `Sleeping ${h} a night`, detail: `Within the 7–9 hours most adults need.${t}` });
  }
  const over = fd.filter((d) => d.over).length;
  if (over * 3 > fd.length) ins.push({ level: 'warn', title: `Over your calorie target on ${over} of ${fd.length} days`, detail: `Average ${summary.avgKcal} kcal against ${st.kcalTarget} kcal.` });
  if (!short && fd.length) ins.push({ level: 'good', title: 'Protein floor met every day', detail: `Averaging ${summary.avgProt} g.` });
  ins.sort((a, b) => ['warn', 'info', 'good'].indexOf(a.level) - ['warn', 'info', 'good'].indexOf(b.level));
  days.reverse();
  return { range, kcalTarget: st.kcalTarget, protFloor: st.protFloor, fiberFloor: st.fiberFloor, days, summary, meds, insights: ins };
}

function planAhead({ days = 7 }) {
  const st = db.settings;
  const out = [];
  let sk = 0, sp = 0, n = 0;
  for (let i = 0; i < days; i++) {
    const d = addDays(TODAY, i);
    const meals = (db.plan[d]?.meals || []).map((m, idx) => ({ idx, slot: m.slot, name: m.name, kcal: m.kcal, prot: m.prot, fiber: m.fiber ?? null, done: !!m.done }));
    const kcal = meals.reduce((a, m) => a + m.kcal, 0), prot = meals.reduce((a, m) => a + m.prot, 0), fiber = meals.reduce((a, m) => a + (m.fiber || 0), 0);
    if (meals.length) { sk += kcal; sp += prot; n++; }
    out.push({ date: d, dow: dow(d), isToday: i === 0, offset: i, meals, kcal, prot: r1(prot), fiber: r1(fiber), over: kcal > st.kcalTarget, protMet: prot >= st.protFloor, emptySlots: MEALS.filter((s) => !meals.some((m) => m.slot === s)), shop: shopDay(d) });
  }
  return { days: out, kcalTarget: st.kcalTarget, protFloor: st.protFloor, fiberFloor: st.fiberFloor, plannedDays: n, avgKcal: n ? Math.round(sk / n) : 0, avgProt: n ? r1(sp / n) : 0, hasEarlier: Object.entries(db.plan).some(([d, p]) => d < TODAY && p.meals.length) };
}

function trainingSummary() {
  const kinds = {}, daily = {};
  for (let i = 27; i >= 0; i--) daily[addDays(TODAY, -i)] = 0;
  let wk = [0, 0, 0], last = null;
  for (const [, w] of live('workouts')) {
    const ago = Math.round((new Date(TODAY) - new Date(w.date)) / 864e5);
    if (ago < 0) continue;
    last = last == null ? ago : Math.min(last, ago);
    if (ago < 7) { wk[0]++; wk[1] += w.mins; wk[2] += w.kcal; }
    if (ago < 28) { kinds[w.kind] = (kinds[w.kind] || 0) + w.mins; daily[w.date] = (daily[w.date] || 0) + w.mins; }
  }
  return { weekSessions: wk[0], weekMins: wk[1], weekKcal: Math.round(wk[2]), lastDaysAgo: last, byKind: Object.entries(kinds).sort((a, b) => b[1] - a[1]), daily: Object.entries(daily) };
}

const handlers = {
  today: () => TODAY,
  get_day: getDay,
  add_log: ({ date, entry }) => {
    if (!entry.name.trim()) throw 'Give the food a name.';
    if (!(entry.kcal > 0 && entry.kcal < 5000)) throw 'Calories must be above 0 and below 5000.';
    const lid = id();
    db.log[lid] = { date, meal: meal(entry.meal), name: entry.name, grams: entry.grams, kcal: Math.round(entry.kcal), prot: r1(entry.prot), est: !!entry.est, ts: db.ts++, fiber: entry.fiber ?? null, nut: entry.nut ?? null, foodRef: entry.foodRef ?? null };
    return lid;
  },
  save_items: ({ date, items }) => {
    for (const it of items) db.log[id()] = { date, meal: meal(it.meal), name: it.name, grams: it.grams, kcal: it.kcal, prot: it.prot, est: it.est, ts: db.ts++, fiber: it.fiber, nut: it.nut, foodRef: it.foodRef ?? null };
    return items.length;
  },
  delete_log: ({ id: lid }) => {
    const l = db.log[lid];
    if (l && db.plan[l.date]) for (const m of db.plan[l.date].meals) if (m.logId === lid) { m.done = false; m.logId = null; }
    delete db.log[lid];
  },
  toggle_plan: ({ date, idx, name }) => {
    const m = planMealAt(date, idx, name);
    if (m.done) { delete db.log[m.logId]; m.done = false; m.logId = null; }
    else { const lid = id(); db.log[lid] = { date, meal: meal(m.slot), name: m.name, grams: null, kcal: m.kcal, prot: m.prot, est: false, ts: db.ts++ }; m.done = true; m.logId = lid; }
    return m.done;
  },
  set_plan: ({ date, meals }) => { db.plan[date] = { ...db.plan[date], meals: meals.map((m) => ({ ...m })) }; },
  set_shop_day: ({ date, trip }) => {
    if (trip != null && trip !== 1 && trip !== 2) throw 'Pick Trip 1 or Trip 2.';
    const p = (db.plan[date] ||= { meals: [] });
    if (trip == null) delete p.shop; else p.shop = trip;
  },
  plan_food: ({ date, slot, reference, grams }) => {
    const f = foodByRef(reference);
    if (!f) throw "That food isn't in your databases any more.";
    if (!(grams > 0 && grams <= 5000)) throw 'Grams must be between 1 and 5000.';
    const k = grams / 100;
    (db.plan[date] ||= { meals: [] }).meals.push({ slot: meal(slot), name: `${f.name} · ${Math.round(grams)} g`, kcal: Math.round(f.kcal100 * k), prot: r1(f.prot100 * k), fiber: f.nut100.fiber != null ? r1(f.nut100.fiber * k) : null, done: false, logId: null });
  },
  copy_yesterday: ({ date }) => {
    const y = addDays(date, -1), ids = planIds(y);
    const rows = live('log').filter(([lid, l]) => l.date === y && !ids.has(lid));
    for (const [, l] of rows) db.log[id()] = { ...l, date, ts: db.ts++ };
    return rows.length;
  },
  // Same as the core: frequency × recency (halves every 14 days), one chip per food, latest amount.
  recent_foods: () => {
    const score = {}, latest = {};
    for (const [, l] of live('log')) {
      const age = Math.max(0, Math.round((new Date(TODAY) - new Date(l.date)) / 864e5));
      if (age > 90) continue;
      const k = l.name.split(' · ')[0].toLowerCase();
      score[k] = (score[k] || 0) + Math.pow(0.5, age / 14);
      if (!latest[k] || l.ts > latest[k].ts) latest[k] = l;
    }
    return Object.keys(latest).sort((a, b) => score[b] - score[a] || latest[b].ts - latest[a].ts).slice(0, 8)
      .map((k) => { const l = latest[k]; return { name: l.name, grams: l.grams, kcal: l.kcal, prot: l.prot, est: l.est, fiber: l.fiber, nut: l.nut, foodRef: l.foodRef }; });
  },
  estimate_food: estimate,
  estimate_photo: () => {
    if (!db.aiAvailable) throw 'No AI is set up. Install Claude Code or add a Gemini key in Settings.';
    return { items: [{ name: 'Curry with rice', grams: 450, kcal: 620, prot: 28, est: true, meal: guess(HHMM) }], unknown: [], note: 'Looks like a curry.', provider: 'gemini', providerText: 'Gemini worked it out', error: null, meal: guess(HHMM) };
  },
  quick_fill: ({ name, grams }) => {
    const k = Object.keys(TABLE).find((n) => name.toLowerCase().includes(n));
    if (!k) return null;
    const [kc, p, piece] = TABLE[k];
    const g = grams || piece || 100;
    return { kcal: Math.round(kc * g / 100), prot: r1(p * g / 100), fiber: nutr(k, g).fiber ?? null, grams: g, matched: k[0].toUpperCase() + k.slice(1) };
  },
  food_names: () => Object.keys(TABLE).map((n) => n[0].toUpperCase() + n.slice(1)),
  get_meds: medsView,
  save_med: saveMed,
  stop_med: ({ groupId }) => { for (const m of Object.values(db.meds)) if (m.groupId === groupId && (!m.until || m.until >= TODAY)) m.until = addDays(TODAY, -1); },
  erase_med: ({ groupId }) => {
    const ids = Object.keys(db.meds).filter((k) => db.meds[k].groupId === groupId);
    for (const k of Object.keys(db.medlog)) if (ids.includes(db.medlog[k].medId)) delete db.medlog[k];
    for (const k of ids) delete db.meds[k];
    return ids.length;
  },
  toggle_dose: ({ date, medId }) => {
    const k = `${date}_${medId}`;
    if (db.medlog[k]) { delete db.medlog[k]; return false; }
    db.medlog[k] = { date, medId, at: HHMM, ts: db.ts++ };
    return true;
  },
  burn_estimate: ({ kind, mins }) => burn(kind, mins),
  add_workout: ({ date, kind, mins, kcal, name }) => {
    if (!(mins > 0 && mins <= 1440)) throw 'Minutes must be between 1 and 1440.';
    const wid = id();
    db.workouts[wid] = { date, kind, mins, kcal: kcal || burn(kind, mins), name: name || kind[0].toUpperCase() + kind.slice(1), ts: db.ts++ };
    return wid;
  },
  delete_workout: ({ id: wid }) => { delete db.workouts[wid]; },
  recent_workouts: ({ limit }) => live('workouts').sort((a, b) => cmpDate(b[1].date, a[1].date) || b[1].ts - a[1].ts).slice(0, limit).map(([wid, w]) => ({ id: wid, date: w.date, kind: w.kind, mins: w.mins, kcal: w.kcal, name: w.name })),
  import_workouts: ({ text }) => {
    let added = 0, skipped = 0;
    for (const line of text.split('\n').filter((l) => l.trim())) {
      const f = line.split(/[,;\t]/).map((x) => x.trim());
      const date = f.find((x) => /^\d{4}-\d{2}-\d{2}$/.test(x));
      const mins = f.find((x) => /^\d+$/.test(x));
      if (!date || !mins || date > TODAY || Number(mins) > 1440) { skipped++; continue; }
      db.workouts[id()] = { date, kind: 'other', mins: Number(mins), kcal: burn('other', Number(mins)), name: f.find((x) => /[a-z]/i.test(x)) || 'Other', ts: db.ts++ };
      added++;
    }
    return { added, skipped };
  },
  set_add_burn: ({ on }) => { db.settings.addBurn = on; },
  get_stock: stockView,
  save_stock: ({ id: sid, item }) => {
    if (!item.name.trim()) throw 'Give the item a name.';
    const k = sid || id();
    db.stock[k] = { ...item, added: item.added || TODAY };
    return k;
  },
  delete_stock: ({ id: sid }) => { delete db.stock[sid]; },
  get_shop: shopView,
  save_list_item: ({ id: lid, item }) => { if (!item.name.trim()) throw 'Give the item a name.'; const k = lid || id(); db.list[k] = { ...item }; return k; },
  toggle_list_item: ({ id: lid }) => (db.list[lid].checked = !db.list[lid].checked),
  delete_list_item: ({ id: lid }) => { delete db.list[lid]; },
  check_have: () => {
    const out = [];
    for (const [lid, l] of live('list')) {
      if (l.inStock) continue;
      const ln = l.name.toLowerCase();
      const hit = live('stock').find(([, s]) => ln.includes(s.name.toLowerCase()) || s.name.toLowerCase().includes(ln));
      if (hit) out.push({ listId: lid, listName: l.name, stockId: hit[0], stockName: hit[1].name, stockQty: hit[1].qty, stockUnit: hit[1].unit });
    }
    return out;
  },
  resolve_have: ({ listId, stockId, stillThere }) => { if (stillThere) { db.list[listId].inStock = true; db.list[listId].checked = false; } else delete db.stock[stockId]; },
  move_back: ({ listId }) => { db.list[listId].inStock = false; },
  bought: ({ trip }) => {
    const items = live('list').filter(([, l]) => l.checked && !l.inStock && l.trip === trip);
    for (const [lid, l] of items) {
      const [qty, unit] = parseQty(l.qty);
      const ex = live('stock').find(([, s]) => s.name.toLowerCase() === l.name.toLowerCase() && s.unit.toLowerCase() === unit.toLowerCase());
      if (ex) ex[1].qty += qty;
      else db.stock[id()] = { name: l.name, qty, unit, cat: l.cat, added: TODAY, useBy: null };
      delete db.list[lid];
    }
    return items.length;
  },
  get_week: weekView,
  repeat_last_week: () => {
    const planned = Object.entries(db.plan).filter(([, p]) => p.meals.length).sort((a, b) => cmpDate(b[0], a[0])).slice(0, 7);
    if (!planned.length) throw "There's no planned week to repeat yet.";
    let n = 0;
    for (const [d, p] of planned) {
      const t = addDays(d, 7);
      if (db.plan[t]?.meals.length) continue;
      db.plan[t] = { meals: p.meals.map((m) => ({ ...m, done: false, logId: null })), ...(db.plan[t]?.shop || p.shop ? { shop: db.plan[t]?.shop || p.shop } : {}) };
      n++;
    }
    return n;
  },
  get_history: history,
  get_plan_ahead: planAhead,
  search_foods: ({ query }) => searchFoods(query),
  draft_item: draftItem,
  rescale_item: rescaleItem,
  food_detail: ({ reference }) => foodByRef(reference),
  add_food: ({ date, meal: m, reference, grams }) => {
    const f = foodByRef(reference);
    if (!f) throw "That food isn't in your databases any more.";
    if (!(grams > 0 && grams <= 5000)) throw 'Grams must be between 1 and 5000.';
    const k = grams / 100;
    const nut = Object.fromEntries(Object.entries(f.nut100).filter(([key]) => key !== 'fiber').map(([key, v]) => [key, Math.round(v * k * 1000) / 1000]));
    const lid = id();
    db.log[lid] = { date, meal: meal(m), name: `${f.name} · ${Math.round(grams)} g`, grams: Math.round(grams), kcal: Math.round(f.kcal100 * k), prot: r1(f.prot100 * k), est: false, ts: db.ts++, fiber: f.nut100.fiber != null ? r1(f.nut100.fiber * k) : null, nut, foodRef: f.ref };
    return lid;
  },
  food_sources: () => db.sources,
  search_off: ({ query }) => {
    if (!db.online) throw "Couldn't reach Open Food Facts (offline?).";
    const found = OFF_SAMPLE.filter((f) => norm(f.name).includes(norm(query).split(' ')[0]) || f.ref === `off:${query.trim()}`);
    for (const f of found) if (!db.off.some((x) => x.ref === f.ref)) db.off.push(f);
    if (found.length && !db.sources.some((x) => x.id === 'off')) db.sources = [...db.sources, { id: 'off', name: 'Open Food Facts', version: 'looked up online', licence: 'Open Database License (ODbL) 1.0', attribution: 'Product data © Open Food Facts contributors, openfoodfacts.org', url: 'https://world.openfoodfacts.org', importedAt: Date.now(), foods: db.off.length }];
    return found.map((f) => ({ ...f, fiber100: f.nut100.fiber ?? null, known: Object.keys(f.nut100).length }));
  },
  ingredient: ({ reference, grams }) => {
    const f = foodByRef(reference);
    if (!f) throw "That food isn't in your databases any more.";
    const k = grams / 100;
    const nut = Object.fromEntries(Object.entries(f.nut100).filter(([key]) => key !== 'fiber').map(([key, v]) => [key, Math.round(v * k * 1000) / 1000]));
    return { name: f.name, foodRef: f.ref, grams: Math.round(grams), kcal: Math.round(f.kcal100 * k), prot: r1(f.prot100 * k), fiber: f.nut100.fiber != null ? r1(f.nut100.fiber * k) : null, nut };
  },
  recipe_preview: ({ recipe }) => recipeView('', cleanRecipe(recipe)),
  save_recipe: ({ id: rid, recipe }) => { const k = rid || id(); db.recipes[k] = cleanRecipe(recipe); return k; },
  get_recipes: () => Object.entries(db.recipes).map(([rid, r]) => recipeView(rid, r)).sort((a, b) => a.name.localeCompare(b.name)),
  delete_recipe: ({ id: rid }) => { delete db.recipes[rid]; },
  download_swiss: () => {
    if (!db.online) throw "Couldn't reach naehrwertdaten.ch (offline?).";
    db.swiss = SWISS_SAMPLE;
    db.sources = [...db.sources.filter((x) => x.id !== 'swiss'), { id: 'swiss', name: 'Swiss Food Composition Database', version: '2026-07', licence: 'Free of charge, source must be acknowledged', attribution: 'Swiss Food Composition Database, Federal Food Safety and Veterinary Office FSVO. naehrwertdaten.ch', url: 'https://naehrwertdaten.ch/en/downloads/', importedAt: Date.now(), foods: 1216 }];
    return 1216;
  },
  remove_food_source: ({ id: sid }) => { if (sid === 'usda') throw "The USDA database comes with the app and can't be removed."; db.sources = db.sources.filter((x) => x.id !== sid); if (sid === 'swiss') db.swiss = []; },
  update_log: ({ id: lid, entry }) => {
    const l = db.log[lid];
    if (!l) throw 'That entry no longer exists.';
    if (!entry.name.trim()) throw 'Give the food a name.';
    if (!(entry.kcal >= 0 && entry.kcal < 5000)) throw 'Calories must be between 0 and 5000.';
    let nm = entry.name.trim();
    const tail = ` · ${Math.round(l.grams)} g`;
    if (l.grams && entry.grams && l.grams !== entry.grams && nm.endsWith(tail)) nm = `${nm.slice(0, -tail.length)} · ${Math.round(entry.grams)} g`;
    Object.assign(l, { name: nm, meal: meal(entry.meal), grams: entry.grams, kcal: Math.round(entry.kcal), prot: r1(entry.prot), fiber: entry.fiber ?? null });
    const p = db.plan[l.date];
    if (p) for (const m of p.meals) if (m.logId === lid) Object.assign(m, { name: l.name, kcal: l.kcal, prot: l.prot, fiber: l.fiber, slot: l.meal });
  },
  update_plan_meal: ({ date, idx, name, meal: m }) => {
    const p = db.plan[date];
    const old = planMealAt(date, idx, name);
    p.meals[idx] = { ...m, done: old.done, logId: old.logId };
    if (old.logId && db.log[old.logId]) Object.assign(db.log[old.logId], { name: m.name, kcal: m.kcal, prot: m.prot, fiber: m.fiber ?? null, meal: m.slot });
  },
  update_workout: ({ id: wid, kind, mins, kcal, name }) => {
    const w = db.workouts[wid];
    if (!w) throw 'That session no longer exists.';
    if (!(mins > 0 && mins <= 1440)) throw 'Minutes must be between 1 and 1440.';
    Object.assign(w, { kind, mins, kcal: kcal || burn(kind, mins), name: name || kind[0].toUpperCase() + kind.slice(1) });
  },
  add_plan_meal: ({ date, meal: m }) => { (db.plan[date] ||= { meals: [] }).meals.push({ ...m, done: false, logId: null }); },
  remove_plan_meal: ({ date, idx, name }) => { planMealAt(date, idx, name); const p = db.plan[date]; if (p) { const [m] = p.meals.splice(idx, 1); if (m?.logId) delete db.log[m.logId]; } },
  training_summary: trainingSummary,
  set_sleep: ({ date, bed, wake }) => {
    if (date > TODAY) throw "You can't log sleep for a day that hasn't come yet.";
    const b = hm(bed), w = hm(wake);
    if (b == null || w == null) throw 'Use times like 23:30 and 07:00.';
    if (!sleepMins(bed, wake)) throw "Bedtime and wake-up time can't be the same, or more than 20 hours apart.";
    db.sleep[date] = { bed: fmtHm(b), wake: fmtHm(w), ts: db.ts++ };
  },
  clear_sleep: ({ date }) => { delete db.sleep[date]; },
  get_settings: () => ({ ...db.settings }),
  save_settings: ({ settings }) => {
    if (!(settings.weightKg >= 30 && settings.weightKg <= 300)) throw 'Weight must be between 30 and 300 kg.';
    if (!(settings.fiberFloor >= 0 && settings.fiberFloor <= 100)) throw 'Fiber floor must be between 0 and 100 g.';
    db.settings = { ...settings };
  },
  get_ai_settings: () => ({ order: db.ai.order, geminiKeySet: !!db.ai.geminiKey, geminiModel: db.ai.geminiModel, claudeModel: db.ai.claudeModel, claudePath: db.ai.claudePath, claudeFound: '/Users/me/.local/bin/claude' }),
  save_ai_settings: ({ ai }) => {
    db.ai.order = ai.order;
    if (ai.clearGeminiKey) db.ai.geminiKey = '';
    else if (ai.geminiKey) db.ai.geminiKey = ai.geminiKey;
    Object.assign(db.ai, { geminiModel: ai.geminiModel, claudeModel: ai.claudeModel, claudePath: ai.claudePath });
  },
  test_ai: () => { if (!db.aiAvailable) throw 'No AI is set up. Install Claude Code or add a Gemini key in Settings.'; return 'Claude answered. AI is working.'; },
  learned_foods: () => Object.entries(db.foods).map(([fid, f]) => ({ id: fid, ...f })),
  delete_learned: ({ id: fid }) => { delete db.foods[fid]; },
  get_rules: () => db.rules,
  rules_chat: ({ message }) => ({ reply: 'Sure. I can add that rule.', change: { add: message.slice(0, 200), remove: null }, providerText: 'Claude worked it out' }),
  apply_rule_change: ({ change }) => {
    const lines = db.rules.split('\n').filter((l) => l.trim() && l.trim() !== change.remove);
    if (change.add) lines.push(change.add);
    db.rules = lines.join('\n');
    return db.rules;
  },
  export_to: ({ path }) => { db.lastExport = path; },
  import_from: () => 3,
  'plugin:dialog|save': () => '/tmp/privatei-backup.json',
  'plugin:app|version': () => '0.4.0',
  // ?update=0.5.0 pretends a newer version is out; downloading it always fails here.
  'plugin:updater|check': () => (params.get('update') ? { rid: 1, currentVersion: '0.4.0', version: params.get('update'), date: null, body: '', rawJson: {} } : null),
  'plugin:updater|download_and_install': () => { throw 'The download was interrupted.'; },
  'plugin:dialog|open': () => '/tmp/privatei-backup.json',
};

/** Like the core: the meal at idx, checked against the name the screen showed. */
function planMealAt(date, idx, name) {
  const m = db.plan[date]?.meals[idx];
  if (!m || (name != null && m.name !== name)) throw "That meal isn't in the plan any more.";
  return m;
}

// ?delay=ms makes every answer slow, like a real disk, to catch double clicks and races.
const DELAY = Number(params.get('delay') || 0);

export function install() {
  seed();
  window.__mock = db;
  mockIPC((cmd, args) => {
    db.calls.push(cmd);
    const fn = handlers[cmd];
    if (!fn) throw `mock: unknown command ${cmd}`;
    const answer = () => {
      const out = fn(args || {});
      return out === undefined ? null : JSON.parse(JSON.stringify(out));
    };
    // Tests can set window.__mockDelay = (cmd, args) => ms to make one answer slower than the next.
    const ms = window.__mockDelay ? window.__mockDelay(cmd, args || {}) : cmd.startsWith('plugin:') ? 0 : DELAY;
    if (!ms) return answer();
    return new Promise((ok, fail) => setTimeout(() => { try { ok(answer()); } catch (e) { fail(e); } }, ms));
  });
}
