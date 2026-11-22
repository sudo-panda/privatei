// A small in-memory stand-in for the Rust commands, for UI tests only.
// It is bundled only in `vite build --mode mock`. The real logic lives in privatei-core
// and has its own tests; this only needs to be believable enough to drive every screen.
import { mockIPC } from '@tauri-apps/api/mocks';

const params = new URLSearchParams(location.search);
const TODAY = params.get('today') || '2026-11-22';
const HHMM = params.get('hhmm') || '09:00';
const MEALS = ['Breakfast', 'Lunch', 'Snack', 'Dinner'];
const CATS = ['cooked', 'chilled', 'produce', 'shelf', 'freezer'];
const MET = { gym: 5, run: 9.8, walk: 3.5, cycle: 7.5, swim: 7, other: 4 };
const TABLE = { 'chicken breast': [165, 31, 0], egg: [143, 12.6, 55], banana: [89, 1.1, 120], skyr: [63, 11, 0], oats: [370, 13, 0] };

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
  rules: '', settings: { addBurn: false, weightKg: 82, kcalTarget: 1500, protFloor: 90 },
  ai: { order: ['claude', 'gemini'], geminiKey: '', geminiModel: 'gemini-flash-latest', claudeModel: '', claudePath: '' },
  aiAvailable: true,
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
  const rows = Number(params.get('rows') || 0);
  for (let i = 0; i < rows; i++) {
    const d = addDays(TODAY, -(i % 60));
    db.log[`r${i}`] = { date: d, meal: MEALS[i % 4], name: `Food ${i % 50}`, grams: 100, kcal: 100 + (i % 7) * 10, prot: 5 + (i % 5), est: i % 3 === 0, ts: db.ts++ };
  }
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
  let kcal = 0, prot = 0;
  for (const [lid, l] of logs) {
    const g = (groups[l.meal] ||= { meal: l.meal, kcal: 0, prot: 0, rows: [] });
    g.kcal += l.kcal; g.prot += l.prot; kcal += l.kcal; prot += l.prot;
    g.rows.push({ id: lid, name: l.name, grams: l.grams, kcal: l.kcal, prot: l.prot, est: l.est, fromPlan: ids.has(lid) });
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
    plan: (db.plan[date]?.meals || []).map((m, idx) => ({ idx, slot: m.slot, name: m.name, kcal: m.kcal, prot: m.prot, done: !!m.done })),
    meals: MEALS.filter((m) => groups[m]).map((m) => ({ ...groups[m], kcal: Math.round(groups[m].kcal), prot: r1(groups[m].prot) })),
    meds, workouts: ws.map(([wid, w]) => ({ id: wid, kind: w.kind, mins: w.mins, kcal: w.kcal, name: w.name })),
    canCopyYesterday: own === 0 && live('log').some(([lid, l]) => l.date === y && !yIds.has(lid)),
  };
}

function place(part) {
  const t = part.toLowerCase();
  const g = t.match(/(\d+(?:\.\d+)?)\s*g\b/);
  const n = t.match(/^(\d+)\s/);
  for (const [name, [k, p, piece]] of Object.entries(TABLE)) {
    if (!t.includes(name)) continue;
    const grams = g ? Number(g[1]) : piece ? (n ? Number(n[1]) : 1) * piece : null;
    if (!grams) return null;
    return { name: `${name[0].toUpperCase()}${name.slice(1)} · ${grams} g`, grams, kcal: Math.round(k * grams / 100), prot: r1(p * grams / 100), est: !g, meal: guess(HHMM) };
  }
  return null;
}

