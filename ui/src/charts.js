// Small hand-made charts (no libraries). Data comes from privatei-core; this only draws.
import { h } from './dom.js';

const SVG = 'http://www.w3.org/2000/svg';
const s = (tag, attrs = {}) => {
  const el = document.createElementNS(SVG, tag);
  for (const [k, v] of Object.entries(attrs)) el.setAttribute(k, String(v));
  return el;
};

/** A bar with a 3px rounded top, square at the baseline. */
function barPath(x, y, w, hgt) {
  const r = Math.min(3, w / 2, hgt);
  return `M${x},${y + hgt}V${y + r}Q${x},${y} ${x + r},${y}H${x + w - r}Q${x + w},${y} ${x + w},${y + r}V${y + hgt}Z`;
}

/**
 * Vertical bars over time with an optional reference line.
 * opts: { data: [{ label, tip, v, tone }], ref, refLabel, height }
 * tone: '' (series colour), 'over' (above the line), 'muted' (no data).
 */
export function barChart(opts) {
  const box = h('div', { class: 'chart', role: 'img', 'aria-label': opts.title || 'Chart' });
  const tip = h('div', { class: 'chart-tip hidden', role: 'status' });
  box.append(tip);
  const draw = () => {
    const W = Math.max(120, box.clientWidth || 600);
    const H = opts.height || 150;
    const pad = { l: 4, r: 4, t: 18, b: 20 };
    const n = Math.max(1, opts.data.length);
    const max = Math.max(1, (opts.ref || 0) * 1.25, ...opts.data.map((d) => d.v || 0));
    const iw = W - pad.l - pad.r;
    const ih = H - pad.t - pad.b;
    const step = iw / n;
    const bw = Math.max(2, step - 2);
    const y = (v) => pad.t + ih - (v / max) * ih;
    box.querySelector('svg')?.remove();
    const svg = s('svg', { width: W, height: H, viewBox: `0 0 ${W} ${H}` });
    svg.append(s('line', { x1: pad.l, x2: W - pad.r, y1: pad.t + ih, y2: pad.t + ih, class: 'chart-axis' }));
    opts.data.forEach((d, i) => {
      const x = pad.l + i * step + (step - bw) / 2;
      const v = d.v || 0;
      const hgt = Math.max(v > 0 ? 2 : 0, pad.t + ih - y(v));
      const g = s('g', { class: 'chart-hit' });
      g.append(s('rect', { x: pad.l + i * step, y: pad.t, width: step, height: ih, fill: 'transparent' }));
      if (hgt > 0) g.append(s('path', { d: barPath(x, pad.t + ih - hgt, bw, hgt), class: `chart-bar ${d.tone || ''}` }));
      g.addEventListener('mouseenter', () => {
        tip.textContent = d.tip;
        tip.classList.remove('hidden');
        const left = Math.min(Math.max(0, x + bw / 2 - 70), W - 140);
        tip.style.left = `${left}px`;
      });
      g.addEventListener('mouseleave', () => tip.classList.add('hidden'));
      svg.append(g);
      // A date label roughly every 7 bars, plus the last one.
      if (d.label && (i % Math.ceil(n / 8) === 0 || i === n - 1)) {
        svg.append(Object.assign(s('text', { x: x + bw / 2, y: H - 5, 'text-anchor': 'middle', class: 'chart-label' }), { textContent: d.label }));
      }
    });
    if (opts.ref) {
      const ry = y(opts.ref);
      svg.append(s('line', { x1: pad.l, x2: W - pad.r, y1: ry, y2: ry, class: 'chart-ref' }));
      svg.append(Object.assign(s('text', { x: W - pad.r, y: ry - 5, 'text-anchor': 'end', class: 'chart-label' }), { textContent: opts.refLabel || '' }));
    }
    box.prepend(svg);
  };
  // Draw once laid out, and again when the width changes.
  const ro = new ResizeObserver(() => draw());
  ro.observe(box);
  requestAnimationFrame(draw);
  return box;
}

/**
 * Horizontal percentage bars against a 100% marker.
 * rows: [{ label, value (0..), text, tone }]
 */
export function pctBars(rows, { max = 150, title } = {}) {
  return h(
    'div',
    { class: 'pct-bars', role: 'list', 'aria-label': title || 'Bars' },
    rows.map((r) =>
      h('div', { class: 'pct-row', role: 'listitem' },
        h('span', { class: 'pct-name', text: r.label }),
        h('div', { class: 'pct-track', title: r.text },
          h('span', { class: `pct-fill ${r.tone || ''}`, style: `width:${Math.min(100, (Math.max(0, r.value) / max) * 100)}%` }),
          h('span', { class: 'pct-mark', style: `left:${(100 / max) * 100}%` })),
        h('span', { class: 'pct-val num small', text: r.text }))),
  );
}
