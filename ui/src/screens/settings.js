import { h, fill, toast, openSheet, iconButton, n0, n1 } from '../dom.js';
import { call, quiet, message } from '../api.js';
import { save as saveDialog, open as openDialog } from '@tauri-apps/plugin-dialog';
import { changed } from '../state.js';

export async function openSettings() {
  const [s, ai, learned] = await Promise.all([call('get_settings'), call('get_ai_settings'), call('learned_foods')]);
  openSheet('Settings', (body) => {
    // Targets
    const weight = h('input', { type: 'number', id: 'set-weight', min: 30, max: 300, step: '0.1', value: s.weightKg, 'aria-label': 'Weight in kg' });
    const kcal = h('input', { type: 'number', id: 'set-kcal', min: 800, max: 6000, value: s.kcalTarget, 'aria-label': 'Daily kcal target' });
    const prot = h('input', { type: 'number', id: 'set-prot', min: 0, max: 400, value: s.protFloor, 'aria-label': 'Protein floor in g' });
    const tErr = h('div', { class: 'error', role: 'alert' });
    const saveTargets = async () => {
      tErr.textContent = '';
      const w = Number(weight.value), k = Number(kcal.value), p = Number(prot.value);
      if (!(w >= 30 && w <= 300)) return (tErr.textContent = 'Weight must be between 30 and 300 kg.');
      if (!(k >= 800 && k <= 6000)) return (tErr.textContent = 'Calorie target must be between 800 and 6000.');
      if (!(p >= 0 && p <= 400)) return (tErr.textContent = 'Protein floor must be between 0 and 400 g.');
      try {
        await quiet('save_settings', { settings: { ...s, weightKg: w, kcalTarget: k, protFloor: p } });
        toast('Targets saved');
        changed();
      } catch (e) {
        tErr.textContent = message(e);
      }
    };

    // AI
    const order = h('select', { id: 'ai-order', 'aria-label': 'AI order' }, h('option', { value: 'claude,gemini', text: 'Claude first, then Gemini' }), h('option', { value: 'gemini,claude', text: 'Gemini first, then Claude' }), h('option', { value: 'claude', text: 'Claude only' }), h('option', { value: 'gemini', text: 'Gemini only' }));
    order.value = ai.order.join(',');
    if (!order.value) order.value = 'claude,gemini';
    const key = h('input', { type: 'password', id: 'ai-key', autocomplete: 'off', 'aria-label': 'Gemini API key', placeholder: ai.geminiKeySet ? 'Saved — leave empty to keep' : 'Paste a key from Google AI Studio' });
    const gModel = h('input', { type: 'text', id: 'ai-gmodel', maxlength: 40, value: ai.geminiModel, 'aria-label': 'Gemini model' });
    const cModel = h('input', { type: 'text', id: 'ai-cmodel', maxlength: 40, value: ai.claudeModel, placeholder: 'default', 'aria-label': 'Claude model' });
    const cPath = h('input', { type: 'text', id: 'ai-cpath', value: ai.claudePath, placeholder: ai.claudeFound || 'not found — enter the full path', 'aria-label': 'Claude path' });
    const aiMsg = h('div', { class: 'small', role: 'status', 'data-testid': 'ai-status' });
    const modelOk = (m) => /^[A-Za-z0-9.-]{0,40}$/.test(m);
    let clearKey = false;
    const saveAi = async () => {
      aiMsg.className = 'small';
      if (!modelOk(gModel.value.trim()) || !modelOk(cModel.value.trim())) {
        aiMsg.className = 'small error';
        aiMsg.textContent = 'Model names may only use letters, digits, dots and dashes.';
        return false;
      }
      try {
        await quiet('save_ai_settings', { ai: { order: order.value.split(','), geminiKey: key.value, clearGeminiKey: clearKey, geminiModel: gModel.value.trim(), claudeModel: cModel.value.trim(), claudePath: cPath.value.trim() } });
        if (key.value || clearKey) key.placeholder = clearKey ? 'Paste a key from Google AI Studio' : 'Saved — leave empty to keep';
        key.value = '';
        clearKey = false;
        aiMsg.textContent = 'AI settings saved.';
        return true;
      } catch (e) {
        aiMsg.className = 'small error';
        aiMsg.textContent = message(e);
        return false;
      }
    };

    // Learned foods
    const learnedList = h('div', { class: 'list', id: 'learned' });
    const renderLearned = (items) =>
      fill(
        learnedList,
        items.length
          ? items.map((f) =>
              h('div', { class: 'item' },
                h('span', { class: 'name' }, h('div', { text: f.name }), h('div', { class: 'meta', text: `${n0(f.kcal100)} kcal · ${n1(f.prot100)} g per 100 g${f.piece ? ` · piece ${n0(f.piece)} g` : ''} · ${f.src}` })),
                iconButton('trash', `Forget ${f.name}`, async () => {
                  await call('delete_learned', { id: f.id });
                  renderLearned(await call('learned_foods'));
                })))
          : h('p', { class: 'muted small', text: 'Foods the AI works out are remembered here, so next time no AI is needed.' }),
      );
    renderLearned(learned);

    body.append(
      h('div', { class: 'card' },
        h('h3', { text: 'Targets' }),
        h('div', { class: 'grid3' }, h('label', { class: 'field' }, 'Weight kg', weight), h('label', { class: 'field' }, 'kcal / day', kcal), h('label', { class: 'field' }, 'Protein floor g', prot)),
        tErr,
        h('button', { type: 'button', class: 'primary', onclick: saveTargets }, 'Save targets')),
      h('div', { class: 'card' },
        h('h3', { text: 'AI (only for photos, unknown foods and the rules chat)' }),
        h('label', { class: 'field' }, 'Order', order),
        h('label', { class: 'field' }, 'Gemini key', key),
        ai.geminiKeySet ? h('button', { type: 'button', class: 'danger', onclick: async (e) => { clearKey = true; if (await saveAi()) e.target.remove(); } }, 'Remove Gemini key') : null,
        h('div', { class: 'grid2' }, h('label', { class: 'field' }, 'Gemini model', gModel), h('label', { class: 'field' }, 'Claude model', cModel)),
        h('label', { class: 'field' }, 'Claude path', cPath),
        h('div', { class: 'small muted', text: ai.claudeFound ? `Found Claude Code at ${ai.claudeFound}` : 'Claude Code was not found in the usual places.' }),
        h('div', { class: 'grid2' },
          h('button', { type: 'button', onclick: saveAi }, 'Save AI settings'),
          h('button', {
            type: 'button',
            id: 'test-ai',
            onclick: async (e) => {
              if (!(await saveAi())) return;
              e.target.disabled = true;
              aiMsg.className = 'small';
              aiMsg.textContent = 'Asking…';
              try {
                aiMsg.textContent = await quiet('test_ai');
                aiMsg.className = 'small ok';
              } catch (err) {
                aiMsg.textContent = message(err);
                aiMsg.className = 'small error';
              } finally {
                e.target.disabled = false;
              }
            },
          }, 'Test the AI')),
        aiMsg),
      h('div', { class: 'card' }, h('h3', { text: 'Learned foods' }), learnedList),
      h('div', { class: 'card' },
        h('h3', { text: 'Backup' }),
        h('p', { class: 'small muted', text: 'A backup holds all your data except the AI keys.' }),
        h('div', { class: 'grid2' },
          h('button', {
            type: 'button',
            onclick: async () => {
              const d = new Date();
              const p = (x) => String(x).padStart(2, '0');
              const path = await saveDialog({ title: 'Back up PrivateI', defaultPath: `privatei-backup-${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}.json`, filters: [{ name: 'PrivateI backup', extensions: ['json'] }] }).catch((e) => toast(message(e), 'error'));
              if (!path) return;
              await call('export_to', { path });
              toast('Backup saved');
            },
          }, 'Back up…'),
          h('button', {
            type: 'button',
            onclick: async () => {
              const path = await openDialog({ title: 'Restore or import a backup', multiple: false, directory: false, filters: [{ name: 'Backup', extensions: ['json'] }] }).catch((e) => toast(message(e), 'error'));
              if (!path) return;
              const n = await call('import_from', { path: typeof path === 'string' ? path : path.path });
              toast(`Imported ${n} record${n === 1 ? '' : 's'}`);
              changed();
            },
          }, 'Restore / import…'))),
    );
  });
}
