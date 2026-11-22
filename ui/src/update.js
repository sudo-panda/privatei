// Auto-update via the Tauri updater: checks GitHub releases' latest.json, which is signed.
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { getVersion } from '@tauri-apps/api/app';
import { h, toast } from './dom.js';
import { message } from './api.js';

export const appVersion = () => getVersion().catch(() => '');

/** Returns the pending update, or null when up to date. Throws on network errors. */
export const findUpdate = () => check();

export async function install(update, onProgress) {
  let total = 0;
  let got = 0;
  await update.downloadAndInstall((e) => {
    if (e.event === 'Started') total = e.data.contentLength || 0;
    if (e.event === 'Progress') {
      got += e.data.chunkLength;
      if (onProgress && total) onProgress(Math.round((got / total) * 100));
    }
  });
  await relaunch();
}

/** Quiet check on launch: shows a banner only when an update exists. */
export async function checkOnLaunch() {
  let update = null;
  try {
    update = await findUpdate();
  } catch {
    return; // offline or GitHub unreachable: try again next launch
  }
  if (!update) return;
  const main = document.getElementById('main');
  const btn = h('button', { type: 'button', class: 'primary' }, 'Install and restart');
  const banner = h(
    'div',
    { class: 'card update-banner', role: 'status', id: 'update-banner' },
    h('div', { class: 'row spread wrap' },
      h('span', { class: 'grow', text: `PrivateI ${update.version} is available.` }),
      h('div', { class: 'row' }, h('button', { type: 'button', class: 'ghost', onclick: () => banner.remove() }, 'Later'), btn)),
  );
  btn.addEventListener('click', async () => {
    btn.disabled = true;
    btn.textContent = 'Downloading…';
    try {
      await install(update, (p) => (btn.textContent = `Downloading… ${p}%`));
    } catch (e) {
      btn.disabled = false;
      btn.textContent = 'Install and restart';
      toast(`Update failed: ${message(e)}`, 'error');
    }
  });
  main.prepend(banner);
}