function estimate({ text }) {
  if (!text.trim()) throw 'Describe what you ate first.';
  const parts = text.split(/[,;+\n]|\band\b|\bwith\b|\bund\b|\bmit\b/i).map((s) => s.trim()).filter(Boolean);
  const items = [], unknown = [];
  for (const p of parts) {
    const it = place(p);
    it ? items.push(it) : unknown.push(p);
  }
  const v = { items, unknown, note: '', provider: null, providerText: null, error: null, meal: guess(HHMM) };
  if (unknown.length) {
    if (db.aiAvailable) {
      for (const u of unknown) v.items.push({ name: u.slice(0, 80), grams: 250, kcal: 400, prot: 20, est: true, meal: guess(HHMM) });
      v.unknown = [];
      v.provider = 'claude';
      v.providerText = 'Claude worked it out';
    } else {
      v.error = 'No AI is set up. Install Claude Code or add a Gemini key in Settings.';
    }
  }
  return v;
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

function history() {
  const food = {}, mins = {}, ticks = new Set();
  for (const [, l] of live('log')) { const e = (food[l.date] ||= [0, 0]); e[0] += l.kcal; e[1] += l.prot; }
  for (const [, w] of live('workouts')) mins[w.date] = (mins[w.date] || 0) + w.mins;
  for (const k of Object.keys(db.medlog)) ticks.add(k);
  const used = (d) => !!food[d] || !!mins[d] || [...ticks].some((t) => t.startsWith(d));
  let first = addDays(TODAY, -59);
  while (first <= TODAY && !used(first)) first = addDays(first, 1);
  const days = [];
  for (let d = first; d <= TODAY; d = addDays(d, 1)) {
    const [k, p] = food[d] || [0, 0];
    const due = live('meds').filter(([, m]) => isDue(m, d));
    const taken = due.filter(([mid]) => ticks.has(`${d}_${mid}`)).length;
    days.push({ date: d, kcal: Math.round(k), prot: r1(p), mins: mins[d] || 0, taken, due: due.length, used: used(d), protMet: p >= db.settings.protFloor, over: k > db.settings.kcalTarget });
  }
  const u = days.filter((d) => d.used);
  const fd = u.filter((d) => d.kcal > 0);
  const due = u.reduce((a, d) => a + d.due, 0), taken = u.reduce((a, d) => a + d.taken, 0);
  days.reverse();
  return {
    days,
    summary: {
      avgKcal: fd.length ? Math.round(fd.reduce((a, d) => a + d.kcal, 0) / fd.length) : 0,
      avgProt: fd.length ? r1(fd.reduce((a, d) => a + d.prot, 0) / fd.length) : 0,
      protDays: u.filter((d) => d.protMet).length,
      adherence: due ? Math.round((taken * 100) / due) : null,
      daysTrained: days.filter((d) => d.mins > 0).length,
      daysUsed: u.length,
    },
  };
}

const handlers = {
  today: () => TODAY,
  get_day: getDay,
  add_log: ({ date, entry }) => {
    if (!entry.name.trim()) throw 'Give the food a name.';
    if (!(entry.kcal > 0 && entry.kcal < 5000)) throw 'Calories must be above 0 and below 5000.';
    const lid = id();
    db.log[lid] = { date, meal: meal(entry.meal), name: entry.name, grams: entry.grams, kcal: Math.round(entry.kcal), prot: r1(entry.prot), est: !!entry.est, ts: db.ts++ };
    return lid;
  },
  save_items: ({ date, items }) => {
    for (const it of items) db.log[id()] = { date, meal: meal(it.meal), name: it.name, grams: it.grams, kcal: it.kcal, prot: it.prot, est: it.est, ts: db.ts++ };
    return items.length;
  },
  delete_log: ({ id: lid }) => {
    const l = db.log[lid];
    if (l && db.plan[l.date]) for (const m of db.plan[l.date].meals) if (m.logId === lid) { m.done = false; m.logId = null; }
    delete db.log[lid];
  },
  toggle_plan: ({ date, idx }) => {
    const m = db.plan[date].meals[idx];
    if (m.done) { delete db.log[m.logId]; m.done = false; m.logId = null; }
    else { const lid = id(); db.log[lid] = { date, meal: meal(m.slot), name: m.name, grams: null, kcal: m.kcal, prot: m.prot, est: false, ts: db.ts++ }; m.done = true; m.logId = lid; }
    return m.done;
  },
  set_plan: ({ date, meals }) => { db.plan[date] = { meals: meals.map((m) => ({ ...m })) }; },
  copy_yesterday: ({ date }) => {
    const y = addDays(date, -1), ids = planIds(y);
    const rows = live('log').filter(([lid, l]) => l.date === y && !ids.has(lid));
    for (const [, l] of rows) db.log[id()] = { ...l, date, ts: db.ts++ };
    return rows.length;
  },
  recent_foods: () => {
    const seen = new Set(), out = [];
    for (const [, l] of live('log').sort((a, b) => b[1].ts - a[1].ts)) {
      if (seen.has(l.name.toLowerCase())) continue;
      seen.add(l.name.toLowerCase());
      out.push({ name: l.name, grams: l.grams, kcal: l.kcal, prot: l.prot, est: l.est });
      if (out.length === 8) break;
    }
    return out;
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
    return { kcal: Math.round(kc * g / 100), prot: r1(p * g / 100), grams: g, matched: k[0].toUpperCase() + k.slice(1) };
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
      db.plan[t] = { meals: p.meals.map((m) => ({ ...m, done: false, logId: null })) };
      n++;
    }
    return n;
  },
  get_history: history,
  get_settings: () => ({ ...db.settings }),
  save_settings: ({ settings }) => {
    if (!(settings.weightKg >= 30 && settings.weightKg <= 300)) throw 'Weight must be between 30 and 300 kg.';
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
  'plugin:updater|check': () => null,
  'plugin:dialog|open': () => '/tmp/larder-data.json',
};

export function install() {
  seed();
  window.__mock = db;
  mockIPC((cmd, args) => {
    db.calls.push(cmd);
    const fn = handlers[cmd];
    if (!fn) throw `mock: unknown command ${cmd}`;
    const out = fn(args || {});
    return out === undefined ? null : JSON.parse(JSON.stringify(out));
  });
}
