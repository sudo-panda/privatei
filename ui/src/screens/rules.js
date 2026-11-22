import { h, fill, toast, openSheet } from '../dom.js';
import { call, quiet, message } from '../api.js';

export async function openRules() {
  const rules = await call('get_rules');
  openSheet('House rules', (body) => buildRules(body, rules));
}

/** House rules inline (Settings), same chat as the sheet. */
export function rulesPanel() {
  let box;
  return {
    id: 'rules', title: 'House rules', icon: 'chat',
    mount(root) { box = h('div', { class: 'rules-inline' }); root.append(box); },
    async refresh(o = {}) {
      if (o.background && box.childElementCount) return;
      fill(box);
      buildRules(box, await call('get_rules'));
    },
  };
}

function buildRules(body, initial) {
  let rules = initial;
  {
    const current = h('div', { class: 'card' });
    const showRules = () =>
      fill(current, h('h3', { text: 'Current rules' }), h('div', { class: 'pre small', 'data-testid': 'rules-text', text: rules || 'No rules yet.' }));
    showRules();
    const log = h('div', { style: 'display:flex;flex-direction:column;gap:8px', 'aria-live': 'polite' });
    const input = h('textarea', { id: 'rules-input', maxlength: 1000, rows: 2, 'aria-label': 'Message about your rules', placeholder: 'e.g. never more than 2 eggs a day' });
    const send = h('button', { type: 'button', class: 'primary' }, 'Send');
    const go = async () => {
      const msg = input.value.trim();
      if (!msg) return;
      log.append(h('div', { class: 'bubble me', text: msg }));
      input.value = '';
      send.disabled = true;
      const wait = h('div', { class: 'bubble muted', text: 'Thinking…' });
      log.append(wait);
      try {
        const r = await quiet('rules_chat', { message: msg });
        wait.remove();
        log.append(reply(r));
      } catch (e) {
        wait.className = 'bubble error';
        wait.textContent = message(e);
      } finally {
        send.disabled = false;
      }
    };
    const reply = (r) => {
      const box = h('div', { class: 'bubble' }, h('div', { text: r.reply }), h('div', { class: 'small muted', text: r.providerText }));
      if (r.change) {
        const diff = h('div', { class: 'diff', 'aria-label': 'Proposed change' },
          r.change.remove ? h('div', { class: 'del', text: `− ${r.change.remove}` }) : null,
          r.change.add ? h('div', { class: 'add', text: `+ ${r.change.add}` }) : null);
        const apply = h('button', {
          type: 'button',
          class: 'primary',
          onclick: async () => {
            rules = await call('apply_rule_change', { change: r.change });
            showRules();
            apply.disabled = true;
            apply.textContent = 'Applied';
            toast('Rules updated');
          },
        }, 'Apply');
        box.append(diff, apply);
      }
      return box;
    };
    send.addEventListener('click', go);
    input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        go();
      }
    });
    body.append(current, h('p', { class: 'small muted', text: 'Tell the AI how you want food logged. Changes are only saved when you tap Apply.' }), log, input, send);
  }
}
