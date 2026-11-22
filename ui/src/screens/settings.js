import { h, fill, toast, iconButton, n0, n1 } from '../dom.js';
import { call, quiet, message } from '../api.js';
import { save as saveDialog, open as openDialog } from '@tauri-apps/plugin-dialog';
import { changed, go } from '../state.js';
import { appVersion, findUpdate, install } from '../update.js';

const CLAUDE_MODELS = [
  ['', 'Default (your Claude Code setting)'],
  ['sonnet', 'Sonnet — latest'],
  ['opus', 'Opus — latest, most capable'],
  ['haiku', 'Haiku — latest, fastest'],
];
const GEMINI_MODELS = [
  ['gemini-flash-latest', 'Gemini Flash — latest'],
  ['gemini-flash-lite-latest', 'Gemini Flash-Lite — latest, fastest'],
  ['gemini-pro-latest', 'Gemini Pro — latest, most capable'],
];
const ORDERS = [
  ['claude,gemini', 'Claude, then Gemini'],
  ['gemini,claude', 'Gemini, then Claude'],
  ['claude', 'Only Claude'],
  ['gemini', 'Only Gemini'],
];
const CUSTOM = '__custom';
const modelOk = (m) => /^[A-Za-z0-9.-]{0,40}$/.test(m);

let els = {};
let s = null;
let ai = null;

function row(label, hint, ...controls) {
  return h(
    'div',
    { class: 'set-row' },
    h('div', { class: 'set-label' }, h('div', { class: 'set-name', text: label }), hint ? h('div', { class: 'small muted', text: hint }) : null),
    h('div', { class: 'set-control' }, ...controls),
  );
}

function section(title, ...rows) {
  return h('section', { class: 'set-section', 'aria-label': title }, h('h2', { text: title }), h('div', { class: 'card' }, ...rows));
}

function select(id, label, options, value) {
  const el = h('select', { id, 'aria-label': label }, options.map(([v, t]) => h('option', { value: v, text: t })));
  el.value = value;
  return el;
}

/** A dropdown of known models plus "Other…" that reveals a text field. */
function modelPicker(id, label, options, value, onChange) {
  const known = options.some(([v]) => v === value);
  const sel = select(id, label, [...options, [CUSTOM, 'Other…']], known ? value : CUSTOM);
  const custom = h('input', { type: 'text', id: `${id}-custom`, maxlength: 40, value: known ? '' : value, placeholder: 'Model name', 'aria-label': `${label} name`, class: known ? 'hidden' : '' });
  const current = () => (sel.value === CUSTOM ? custom.value.trim() : sel.value);
  sel.addEventListener('change', () => {
    custom.classList.toggle('hidden', sel.value !== CUSTOM);
    if (sel.value === CUSTOM) custom.focus();
    else onChange(current());
  });
  custom.addEventListener('change', () => onChange(current()));
  return h('div', { class: 'stack' }, sel, custom);
}

function status(id) {
  return h('div', { class: 'small set-status', id, role: 'status' });
}
function say(el, text, kind = '') {
  el.className = `small set-status ${kind}`;
  el.textContent = text;
}

function mount(root) {
  els.targets = h('div');
  els.ai = h('div');
  els.learned = h('div');
  els.backup = h('div');
  els.updates = h('div');
  root.append(h('div', { class: 'settings' }, els.targets, els.ai, els.learned, els.backup, els.updates));
}

async function refresh(opts = {}) {
  if (opts.background) return; // never rebuild fields while you might be typing
  [s, ai] = await Promise.all([call('get_settings'), call('get_ai_settings')]);
  renderTargets();
  renderAi();
  renderLearned(await call('learned_foods'));
  renderBackup();
  renderUpdates();
}

// ---------- Targets (saved as you leave each field) ----------

