import '@fontsource/archivo/latin-600.css';
import '@fontsource/archivo/latin-700.css';
import '@fontsource/source-sans-3/latin-400.css';
import '@fontsource/source-sans-3/latin-600.css';
import '@fontsource/roboto-mono/latin-400.css';
import '@fontsource/roboto-mono/latin-500.css';
import './styles.css';

if (import.meta.env.MODE === 'mock') {
  const m = await import('../tests/mock-backend.js');
  m.install();
}

import { h, icon, anySheetOpen } from './dom.js';
import { quiet } from './api.js';
import { state, onChange, setGo } from './state.js';
import today from './screens/today.js';
import add from './screens/add.js';
import meds from './screens/meds.js';
import stock from './screens/stock.js';
import week from './screens/week.js';
import shop from './screens/shop.js';
import settings, { openSettings } from './screens/settings.js';
import { checkOnLaunch } from './update.js';
import { openRules, rulesPanel } from './screens/rules.js';
import { trainingPanel, trainingSummaryPanel } from './screens/training.js';
import { dosesPanel } from './screens/doses.js';
import plan from './screens/plan.js';
import history from './screens/history.js';
import { composite } from './compose.js';
import { recipesPanel } from './screens/recipes.js';

// Layouts being tried out. "current" is the shipped one until the new one is approved.
const LAYOUTS = {
  current: { name: 'Current', build: () => [today, add, meds, stock, week, shop, settings] },
  new: {
    name: 'New: Log · Plan · Kitchen · Meds · Training · History',
    build: () => [
      { ...today, title: 'Log', icon: 'book', mount: (r) => today.mount(r, { strip: true, addPanel: add }) },
      plan,
      composite({ id: 'kitchen', title: 'Kitchen', icon: 'kitchen', parts: [{ label: 'Shopping', panel: shop }, { label: 'Stock', panel: stock }, { label: 'Recipes', panel: recipesPanel() }] }),
      composite({ id: 'meds', title: 'Meds', icon: 'meds', mode: 'stack', parts: [{ panel: dosesPanel() }, { label: 'Schedules', panel: meds }] }),
      composite({ id: 'training', title: 'Training', icon: 'run', mode: 'stack', parts: [{ panel: trainingSummaryPanel() }, { panel: trainingPanel() }] }),
      history,
      composite({ id: 'settings', title: 'Settings', icon: 'gear', mode: 'stack', narrow: true, parts: [{ panel: settings }, { label: 'House rules', panel: rulesPanel() }] }),
    ],
  },
};
const params = new URLSearchParams(location.search);
let layoutId = params.get('layout') || '';
try {
  if (!layoutId) layoutId = localStorage.getItem('layout') || '';
} catch {}
if (!LAYOUTS[layoutId]) layoutId = 'current';
document.body.dataset.layout = layoutId;

const screens = LAYOUTS[layoutId].build().map((x) => (x.id === 'settings' ? { ...x, noTab: true } : x));
const tabbed = screens.filter((s) => !s.noTab);
const main = document.getElementById('main');
const tabs = document.getElementById('tabs');
const title = document.getElementById('title');
let current = null;

const MAC = /Mac/.test(navigator.platform || navigator.userAgent);
screens.forEach((s, i) => {
  s.el = h('section', { class: 'screen hidden', id: `screen-${s.id}`, 'aria-label': s.title });
  main.append(s.el);
  if (s.noTab) return;
  s.tab = h(
    'button',
    { type: 'button', 'data-tab': s.id, 'aria-label': s.title, title: `${s.title} (${MAC ? '⌘' : 'Ctrl+'}${i + 1})`, onclick: () => show(s.id) },
    icon(s.icon),
    h('span', { text: s.short || s.title }),
    h('kbd', { 'aria-hidden': 'true', text: `${MAC ? '⌘' : '^'}${i + 1}` }),
  );
  tabs.append(s.tab);
});

const dateChip = document.getElementById('date-chip');
function updateDateChip() {
  if (!state.today) return;
  const [y, m, d] = state.today.split('-').map(Number);
  dateChip.textContent = new Date(y, m - 1, d).toLocaleDateString(undefined, { weekday: 'long', day: 'numeric', month: 'long' });
}

