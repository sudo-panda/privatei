// Combine panels into one screen: side by side sections ("stack") or a segmented switcher.
import { h } from './dom.js';

export function composite({ id, title, icon, short, parts, mode = 'segment', narrow = false }) {
  let active = 0;
  let seg;
  const s = {
    id, title, icon, short,
    mount(root) {
      if (mode === 'segment') {
        seg = h('div', { class: 'segment', role: 'group', 'aria-label': title });
        parts.forEach((p, i) =>
          seg.append(h('button', { type: 'button', 'data-part': p.label, 'aria-pressed': String(i === 0), onclick: () => select(i) }, p.label)));
        root.append(h('div', { class: 'toolbar' }, seg));
      }
      const wrap = h('div', { class: narrow ? 'settings' : 'screen' });
      root.append(wrap);
      for (const p of parts) {
        p.box = mode === 'stack' && p.label
          ? h('section', { class: 'set-section', 'aria-label': p.label }, h('h2', { text: p.label }))
          : h('div', { class: mode === 'stack' ? 'set-section' : 'screen' });
        if (mode === 'segment') p.box.classList.toggle('hidden', p !== parts[0]);
        wrap.append(p.box);
      }
    },
    refresh(o = {}) {
      // o.part ("Shopping") opens that part, e.g. "Open trip" from the Plan.
      if (o.part && mode === 'segment') {
        const i = parts.findIndex((p) => p.label === o.part);
        if (i >= 0 && i !== active) {
          active = i;
          parts.forEach((p, j) => p.box.classList.toggle('hidden', j !== i));
          for (const b of seg.children) b.setAttribute('aria-pressed', String(b.dataset.part === parts[i].label));
        }
      }
      const visible = mode === 'segment' ? [parts[active]] : parts;
      return Promise.all(visible.map((p) => {
        if (!p.mounted) { p.panel.mount(p.box); p.mounted = true; }
        return p.panel.refresh(o);
      }));
    },
  };
  function select(i) {
    active = i;
    parts.forEach((p, j) => p.box.classList.toggle('hidden', j !== i));
    for (const b of seg.children) b.setAttribute('aria-pressed', String(b.dataset.part === parts[i].label));
    s.refresh();
  }
  return s;
}