function renderTargets() {
  const st = status('targets-status');
  const field = (id, label, value, min, max, step, unit) =>
    h('div', { class: 'with-unit' }, h('input', { type: 'number', id, min, max, step, value, 'aria-label': label, inputmode: 'decimal' }), h('span', { class: 'muted small', text: unit }));
  const weight = field('set-weight', 'Weight', s.weightKg, 30, 300, '0.1', 'kg');
  const kcal = field('set-kcal', 'Daily calories', s.kcalTarget, 800, 6000, '10', 'kcal');
  const prot = field('set-prot', 'Protein floor', s.protFloor, 0, 400, '1', 'g');
  const save = async () => {
    const w = Number(weight.firstChild.value), k = Number(kcal.firstChild.value), p = Number(prot.firstChild.value);
    if (!(w >= 30 && w <= 300)) return say(st, 'Weight must be between 30 and 300 kg.', 'error');
    if (!(k >= 800 && k <= 6000)) return say(st, 'Daily calories must be between 800 and 6000.', 'error');
    if (!(p >= 0 && p <= 400)) return say(st, 'Protein floor must be between 0 and 400 g.', 'error');
    if (w === s.weightKg && k === s.kcalTarget && p === s.protFloor) return say(st, '');
    try {
      await quiet('save_settings', { settings: { ...s, weightKg: w, kcalTarget: k, protFloor: p } });
      s = { ...s, weightKg: w, kcalTarget: k, protFloor: p };
      say(st, 'Saved', 'ok');
      changed();
    } catch (e) {
      say(st, message(e), 'error');
    }
  };
  for (const f of [weight, kcal, prot]) f.firstChild.addEventListener('change', save);
  fill(
    els.targets,
    section(
      'Targets',
      row('Weight', 'Used for training burn.', weight),
      row('Daily calories', 'Your energy target.', kcal),
      row('Protein floor', 'The minimum to hit each day.', prot),
      st,
    ),
  );
}

// ---------- AI ----------

function renderAi() {
  const st = status('ai-status');
  st.setAttribute('data-testid', 'ai-status');
  const saveAi = async (patch) => {
    const next = {
      order: ai.order,
      geminiKey: '',
      clearGeminiKey: false,
      geminiModel: ai.geminiModel,
      claudeModel: ai.claudeModel,
      claudePath: ai.claudePath,
      ...patch,
    };
    if (!modelOk(next.geminiModel) || !modelOk(next.claudeModel)) {
      say(st, 'Model names may only use letters, digits, dots and dashes.', 'error');
      return false;
    }
    try {
      await quiet('save_ai_settings', { ai: next });
      ai = await call('get_ai_settings');
      say(st, 'Saved', 'ok');
      return true;
    } catch (e) {
      say(st, message(e), 'error');
      return false;
    }
  };

  const order = select('ai-order', 'Which AI to try first', ORDERS, ai.order.join(','));
  order.addEventListener('change', () => saveAi({ order: order.value.split(',') }));

  const claudeModel = modelPicker('ai-cmodel', 'Claude model', CLAUDE_MODELS, ai.claudeModel, (m) => saveAi({ claudeModel: m }));
  const geminiModel = modelPicker('ai-gmodel', 'Gemini model', GEMINI_MODELS, ai.geminiModel || 'gemini-flash-latest', (m) => saveAi({ geminiModel: m || 'gemini-flash-latest' }));

  // Gemini key: never shown again once saved.
  const keyBox = h('div', { class: 'stack' });
  const renderKey = () => {
    if (ai.geminiKeySet) {
      fill(keyBox, h('div', { class: 'row' }, h('span', { class: 'badge good', text: 'Key saved' }), h('button', { type: 'button', class: 'ghost danger', onclick: async () => { if (await saveAi({ clearGeminiKey: true })) renderKey(); } }, 'Remove')));
    } else {
      const key = h('input', { type: 'password', id: 'ai-key', autocomplete: 'off', 'aria-label': 'Gemini API key', placeholder: 'Paste a key from Google AI Studio' });
      const add = h('button', { type: 'button', onclick: async () => { if (!key.value.trim()) return; if (await saveAi({ geminiKey: key.value.trim() })) renderKey(); } }, 'Save key');
      key.addEventListener('keydown', (e) => e.key === 'Enter' && add.click());
      fill(keyBox, h('div', { class: 'row' }, h('div', { class: 'grow' }, key), add));
    }
  };
  renderKey();

  const path = h('input', { type: 'text', id: 'ai-cpath', value: ai.claudePath, placeholder: ai.claudeFound || '/opt/homebrew/bin/claude', 'aria-label': 'Claude path' });
  path.addEventListener('change', () => saveAi({ claudePath: path.value.trim() }));
  const found = ai.claudeFound
    ? h('span', { class: 'badge good', title: ai.claudeFound, text: 'Found' })
    : h('span', { class: 'badge warn', text: 'Not found' });

  const test = h('button', { type: 'button', id: 'test-ai' }, 'Test the AI');
  test.addEventListener('click', async () => {
    test.disabled = true;
    say(st, 'Asking…');
    try {
      say(st, await quiet('test_ai'), 'ok');
    } catch (e) {
      say(st, message(e), 'error');
    } finally {
      test.disabled = false;
    }
  });

  fill(
    els.ai,
    section(
      'AI',
      h('p', { class: 'small muted set-intro', text: 'Only used for photos, foods the tables don’t know, and the rules chat.' }),
      row('Try first', null, order),
      row('Claude Code', ai.claudeFound ? ai.claudeFound : 'Install Claude Code and run it once in Terminal.', found),
      row('Claude model', null, claudeModel),
      row('Gemini key', 'Optional fallback.', keyBox),
      row('Gemini model', null, geminiModel),
      h('details', { class: 'set-more' }, h('summary', { class: 'small', text: 'Use a different claude' }), row('Claude path', 'Full path to the claude program.', path)),
      h('div', { class: 'set-row' }, h('div', { class: 'set-label' }, st), h('div', { class: 'set-control' }, test)),
    ),
  );
}