// Desktop shortcuts: ⌘1–6 screens, ⌘, settings, ⌘K rules chat, ⌘N describe food, ←/→ step days on Today.
document.addEventListener('keydown', (e) => {
  const mod = MAC ? e.metaKey : e.ctrlKey;
  const typing = /^(INPUT|TEXTAREA|SELECT)$/.test(document.activeElement?.tagName || '');
  if (mod && !e.shiftKey && !e.altKey) {
    if (/^[1-6]$/.test(e.key) && !anySheetOpen()) {
      e.preventDefault();
      show(tabbed[Number(e.key) - 1].id);
    } else if (e.key === ',' && !anySheetOpen()) {
      e.preventDefault();
      openSettings();
    } else if (e.key.toLowerCase() === 'k' && !anySheetOpen()) {
      e.preventDefault();
      openRules();
    } else if (e.key.toLowerCase() === 'n' && !anySheetOpen()) {
      e.preventDefault();
      if (screens.some((x) => x.id === 'add')) show('add').then(() => document.getElementById('describe')?.focus());
      else show('today').then(() => today.openAdd());
    }
    return;
  }
  if (!typing && !anySheetOpen() && current?.id === 'today' && (e.key === 'ArrowLeft' || e.key === 'ArrowRight')) {
    const btn = document.querySelector(e.key === 'ArrowLeft' ? '[aria-label="Previous day"]' : '[aria-label="Next day"]');
    if (btn && !btn.disabled) {
      e.preventDefault();
      btn.click();
    }
  }
});

function show(id, opts = {}) {
  const s = screens.find((x) => x.id === id) || screens[0];
  if (!s.mounted) {
    s.mount(s.el);
    s.mounted = true;
  }
  for (const x of screens) {
    x.el.classList.toggle('hidden', x !== s);
    if (!x.tab) continue;
    if (x === s) x.tab.setAttribute('aria-current', 'page');
    else x.tab.removeAttribute('aria-current');
  }
  current = s;
  title.textContent = s.title;
  if (s.id !== 'today') updateDateChip();
  main.scrollTop = 0;
  return s.refresh(opts);
}
setGo(show);

const rulesBtn = document.getElementById('open-rules');
rulesBtn.append(icon('chat'));
if (layoutId !== 'current') rulesBtn.classList.add('hidden'); // rules live in Settings
rulesBtn.addEventListener('click', openRules);
const gear = document.getElementById('open-settings');
gear.append(icon('gear'));
gear.addEventListener('click', openSettings);

onChange(() => {
  if (current) current.refresh({ background: true });
});

async function rollDay() {
  const t = await quiet('today');
  queueMicrotask(() => { if (current?.id !== 'today') updateDateChip(); });
  if (t !== state.today) {
    const wasToday = state.date === state.today;
    state.today = t;
    if (wasToday || !state.date) state.date = t;
    return true;
  }
  return false;
}

async function start() {
  await rollDay();
  await show('today');
  // Keep "today" and overdue doses current. Only read-only parts re-render.
  setInterval(async () => {
    const rolled = await rollDay().catch(() => false);
    if (current && !anySheetOpen() && (rolled || current.id === 'today')) current.refresh({ background: true });
  }, 60_000);
  if (import.meta.env.MODE === 'mock') {
    window.__bgRefresh = () => current && current.refresh({ background: true });
    // Timing hook for UI tests: switch tab, wait for its data and render, force layout.
    window.__showTimed = async (id) => {
      const t0 = performance.now();
      await show(id);
      void document.body.offsetHeight;
      return performance.now() - t0;
    };
  }
  if (params.has('layout')) layoutSwitcher();
  document.body.dataset.ready = '1';
  setTimeout(checkOnLaunch, 3000);
}

/** Preview-only: switch between the layouts being compared. */
function layoutSwitcher() {
  const bar = h('div', { class: 'layout-switcher', role: 'group', 'aria-label': 'Layout to try' });
  for (const [id, l] of Object.entries(LAYOUTS)) {
    bar.append(h('button', {
      type: 'button',
      'aria-pressed': String(id === layoutId),
      onclick: () => {
        const p = new URLSearchParams(location.search);
        p.set('layout', id);
        location.search = p.toString();
      },
    }, l.name));
  }
  document.body.append(bar);
}

start();
