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
import { openSettings } from './screens/settings.js';
import { openRules } from './screens/rules.js';

const screens = [today, add, meds, stock, week, shop];
const main = document.getElementById('main');
const tabs = document.getElementById('tabs');
const title = document.getElementById('title');
let current = null;

for (const s of screens) {
  s.el = h('section', { class: 'screen hidden', id: `screen-${s.id}`, 'aria-label': s.title });
  main.append(s.el);
  s.tab = h('button', { type: 'button', 'data-tab': s.id, 'aria-label': s.title, onclick: () => show(s.id) }, icon(s.icon), h('span', { text: s.short || s.title }));
  tabs.append(s.tab);
}

function show(id, opts = {}) {
  const s = screens.find((x) => x.id === id) || screens[0];
  if (!s.mounted) {
    s.mount(s.el);
    s.mounted = true;
  }
  for (const x of screens) {
    x.el.classList.toggle('hidden', x !== s);
    if (x === s) x.tab.setAttribute('aria-current', 'page');
    else x.tab.removeAttribute('aria-current');
  }
  current = s;
  title.textContent = s.title;
  main.scrollTop = 0;
  return s.refresh(opts);
}
setGo(show);

const rulesBtn = document.getElementById('open-rules');
rulesBtn.append(icon('chat'));
rulesBtn.addEventListener('click', openRules);
const gear = document.getElementById('open-settings');
gear.append(icon('gear'));
gear.addEventListener('click', openSettings);

onChange(() => {
  if (current) current.refresh({ background: true });
});

async function rollDay() {
  const t = await quiet('today');
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
  if (import.meta.env.MODE === 'mock') window.__bgRefresh = () => current && current.refresh({ background: true });
  document.body.dataset.ready = '1';
}

start();