// ---------- Learned foods ----------

function renderLearned(items) {
  let all = false;
  const list = h('div', { class: 'list', id: 'learned' });
  const draw = () => {
    const shown = all ? items : items.slice(0, 6);
    fill(
      list,
      items.length
        ? shown.map((f) =>
            h('div', { class: 'item' },
              h('span', { class: 'name' }, h('div', { text: f.name }), h('div', { class: 'meta', text: `${n0(f.kcal100)} kcal · ${n1(f.prot100)} g protein per 100 g${f.piece ? ` · piece ${n0(f.piece)} g` : ''}` })),
              iconButton('trash', `Forget ${f.name}`, async () => {
                await call('delete_learned', { id: f.id });
                renderLearned(await call('learned_foods'));
              })))
        : h('p', { class: 'muted small', text: 'Foods the AI works out are remembered here, so next time no AI is needed.' }),
      items.length > 6 && !all ? h('button', { type: 'button', class: 'ghost', onclick: () => { all = true; draw(); } }, `Show all ${items.length}`) : null,
    );
  };
  draw();
  fill(els.learned, section('Learned foods', list));
}

// ---------- Backup ----------

function renderBackup() {
  const backup = h('button', {
    type: 'button',
    onclick: async () => {
      const d = new Date();
      const p = (x) => String(x).padStart(2, '0');
      const path = await saveDialog({ title: 'Back up PrivateI', defaultPath: `privatei-backup-${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}.json`, filters: [{ name: 'PrivateI backup', extensions: ['json'] }] }).catch((e) => toast(message(e), 'error'));
      if (!path) return;
      await call('export_to', { path });
      toast('Backup saved');
    },
  }, 'Back up…');
  const restore = h('button', {
    type: 'button',
    onclick: async () => {
      const path = await openDialog({ title: 'Restore or import a backup', multiple: false, directory: false, filters: [{ name: 'Backup', extensions: ['json'] }] }).catch((e) => toast(message(e), 'error'));
      if (!path) return;
      const n = await call('import_from', { path: typeof path === 'string' ? path : path.path });
      toast(`Imported ${n} record${n === 1 ? '' : 's'}`);
      changed();
      refresh();
    },
  }, 'Restore / import…');
  fill(
    els.backup,
    section(
      'Backup',
      row('Save a backup', 'All your data except AI keys.', backup),
      row('Restore or import', 'A PrivateI backup file.', restore),
    ),
  );
}

// ---------- Updates ----------

async function renderUpdates() {
  const v = await appVersion();
  const st = status('update-status');
  const btn = h('button', { type: 'button', id: 'check-updates' }, 'Check for updates');
  btn.addEventListener('click', async () => {
    btn.disabled = true;
    say(st, 'Checking…');
    try {
      const u = await findUpdate();
      if (!u) {
        say(st, 'You’re up to date.', 'ok');
        btn.disabled = false;
        return;
      }
      say(st, `Version ${u.version} is available.`);
      btn.textContent = 'Install and restart';
      btn.disabled = false;
      btn.onclick = async (e) => {
        e.stopImmediatePropagation();
        btn.disabled = true;
        try {
          await install(u, (p) => say(st, `Downloading… ${p}%`));
        } catch (err) {
          say(st, `Update failed: ${message(err)}`, 'error');
          btn.disabled = false;
        }
      };
    } catch (e) {
      say(st, `Couldn’t check: ${message(e)}`, 'error');
      btn.disabled = false;
    }
  });
  fill(els.updates, section('Updates', row('PrivateI', v ? `Version ${v}` : null, btn), st));
}

export function openSettings() {
  go('settings');
}

export default { id: 'settings', title: 'Settings', icon: 'gear', mount, refresh, noTab: true };
