import { test, expect } from '@playwright/test';

// The new layout is the app; tests written for the older one ask for it by name.
async function open(page, query = 'seed=demo') {
  if (!/(^|&)layout=/.test(query)) query += '&layout=current';
  await page.goto(`/?${query}`);
  await expect(page.locator('body[data-ready="1"]')).toBeAttached();
}
const tab = (page, name) => page.locator(`nav.tabs button[data-tab="${name}"]`);

test.describe('Today', () => {
  test('plan ticks, meds, day stepping and copy yesterday', async ({ page }) => {
    await open(page);
    await expect(page.getByTestId('kcal')).toHaveText('0');
    await page.getByLabel('Ate Tofu bowl').check();
    await expect(page.getByTestId('kcal')).toHaveText('520');
    await page.getByLabel('Ate Tofu bowl').uncheck();
    await expect(page.getByTestId('kcal')).toHaveText('0');

    const vitD = page.locator('[data-med]', { hasText: 'Vitamin D' });
    await expect(vitD.getByText('overdue')).toBeVisible();
    await page.getByLabel('Took Vitamin D').check();
    await expect(vitD.getByText('overdue')).toHaveCount(0);

    await expect(page.getByLabel('Next day')).toBeDisabled();
    await page.getByLabel('Previous day').click();
    await expect(page.locator('#day-label')).not.toHaveText('Today');
    await expect(page.getByTestId('kcal')).toHaveText('374');
    await expect(page.locator('[data-med]', { hasText: 'Magnesium' }).getByText('missed')).toBeVisible();
    await page.getByRole('button', { name: 'Back to today' }).click();
    await expect(page.locator('#day-label')).toHaveText('Today');

    await page.getByRole('button', { name: "Copy yesterday's food" }).click();
    await expect(page.getByTestId('kcal')).toHaveText('374');
    await expect(page.getByRole('button', { name: "Copy yesterday's food" })).toHaveCount(0);
    // Delete a row
    // Tap a row: one editor with Save and Delete.
    await page.getByLabel('Edit Chicken breast · 150 g').click();
    await page.locator('#ed-grams').fill('300');
    await expect(page.locator('#ed-kcal')).toHaveValue('496');
    await page.getByRole('dialog').getByRole('button', { name: 'Save' }).click();
    await expect(page.getByTestId('kcal')).toHaveText('622');
    await page.getByLabel('Edit Skyr · 200 g').click();
    await page.getByRole('dialog').getByRole('button', { name: 'Delete' }).click();
    await expect(page.getByTestId('kcal')).toHaveText('496');
  });

  test('add training with estimate, burn toggle and paste import', async ({ page }) => {
    await open(page);
    await page.getByRole('button', { name: 'Add training' }).click();
    await page.getByRole('button', { name: 'Run', exact: true }).click();
    await page.locator('#tr-mins').fill('30');
    await expect(page.getByTestId('burn-estimate')).toHaveText('Estimate: 422 kcal');
    await page.getByRole('button', { name: 'Save training' }).click();
    await expect(page.locator('.card[aria-label="Training"]')).toContainText('422 kcal');
    const toggle = page.getByLabel('Add training burn to target');
    await toggle.check();
    await expect(page.locator('.card[aria-label="Energy and protein"]')).toContainText('/ 1922 kcal');

    await page.getByRole('button', { name: 'Add training' }).click();
    await page.getByText('Paste from another app').click();
    await page.locator('#tr-paste').fill('Date,Sport,Minutes\n2026-11-21,Gym,45\n2026-12-30,Run,30');
    await page.getByRole('button', { name: 'Import pasted sessions' }).click();
    await expect(page.locator('.toast').last()).toContainText('Added 1 session, skipped 2 lines');
  });
});

test.describe('Add', () => {
  test('describe → local + AI draft → save', async ({ page }) => {
    await open(page);
    await tab(page, 'add').click();
    await page.locator('#describe').fill('200g chicken breast, zorblax casserole');
    await page.getByRole('button', { name: 'Work out the calories' }).click();
    const draft = page.locator('#draft');
    await expect(draft.getByTestId('provider')).toHaveText('Claude worked it out');
    await expect(draft.locator('[data-draft]')).toHaveCount(2);
    // Nothing in the databases is called that, so it stays an AI estimate, marked as such.
    await expect(draft.locator('[data-draft="1"]')).toContainText('AI estimate');
    await draft.getByLabel('Remove zorblax casserole').click();
    await expect(draft.getByTestId('draft-total')).toContainText('330 kcal · 62 g protein');
    await draft.getByRole('button', { name: 'Save' }).click();
    await expect(page.locator('#draft')).toBeHidden();
    await tab(page, 'today').click();
    await expect(page.getByTestId('kcal')).toHaveText('330');
  });

  test('without AI it shows what it placed and names the rest', async ({ page }) => {
    await open(page);
    await page.evaluate(() => (window.__mock.aiAvailable = false));
    await tab(page, 'add').click();
    await page.locator('#describe').fill('2 eggs and some mystery stew');
    await page.getByRole('button', { name: 'Work out the calories' }).click();
    await expect(page.locator('[data-draft]')).toHaveCount(1);
    await expect(page.getByTestId('unknown')).toContainText('some mystery stew');
    await expect(page.locator('#draft')).toContainText('No AI is set up');
  });

  test('photo uses AI', async ({ page }) => {
    await open(page);
    await tab(page, 'add').click();
    await page.locator('#photo').setInputFiles({ name: 'meal.jpg', mimeType: 'image/jpeg', buffer: Buffer.from([0xff, 0xd8, 0xff, 0xd9]) });
    await expect(page.getByTestId('provider')).toHaveText('Gemini worked it out');
    await expect(page.locator('#draft')).toContainText('Curry with rice');
  });

  test('manual form autofills, validates and logs; recent chip relogs', async ({ page }) => {
    await open(page);
    await tab(page, 'add').click();
    await page.locator('#m-name').fill('Egg');
    await expect(page.locator('#m-kcal')).toHaveValue('79');
    await page.locator('#m-grams').fill('110');
    await expect(page.locator('#m-kcal')).toHaveValue('157');
    await page.locator('#m-kcal').fill('150'); // my own number wins
    await page.locator('#m-grams').fill('120');
    await expect(page.locator('#m-kcal')).toHaveValue('150');
    await page.locator('#m-kcal').fill('');
    await page.getByRole('button', { name: 'Log it' }).click();
    await expect(page.locator('#manual .error')).toHaveText('kcal must be above 0 and below 5000.');
    await page.locator('#m-kcal').fill('150');
    await page.getByRole('button', { name: 'Log it' }).click();
    await expect(page.locator('#m-name')).toHaveValue('');
    await expect(page.getByLabel('Add Egg to the draft')).toBeVisible();
    await page.getByLabel('Add Egg to the draft').click();
    await page.getByLabel('Add Egg to the draft').click(); // twice: two items in the draft
    await expect(page.locator('#draft [data-draft]')).toHaveCount(2);
    await page.locator('#draft').getByLabel('Remove Egg').first().click();
    await page.locator('#draft').getByRole('button', { name: 'Save' }).click();
    await tab(page, 'today').click();
    await expect(page.getByTestId('kcal')).toHaveText('300');
  });

  test('typing never loses focus or text on a background refresh', async ({ page }) => {
    await open(page);
    await tab(page, 'add').click();
    await page.locator('#m-name').click();
    await page.keyboard.type('Ski');
    await page.evaluate(() => window.__bgRefresh());
    await page.keyboard.type('r');
    await expect(page.locator('#m-name')).toHaveValue('Skir');
    await expect(page.locator('#m-name')).toBeFocused();
    await page.locator('#describe').fill('half a ');
    await page.locator('#describe').focus();
    await page.evaluate(() => window.__bgRefresh());
    await page.keyboard.type('banana');
    await expect(page.locator('#describe')).toHaveValue('half a banana');
  });
});

test('Meds: add, change schedule note, stop and erase', async ({ page }) => {
  await open(page);
  await tab(page, 'meds').click();
  await page.getByRole('button', { name: 'Add medicine' }).click();
  await page.locator('#med-name').fill('Iron');
  await page.locator('#med-dose').fill('1 tablet');
  await page.getByRole('button', { name: 'Certain days' }).click();
  await page.getByRole('button', { name: 'Save' }).click();
  await expect(page.locator('.sheet .error')).toHaveText('Pick at least one day.');
  await page.getByRole('button', { name: 'Mon', exact: true }).click();
  await page.getByRole('button', { name: 'Thu', exact: true }).click();
  await page.locator('#med-time').fill('07:30');
  await page.getByRole('button', { name: 'Save' }).click();
  const days = page.locator('.card[aria-label="Certain days"]');
  await expect(days).toContainText('Iron');
  await expect(days).toContainText('Mon, Thu');

  await page.getByLabel('Edit Vitamin D').click();
  await expect(page.getByTestId('new-schedule-note')).toBeHidden();
  await page.locator('#med-time').fill('09:00');
  await expect(page.getByTestId('new-schedule-note')).toBeVisible();
  await page.getByRole('button', { name: 'Save' }).click();
  await expect(page.locator('.toast').last()).toHaveText('Saved — new schedule from today');

  await page.getByLabel('Edit Iron').click();
  await page.getByRole('button', { name: 'Stop taking' }).click();
  await expect(page.locator('.card[aria-label="No longer taking"]')).toContainText('Iron');

  await page.getByLabel('Edit Iron').click();
  await page.getByRole('button', { name: 'Erase' }).click();
  await expect(page.getByRole('dialog', { name: 'Erase medicine?' })).toBeVisible();
  await page.getByRole('dialog', { name: 'Erase medicine?' }).getByRole('button', { name: 'Erase' }).click();
  await expect(page.getByText('Iron')).toHaveCount(0);
});

test('Stock: banner, eat today, search add, draft editor', async ({ page }) => {
  await open(page);
  await tab(page, 'stock').click();
  await expect(page.locator('.card[aria-label="Cooked"]')).toContainText('eat today');
  const banner = page.locator('#soon-banner');
  await expect(banner).toHaveText('1 item to use within 7 days');
  await banner.click();
  await expect(page.locator('.card[aria-label="Shelf"]')).toHaveCount(0);
  await banner.click();
  await expect(page.locator('.card[aria-label="Shelf"]')).toBeVisible();

  await page.locator('#stock-search').fill('Tofu');
  await page.locator('#add-from-search').click();
  await expect(page.locator('#st-name')).toHaveValue('Tofu');
  await page.keyboard.press('Escape');
  await expect(page.locator('.sheet')).toHaveCount(0);
  expect(await page.evaluate(() => Object.keys(window.__mock.stock).length)).toBe(3); // closing a new item leaves no row

  await page.locator('#add-from-search').click();
  await page.locator('#st-unit').fill('g');
  await page.locator('#st-qty').fill('200');
  await page.getByLabel('More').click();
  await expect(page.locator('#st-qty')).toHaveValue('250');
  await page.locator('#st-unit').fill('packs');
  await page.getByLabel('Less').click();
  await expect(page.locator('#st-qty')).toHaveValue('249');
  await page.getByRole('button', { name: 'Fridge' }).click();
  await page.getByRole('button', { name: 'Save' }).click();
  await page.locator('#stock-search').fill('');
  await expect(page.locator('.card[aria-label="Fridge"]')).toContainText('Tofu');
});

test('Week: plan, repeat, history opens a day', async ({ page }) => {
  await open(page);
  await tab(page, 'week').click();
  await expect(page.getByTestId('plan-avg')).toHaveText('avg 520 kcal · 38 g');
  await page.getByRole('button', { name: 'Repeat the last planned week' }).click();
  await expect(page.locator('.toast').last()).toHaveText('Planned 1 more day');
  await page.locator('[data-plan-day] button[aria-expanded]').first().click();
  await page.getByLabel('Planned meal name').fill('Oats');
  await page.getByLabel('Planned kcal').fill('400');
  await page.getByRole('button', { name: 'Add to plan' }).click();
  await expect(page.locator('[data-plan-day]').first()).toContainText('Oats');

  await page.getByRole('button', { name: 'What happened' }).click();
  await expect(page.locator('.card[aria-label="Summary"]')).toContainText('Avg kcal');
  await page.locator('[data-hist]').nth(1).click();
  await expect(page.locator('#title')).toHaveText('Today');
  await expect(page.getByTestId('kcal')).toHaveText('374');
});

test('Shop: budget, check what you have, move back, bought it', async ({ page }) => {
  await open(page);
  await tab(page, 'shop').click();
  await expect(page.getByTestId('spend')).toHaveText('CHF 8.60 / 80');
  await page.locator('#shop-name').fill('Tofu');
  await page.locator('#shop-qty').fill('2 packs');
  await page.locator('#shop-price').fill('5');
  await page.getByRole('button', { name: 'Add to list' }).click();
  await expect(page.getByTestId('spend')).toHaveText('CHF 13.60 / 80');

  await page.getByRole('button', { name: 'Check what you already have' }).click();
  await page.getByRole('dialog').getByRole('button', { name: 'Still there' }).click();
  const inStock = page.locator('.card[aria-label="Already in stock"]');
  await expect(inStock).toContainText('Skyr');
  await inStock.getByRole('button', { name: 'Move Skyr back to the list' }).click();
  await expect(inStock).toBeHidden();

  await page.getByLabel('Got Tofu').check();
  await page.getByRole('button', { name: 'Bought it — add to stock' }).click();
  await expect(page.locator('.toast').last()).toHaveText('Added 1 item to stock');
  await tab(page, 'stock').click();
  await expect(page.locator('#screen-stock .card[aria-label="Fridge"]')).toContainText('Tofu');
  await expect(page.locator('#screen-stock .card[aria-label="Fridge"]')).toContainText('2 packs');
});

test('Settings page: autosaved targets, model dropdowns, key, AI test, learned foods, backup, updates', async ({ page }) => {
  await open(page);
  await page.locator('#open-settings').click();
  await expect(page.locator('#title')).toHaveText('Settings');
  const targets = page.locator('#targets-status');
  await page.locator('#set-weight').fill('20');
  await page.locator('#set-weight').press('Tab');
  await expect(targets).toHaveText('Weight must be between 30 and 300 kg.');
  await page.locator('#set-weight').fill('80');
  await page.locator('#set-weight').press('Tab');
  await expect(targets).toHaveText('Saved');
  expect(await page.evaluate(() => window.__mock.settings.weightKg)).toBe(80);

  const st = page.getByTestId('ai-status');
  await page.locator('#ai-cmodel').selectOption('opus');
  await expect(st).toHaveText('Saved');
  expect(await page.evaluate(() => window.__mock.ai.claudeModel)).toBe('opus');
  await page.locator('#ai-gmodel').selectOption('__custom');
  await expect(page.locator('#ai-gmodel-custom')).toBeVisible();
  await page.locator('#ai-gmodel-custom').fill('bad model!');
  await page.locator('#ai-gmodel-custom').press('Tab');
  await expect(st).toContainText('Model names may only use');
  await page.locator('#ai-gmodel-custom').fill('gemini-2.5-flash');
  await page.locator('#ai-gmodel-custom').press('Tab');
  await expect(st).toHaveText('Saved');
  expect(await page.evaluate(() => window.__mock.ai.geminiModel)).toBe('gemini-2.5-flash');
  await page.locator('#ai-order').selectOption('gemini,claude');
  await expect.poll(() => page.evaluate(() => window.__mock.ai.order.join(','))).toBe('gemini,claude');

  await expect(page.locator('#ai-key')).toHaveAttribute('type', 'password');
  await page.locator('#ai-key').fill('test-key');
  await page.getByRole('button', { name: 'Save key' }).click();
  await expect(page.getByText('Key saved')).toBeVisible();
  expect(await page.evaluate(() => window.__mock.ai.geminiKey)).toBe('test-key');

  await page.getByRole('button', { name: 'Test the AI' }).click();
  await expect(st).toHaveText('Claude answered. AI is working.');

  await page.getByLabel('Forget Protein pudding').click();
  await expect(page.locator('#learned')).toContainText('Foods the AI works out');

  await page.getByRole('button', { name: 'Back up…' }).click();
  await expect(page.locator('.toast').last()).toHaveText('Backup saved');
  await page.getByRole('button', { name: 'Restore / import…' }).click();
  await expect(page.locator('.toast').last()).toHaveText('Imported 3 records');

  await expect(page.locator('.set-section[aria-label="Updates"]')).toContainText('Version 0.4.0');
  await page.getByRole('button', { name: 'Check for updates' }).click();
  await expect(page.locator('#update-status')).toHaveText('You’re up to date.');
});

test('Rules chat shows a diff and saves only on Apply', async ({ page }) => {
  await open(page);
  await page.getByLabel('House rules chat').click();
  await expect(page.getByTestId('rules-text')).toHaveText('No cheese after 20:00');
  await page.locator('#rules-input').fill('Max 2 eggs a day');
  await page.getByRole('button', { name: 'Send' }).click();
  await expect(page.locator('.diff .add')).toHaveText('+ Max 2 eggs a day');
  expect(await page.evaluate(() => window.__mock.rules)).toBe('No cheese after 20:00');
  await page.getByRole('button', { name: 'Apply' }).click();
  await expect(page.getByTestId('rules-text')).toHaveText('No cheese after 20:00\nMax 2 eggs a day');
});

test('Sheets close on Escape and on the backdrop', async ({ page }) => {
  await open(page);
  await page.getByRole('button', { name: 'Add training' }).click();
  await expect(page.locator('.sheet')).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.locator('.sheet')).toHaveCount(0);
  await page.getByLabel('House rules chat').click();
  await page.mouse.click(10, 10);
  await expect(page.locator('.sheet')).toHaveCount(0);
});

test('Failed saves show a short message', async ({ page }) => {
  await open(page);
  await tab(page, 'shop').click();
  await page.getByRole('button', { name: 'Add to list' }).click();
  await expect(page.locator('#screen-shop .card .error').first()).toHaveText('Name the item.');
  await page.evaluate(() => (window.__mock.aiAvailable = false));
  await page.locator('#open-settings').click();
  await page.getByRole('button', { name: 'Test the AI' }).click();
  await expect(page.getByTestId('ai-status')).toContainText('No AI is set up');
});

test('All buttons have accessible names', async ({ page }) => {
  await open(page);
  for (const t of ['today', 'add', 'meds', 'stock', 'week', 'shop', 'settings']) {
    if (t === 'settings') await page.locator('#open-settings').click();
    else await tab(page, t).click();
    const unnamed = await page.evaluate(() =>
      [...document.querySelectorAll('button')].filter((b) => b.offsetParent !== null && !(b.getAttribute('aria-label') || b.textContent.trim())).length,
    );
    expect(unnamed, `unnamed buttons on ${t}`).toBe(0);
  }
});

test('360 px wide: no sideways scrolling on any screen', async ({ page }) => {
  await page.setViewportSize({ width: 360, height: 780 });
  await open(page);
  for (const t of ['today', 'add', 'meds', 'stock', 'week', 'shop']) {
    await tab(page, t).click();
    const w = await page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth, document.getElementById('main').scrollWidth));
    expect(w, `scroll width on ${t}`).toBeLessThanOrEqual(360);
  }
  await page.locator('#open-settings').click();
  const sw = await page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.getElementById('main').scrollWidth));
  expect(sw, 'scroll width on settings').toBeLessThanOrEqual(360);
});

test.describe('dark mode', () => {
  test.use({ colorScheme: 'dark' });
  test('follows the system', async ({ page }) => {
    await open(page);
    const bg = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);
    expect(bg).toBe('rgb(26, 27, 38)');
    const accent = await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue('--accent').trim());
    expect(accent.toUpperCase()).toBe('#BB9AF7');
  });
});

test('light mode colours', async ({ page }) => {
  await page.emulateMedia({ colorScheme: 'light' });
  await open(page);
  expect(await page.evaluate(() => getComputedStyle(document.body).backgroundColor)).toBe('rgb(225, 226, 231)');
});

test('3000 log rows: every tab renders in under 400 ms', async ({ page }) => {
  await open(page, 'seed=demo&rows=3000');
  // Warm the engine once (fonts, first layout), then time a full data refresh + render of each tab.
  for (const t of ['add', 'meds', 'stock', 'week', 'shop', 'today']) await tab(page, t).click();
  await page.evaluate(() => document.fonts.ready);
  for (const t of ['add', 'meds', 'stock', 'week', 'shop', 'today']) {
    const ms = await page.evaluate((name) => window.__showTimed(name), t);
    expect(ms, `render time for ${t}`).toBeLessThan(400);
  }
  await page.evaluate(() => window.__showTimed('week'));
  const ms = await page.evaluate(async () => {
    const t0 = performance.now();
    document.querySelector('[data-mode="history"]').click();
    await new Promise((r) => setTimeout(r, 0));
    await window.__bgRefresh();
    void document.body.offsetHeight;
    return performance.now() - t0;
  });
  expect(ms, 'render time for history').toBeLessThan(400);
  await expect(page.locator('[data-hist]').first()).toBeVisible();
});

test('Desktop layout: sidebar, columns, shortcuts, no sideways scroll', async ({ page }) => {
  await page.setViewportSize({ width: 1200, height: 800 });
  await open(page);
  const nav = await page.locator('nav.tabs').boundingBox();
  expect(nav.height).toBeGreaterThan(500); // vertical sidebar, not a bottom bar
  expect(nav.x).toBe(0);
  const cols = page.locator('#screen-today .cols > .col');
  const [a, b] = [await cols.nth(0).boundingBox(), await cols.nth(1).boundingBox()];
  expect(b.x).toBeGreaterThan(a.x + a.width - 1); // side by side
  for (const t of ['today', 'add', 'meds', 'stock', 'week', 'shop']) {
    await tab(page, t).click();
    const w = await page.evaluate(() => document.documentElement.scrollWidth);
    expect(w, `scroll width on ${t}`).toBeLessThanOrEqual(1200);
  }
  await page.keyboard.press('ControlOrMeta+1');
  await expect(page.locator('#title')).toHaveText('Today');
  await page.keyboard.press('ArrowLeft');
  await expect(page.getByRole('button', { name: 'Back to today' })).toBeVisible();
  await page.keyboard.press('ControlOrMeta+n');
  await expect(page.locator('#describe')).toBeFocused();
  await page.keyboard.press('ControlOrMeta+,');
  await expect(page.locator('#title')).toHaveText('Settings');
  await page.keyboard.press('ControlOrMeta+k');
  const dlg = await page.locator('.sheet').boundingBox();
  expect(dlg.y).toBeGreaterThan(10); // centered dialog, not a bottom sheet
  expect(dlg.y + dlg.height).toBeLessThan(800 - 10);
});

test('New layout: plan board, edit sheets, charts and insights', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await open(page, 'seed=rich&layout=new&hhmm=13:10');
  await expect(page.locator('#title')).toHaveText('Log');
  await expect(page.getByTestId('fiber')).toBeVisible();
  await expect(page.locator('.micro-row')).toHaveCount(0); // vitamins live in History
  // Plan: upcoming days only, tap a meal to edit
  await tab(page, 'plan').click();
  const today = page.locator('.plan-day').first();
  await expect(today).toContainText('Today');
  await today.getByLabel('Edit Tofu bowl').click();
  await page.locator('#pm-kcal').fill('600');
  await page.getByRole('dialog').getByRole('button', { name: 'Save' }).click();
  await expect(today).toContainText('600');
  await today.getByLabel('Edit Tofu bowl').click();
  await page.getByRole('button', { name: 'Remove from plan' }).click();
  await expect(today).not.toContainText('Tofu bowl');
  await today.getByLabel(/Add snack on Today/).click();
  await page.locator('#pm-name').fill('Banana');
  await expect(page.locator('#pm-kcal')).toHaveValue('107');
  await page.getByRole('dialog').getByRole('button', { name: 'Add to plan' }).click();
  await expect(today).toContainText('Banana');
  // Training: edit a session
  await tab(page, 'training').click();
  await page.locator('#screen-training [aria-label^="Edit "]').first().click();
  await page.locator('#tr-mins').fill('90');
  await page.getByRole('button', { name: 'Save training' }).click();
  await expect(page.locator('.card[aria-label="Training log"]')).toContainText('90 min');
  // History: insights and charts
  await tab(page, 'history').click();
  await expect(page.locator('.insights .insight').first()).toContainText('Needs attention');
  await expect(page.locator('#screen-history .chart svg')).toHaveCount(5); // calories, protein, fiber, training, sleep
  await expect(page.locator('.card[aria-label="Nutrients"] .pct-row').first()).toBeVisible();
});

test('Food database: search, pick a portion, log it; Swiss download; nutrients in History', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await open(page, 'seed=demo&layout=new&hhmm=12:00');
  await expect(page.locator('#food-search')).toBeHidden(); // adding food lives in an overlay
  await page.locator('#open-add').click();
  await expect(page.locator('#food-search')).toBeFocused();
  await page.locator('#food-search').fill('lentils');
  const first = page.locator('.food-hit').first();
  await expect(first).toContainText('Lentils');
  await expect(first).toContainText('USDA');
  await first.click();
  await expect(page.locator('#food-search-grams')).toHaveValue('100');
  await page.locator('.food-picked .chip', { hasText: 'cup' }).first().click();
  await page.getByRole('button', { name: 'Lunch', exact: true }).click();
  await page.getByRole('button', { name: 'Add to log' }).click();
  await expect(page.locator('.toast').last()).toContainText('Logged Lentils');
  await expect(page.locator('#food-search')).toBeHidden(); // the overlay closes after logging
  await expect(page.locator('[data-meal="Lunch"]')).toContainText('Lentils');
  // Built-in foods come first for words they know (skyr isn't in USDA).
  await page.keyboard.press('ControlOrMeta+n');
  await page.locator('#food-search').fill('skyr');
  await expect(page.locator('.food-hit').first()).toContainText('Built-in');
  await page.keyboard.press('Escape');
  // Plan editor: search fills the meal
  await tab(page, 'plan').click();
  await page.locator('.plan-day').first().getByLabel(/Add dinner on Today/).click();
  await page.locator('#plan-food').fill('chickpeas');
  await page.locator('.sheet:visible .food-hit').first().click();
  await page.locator('#plan-food-grams').fill('200');
  await page.getByRole('button', { name: 'Use this' }).click();
  await expect(page.locator('#pm-name')).toHaveValue(/Chickpeas.*200 g/);
  await expect(page.locator('#pm-kcal')).not.toHaveValue('');
  await page.getByRole('button', { name: 'Add to plan' }).click();
  await expect(page.locator('.plan-day').first()).toContainText('Chickpeas');
  // Settings: download the Swiss database, then search it
  await page.locator('#open-settings').click();
  await expect(page.locator('[data-source="usda"]')).toContainText('8,237 foods');
  await page.locator('#get-swiss').click();
  await expect(page.locator('#foods-status')).toHaveText('Added 1,216 foods.');
  await expect(page.locator('[data-source="swiss"]')).toBeVisible();
  await tab(page, 'today').click();
  await page.locator('#open-add').click();
  await page.locator('#food-search').fill('rosti');
  await expect(page.locator('.food-hit').first()).toContainText('Swiss');
  // History lists every nutrient it knows about, grouped
  await page.keyboard.press('Escape');
  await tab(page, 'history').click();
  const nut = page.locator('.card[aria-label="Nutrients"]');
  await expect(nut.locator('.nut-group h3', { hasText: 'Minerals' })).toBeVisible();
  await expect(nut.locator('.pct-row', { hasText: 'Folate' })).toBeVisible();
});

test('Recipes: build from ingredients, then log a serving from search', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await open(page, 'seed=demo&layout=new&hhmm=19:00');
  await tab(page, 'kitchen').click();
  await page.locator('[data-part="Recipes"]').click();
  await page.getByRole('button', { name: 'New recipe' }).click();
  await page.locator('#rc-name').fill('Masoor dal');
  await page.locator('#rc-servings').fill('4');
  await page.locator('#rc-cooked').fill('1200');
  for (const [q, g] of [['lentils', '400'], ['onions', '150']]) {
    await page.locator('#rc-food').fill(q);
    await page.locator('.sheet:visible .food-hit').first().click();
    await page.locator('#rc-food-grams').fill(g);
    await page.getByRole('button', { name: 'Add ingredient' }).click();
  }
  await expect(page.locator('#rc-ingredients .item')).toHaveCount(2);
  await expect(page.getByTestId('recipe-totals')).toContainText('per serving'.replace('per', 'Per'));
  await expect(page.getByTestId('recipe-totals')).toContainText('(300 g)');
  await page.getByRole('button', { name: 'Save recipe' }).click();
  await expect(page.locator('.recipe-card')).toContainText('Masoor dal');
  // Log one serving from the Log's search
  await tab(page, 'today').click();
  await page.locator('#open-add').click();
  await page.locator('#food-search').fill('masoor');
  const hit = page.locator('.food-hit').first();
  await expect(hit).toContainText('Recipe');
  await hit.click();
  await page.locator('.food-picked .chip', { hasText: '1 serving' }).click();
  await expect(page.locator('#food-search-grams')).toHaveValue('300');
  await page.locator('.food-picked [aria-label="Meal"] [data-v="Dinner"]').click();
  await page.getByRole('button', { name: 'Add to log' }).click();
  await expect(page.locator('[data-meal="Dinner"]')).toContainText('Masoor dal');
});

test('Open Food Facts: look up packaged products online, then log one', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await open(page, 'seed=demo&layout=new&hhmm=08:00');
  await page.locator('#open-add').click();
  await page.locator('#food-search').fill('skyr');
  await page.locator('#food-search-online').click();
  const hit = page.locator('.food-hit', { hasText: 'Skyr vanilla' });
  await expect(hit).toContainText('Product');
  await hit.click();
  await page.locator('.food-picked .chip', { hasText: '1 serving' }).click();
  await page.locator('.food-picked').getByRole('button', { name: 'Breakfast', exact: true }).click();
  await page.getByRole('button', { name: 'Add to log' }).click();
  await expect(page.locator('[data-meal="Breakfast"]')).toContainText('Skyr vanilla');
  // Offline afterwards: it's now in local search, and listed with its licence in Settings.
  await page.evaluate(() => (window.__mock.online = false));
  await page.locator('#open-add').click();
  await page.locator('#food-search').fill('skyr van');
  await expect(page.locator('.food-hit').first()).toContainText('Skyr vanilla');
  await page.keyboard.press('Escape');
  await page.locator('#open-settings').click();
  await expect(page.locator('[data-source="off"]')).toContainText('Open Food Facts');
});

test('Log date bar: scrolls to the selected day, Today pinned and outlined', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await open(page, 'seed=demo&layout=new&hhmm=12:00');
  const todayBtn = page.locator('.strip-day.is-today');
  await expect(todayBtn).toHaveAttribute('aria-pressed', 'true');
  // Stepping back to days already in view doesn't move the bar.
  const track = page.locator('.strip-track');
  const before = await track.evaluate((t) => t.scrollLeft);
  for (let i = 0; i < 3; i++) await page.getByLabel('Previous day').click();
  await expect(page.locator('.strip-track [aria-pressed="true"]')).toHaveAttribute('data-date', '2026-11-19');
  expect(await track.evaluate((t) => t.scrollLeft)).toBe(before);
  for (let i = 0; i < 42; i++) await page.getByLabel('Previous day').click();
  const sel = page.locator('.strip-track [aria-pressed="true"]');
  await expect(sel).toHaveAttribute('data-date', '2026-10-08');
  await expect(sel).toBeInViewport();
  await expect(todayBtn).toBeInViewport(); // never scrolled away
  await expect(todayBtn).toHaveAttribute('aria-pressed', 'false');
  await expect(page.locator('#date-chip')).toContainText('not today');
  await expect(page.locator('.strip-track')).toHaveClass(/more-right/); // days hidden before Today

  await todayBtn.click();
  await expect(todayBtn).toHaveAttribute('aria-pressed', 'true');
  await expect(page.getByLabel('Next day')).toBeDisabled();
  await expect(page.locator('.strip-track')).not.toHaveClass(/more-right/); // nothing hidden next to Today
  // One combined bar: no separate date card.
  await expect(page.locator('#day-label')).toHaveCount(0);
});

test('A dish with toppings stays one item; find it as a packaged product', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await open(page, 'seed=demo&layout=new&hhmm=19:00');
  await page.evaluate(() => (window.__mock.aiAvailable = false));
  await page.locator('#open-add').click();
  await page.locator('#describe').fill('1 daemilio pizza with mushroom and ham');
  await page.getByRole('button', { name: 'Work out the calories' }).click();
  await expect(page.getByTestId('unknown')).toContainText('1 daemilio pizza with mushroom and ham');
  await expect(page.locator('[data-draft]')).toHaveCount(0); // not three separate guesses
  await page.evaluate(() => (window.__mock.aiAvailable = true));
  await page.getByRole('button', { name: 'Work out the calories' }).click();
  await expect(page.locator('[data-draft]')).toHaveCount(1);
  // No database food has that name: an AI estimate you can swap for a packaged product.
  await expect(page.locator('[data-draft="0"]')).toContainText('AI estimate');
  await page.getByLabel(/Change food: 1 daemilio pizza with mushroom and ham/).click();
  await expect(page.locator('#draft-food-q')).toHaveValue('1 daemilio pizza with mushroom and ham');
  await expect(page.getByRole('button', { name: 'Look it up as a packaged product' })).toBeVisible();
});

test('Add food opens near the top, stays put as results come, and fits the window', async ({ page }) => {
  for (const [w, hgt] of [[1280, 900], [1280, 640], [400, 800]]) {
    await page.setViewportSize({ width: w, height: hgt });
    await open(page, 'seed=demo&layout=new&hhmm=12:00');
    await page.locator('#open-add').click();
    const before = await page.locator('.overlay-sheet').boundingBox();
    expect(before.y, `not stuck to the top edge at ${w}×${hgt}`).toBeGreaterThan(8);
    expect(before.y, `near the top at ${w}×${hgt}`).toBeLessThan(hgt * 0.12);
    await page.locator('#food-search').fill('lentils');
    await expect(page.locator('.food-hit').first()).toBeVisible();
    const after = await page.locator('.overlay-sheet').boundingBox();
    expect(after.y, `doesn't jump at ${w}×${hgt}`).toBe(before.y);
    expect(after.y + after.height, `fits the window at ${w}×${hgt}`).toBeLessThanOrEqual(hgt);
  }
});

test('Usual-food chips go into the draft next to what you described', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await open(page, 'seed=rich&layout=new&hhmm=08:00');
  await page.locator('#open-add').click();
  await page.locator('#describe').fill('1 banana');
  await page.getByRole('button', { name: 'Work out the calories' }).click();
  await expect(page.locator('#draft [data-draft]')).toHaveCount(1);
  const chip = page.locator('.overlay-sheet .chips .chip').first();
  const name = (await chip.getAttribute('aria-label')).replace(/^Add | to the draft$/g, '').split(' · ')[0];
  await chip.click();
  await expect(page.locator('#draft [data-draft]')).toHaveCount(2);
  await expect(page.locator('#draft')).toContainText(name);
  await expect(page.locator('.overlay-sheet')).toBeVisible(); // nothing logged yet
  await page.locator('#draft').getByRole('button', { name: 'Save' }).click();
  await expect(page.locator('.toast').last()).toHaveText('Saved 2 items');
  await expect(page.locator('.overlay-sheet')).toBeHidden();
});

test('Log always shows the four meals, a compact summary and a floating Add button', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await open(page, 'layout=new&hhmm=08:00');
  const meals = page.locator('#screen-today .food-group');
  await expect(meals).toHaveCount(4);
  await expect(meals.locator('.food-head .fn')).toHaveText(['Breakfast', 'Lunch', 'Snack', 'Dinner']);
  await expect(page.locator('[data-meal="Snack"]')).toContainText('Nothing yet');
  // Summary: three single-line rows, well under the old height
  const summary = await page.locator('#screen-today [aria-label="Energy and protein"]').boundingBox();
  expect(summary.height).toBeLessThan(170);
  await expect(page.getByRole('progressbar', { name: 'Fiber against floor' })).toBeVisible();
  const slashes = await page.locator('#screen-today .macro .mval .muted').evaluateAll((es) => es.map((e) => Math.round(e.getBoundingClientRect().left)));
  expect(new Set(slashes).size, 'the / lines up in all three rows').toBe(1);
  // The button floats: it stays in the same place on screen when the page scrolls
  const fab = page.getByRole('button', { name: 'Add food', exact: true });
  const before = await fab.boundingBox();
  expect(before.x + before.width).toBeGreaterThan(1280 - 80);
  await page.locator('main').evaluate((m) => m.scrollTo(0, 400));
  expect((await fab.boundingBox()).y).toBe(before.y);
  // Meals have no + of their own; food goes in through the floating button
  await expect(page.locator('.food-head button')).toHaveCount(0);
  await fab.click();
  await page.locator('#food-search').fill('lentils');
  await page.locator('.overlay-sheet .food-hit').first().click();
  await page.locator('.food-picked [aria-label="Meal"] [data-v="Snack"]').click();
  await page.getByRole('button', { name: 'Add to log' }).click();
  await expect(page.locator('[data-meal="Snack"]')).toContainText('Lentils');
  await expect(page.locator('[data-meal="Snack"]')).not.toContainText('Nothing yet');
});

test('Floating Add button sits above the phone tabs', async ({ page }) => {
  await page.setViewportSize({ width: 360, height: 740 });
  await open(page, 'layout=new');
  const fab = await page.getByRole('button', { name: 'Add food', exact: true }).boundingBox();
  const tabs = await page.locator('nav.tabs').boundingBox();
  expect(fab.y + fab.height).toBeLessThanOrEqual(tabs.y);
  await expect(page.locator('#screen-today .food-group')).toHaveCount(4);
  await tab(page, 'history').click();
  await expect(page.getByRole('button', { name: 'Add food', exact: true })).toBeHidden();
});

test('Settings targets: each box sits on the same line as its name, hint underneath', async ({ page }) => {
  for (const w of [1280, 360]) {
    await page.setViewportSize({ width: w, height: 800 });
    await open(page, 'seed=demo&layout=new');
    await page.locator('#open-settings').click();
    const rows = page.locator('.target-row');
    await expect(rows).toHaveCount(4);
    for (const r of await rows.all()) {
      const n = await r.locator('.set-name').boundingBox();
      const i = await r.locator('input').boundingBox();
      expect(Math.abs(n.y + n.height / 2 - (i.y + i.height / 2)), `same line at ${w}px`).toBeLessThan(3);
      expect((await r.locator('.target-hint').boundingBox()).y, 'hint underneath').toBeGreaterThan(i.y + i.height / 2);
      expect((await r.boundingBox()).height).toBeLessThan(72);
    }
  }
});

test('The settings gear is hidden on the Settings page', async ({ page }) => {
  await open(page, 'seed=demo&layout=new');
  await expect(page.locator('#open-settings')).toBeVisible();
  await page.locator('#open-settings').click();
  await expect(page.locator('#open-settings')).toBeHidden();
  await tab(page, 'today').click();
  await expect(page.locator('#open-settings')).toBeVisible();
});

// ---------- Bugs from the UI review ----------

test('A double click saves once: add food, plan meal, training', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await open(page, 'seed=demo&layout=new&hhmm=12:00&delay=200');
  const calls = (cmd) => page.evaluate((c) => window.__mock.calls.filter((x) => x === c).length, cmd);
  await page.locator('#open-add').click();
  await page.locator('#food-search').fill('lentils');
  await page.locator('.food-hit').first().click();
  await page.locator('.food-picked').getByRole('button', { name: 'Add to log' }).dblclick();
  await expect(page.locator('.food-picked')).toBeHidden();
  expect(await calls('add_food')).toBe(1);
  // Adding closes the panel and redraws the Log (slowly here): wait for it, so the next
  // click doesn't land on a button that's about to be replaced.
  await expect(page.locator('.overlay-backdrop')).toBeHidden();
  await expect(page.locator('[aria-label="Food logged"]')).toContainText(/lentil/i);
  await expect(page.locator('[aria-busy="true"]')).toHaveCount(0);

  await page.getByRole('button', { name: 'Add training' }).click();
  await expect(page.locator('#tr-mins')).toBeVisible();
  await page.locator('#tr-mins').fill('30');
  await page.getByRole('button', { name: 'Save training' }).dblclick();
  await expect(page.locator('.backdrop:not(.hidden) .sheet')).toHaveCount(0);
  expect(await calls('add_workout')).toBe(1);
});

test('Double-clicking "Remove from plan" removes only that meal', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await open(page, 'seed=rich&layout=new&hhmm=13:10&delay=150');
  await tab(page, 'plan').click();
  const today = page.locator('.plan-day').first();
  await expect(today.getByLabel('Edit Skyr, oats, raspberries')).toBeVisible();
  const before = await today.locator('[aria-label^="Edit "]').count();
  await today.getByLabel('Edit Skyr, oats, raspberries').click();
  await page.getByRole('button', { name: 'Remove from plan' }).dblclick();
  await expect(today).not.toContainText('Skyr, oats, raspberries');
  await expect(today.locator('[aria-label^="Edit "]')).toHaveCount(before - 1);
  await expect(today).toContainText('Tofu bowl');
});

test('Clicking days quickly shows the last one clicked; two Previous clicks go back two days', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await open(page, 'seed=demo&layout=new&hhmm=12:00');
  // The first day's answer is slow, the second's fast: the slow one must not win.
  await page.evaluate(() => { window.__mockDelay = (cmd, a) => (cmd === 'get_day' && a.date === '2026-11-15' ? 400 : 20); });
  await page.locator('.strip-day[data-date="2026-11-15"]').click();
  await page.locator('.strip-day[data-date="2026-11-20"]').click();
  await page.waitForTimeout(600);
  await expect(page.locator('.strip-day[aria-pressed="true"]')).toHaveAttribute('data-date', '2026-11-20');
  await expect(page.locator('#date-chip')).toContainText('20');
  await page.evaluate(() => { window.__mockDelay = () => 150; });
  await page.getByLabel('Previous day').click();
  await page.getByLabel('Previous day').click();
  await expect(page.locator('.strip-day[aria-pressed="true"]')).toHaveAttribute('data-date', '2026-11-18');
});

test('Editing grams updates the amount in the name; 0 kcal foods can be edited', async ({ page }) => {
  await open(page);
  await page.getByLabel('Previous day').click();
  await page.getByLabel('Edit Skyr · 200 g').click();
  await page.locator('#ed-grams').fill('100');
  await expect(page.locator('#ed-name')).toHaveValue('Skyr · 100 g');
  await page.locator('#ed-kcal').fill('0');
  await page.getByRole('dialog').getByRole('button', { name: 'Save' }).click();
  await expect(page.locator('.sheet')).toHaveCount(0);
  await page.getByLabel('Edit Skyr · 100 g').click();
  await expect(page.locator('#ed-kcal')).toHaveValue('0');
  await page.getByRole('dialog').getByRole('button', { name: 'Save' }).click();
  await expect(page.locator('.sheet')).toHaveCount(0);
});

test('Settings: an empty protein or fiber floor is refused, not saved as 0', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await open(page, 'seed=demo&layout=new');
  await page.locator('#open-settings').click();
  const before = await page.evaluate(() => window.__mock.settings.protFloor);
  await page.locator('#set-prot').fill('');
  await page.locator('#set-prot').blur();
  await expect(page.locator('#targets-status')).toContainText('Protein floor must be between');
  await expect(page.locator('#set-prot')).toHaveAttribute('aria-invalid', 'true');
  expect(await page.evaluate(() => window.__mock.settings.protFloor)).toBe(before);
  await page.locator('#set-fiber').fill('');
  await page.locator('#set-fiber').blur();
  expect(await page.evaluate(() => window.__mock.settings.fiberFloor)).not.toBe(0);
});

test('Install and restart installs without checking again', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await open(page, 'seed=demo&layout=new&update=0.5.0');
  await page.locator('#open-settings').click();
  const checks = () => page.evaluate(() => window.__mock.calls.filter((x) => x === 'plugin:updater|check').length);
  await page.locator('#check-updates').click();
  await expect(page.locator('#check-updates')).toHaveText('Install and restart');
  const n = await checks();
  await page.locator('#check-updates').click();
  await expect(page.locator('#update-status')).toContainText('Update failed');
  expect(await checks()).toBe(n);
  expect(await page.evaluate(() => window.__mock.calls.filter((x) => x === 'plugin:updater|download_and_install').length)).toBe(1);
});

test('Food results never overlap, and the online button sits below them', async ({ page }) => {
  for (const [w, hgt] of [[1280, 900], [360, 740]]) {
    await page.setViewportSize({ width: w, height: hgt });
    await open(page, 'seed=demo&layout=new&hhmm=12:00');
    await page.locator('#open-add').click();
    await page.locator('#food-search').fill('a');
    await page.locator('#food-search').fill('ap');
    await expect(page.locator('.food-hit').first()).toBeVisible();
    const boxes = await page.locator('.food-results > *').evaluateAll((els) => els.map((e) => [e.offsetTop, e.offsetHeight]));
    expect(boxes.length).toBeGreaterThan(2);
    for (let i = 1; i < boxes.length; i++) {
      expect(boxes[i][0], `row ${i} starts below row ${i - 1} at ${w}px`).toBeGreaterThanOrEqual(boxes[i - 1][0] + boxes[i - 1][1] - 1);
    }
  }
});

test('Sheets never show a stray "null"', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await open(page, 'seed=rich&layout=new&hhmm=13:10');
  const sheet = page.locator('.backdrop:not(.hidden) .sheet');
  await page.locator('[aria-label^="Edit "].food-row').first().click();
  await expect(sheet).toBeVisible();
  await expect(sheet).not.toContainText('null');
  await page.keyboard.press('Escape');
  await tab(page, 'plan').click();
  await page.locator('.plan-day').first().getByLabel('Edit Tofu bowl').click();
  await expect(sheet).toBeVisible();
  await expect(sheet).not.toContainText('null');
  await page.keyboard.press('Escape');
  await tab(page, 'meds').click();
  await page.getByRole('button', { name: 'Add medicine' }).click();
  await expect(sheet).toBeVisible();
  await expect(sheet).not.toContainText('null');
});

test('Meds: a new medicine shows in Today straight away', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await open(page, 'seed=demo&layout=new&hhmm=07:00');
  await tab(page, 'meds').click();
  await page.getByRole('button', { name: 'Add medicine' }).click();
  await page.locator('#med-name').fill('Iron');
  await page.locator('#med-dose').fill('1 tablet');
  await page.getByRole('button', { name: 'Save' }).click();
  await expect(page.getByLabel('Took Iron')).toBeVisible();
  await page.getByLabel('Edit Iron').click();
  await page.getByRole('button', { name: 'Stop taking' }).click();
  await expect(page.getByLabel('Took Iron')).toHaveCount(0);
});

test('History "Day by day" shows that it opens', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await open(page, 'seed=rich&layout=new');
  await tab(page, 'history').click();
  const summary = page.locator('details > summary', { hasText: 'Day by day' });
  const w = await summary.evaluate((el) => parseFloat(getComputedStyle(el, '::before').width));
  expect(w).toBeGreaterThan(4);
});

test('Add training from the Training tab keeps the Log on the day you were looking at', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await open(page, 'seed=demo&layout=new&hhmm=12:00');
  await page.getByLabel('Previous day').click();
  await expect(page.locator('.strip-day[aria-pressed="true"]')).toHaveAttribute('data-date', '2026-11-21');
  await tab(page, 'training').click();
  await page.getByRole('button', { name: 'Add training' }).click();
  await page.locator('#tr-mins').fill('20');
  await page.getByRole('button', { name: 'Save training' }).click();
  await expect(page.locator('.card[aria-label="Training log"]')).toContainText('Today');
  await tab(page, 'today').click();
  await expect(page.locator('.strip-day[aria-pressed="true"]')).toHaveAttribute('data-date', '2026-11-21');
});

test('Keyboard focus survives redraws and stays inside open sheets', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await open(page, 'seed=demo&layout=new&hhmm=12:00');
  await page.locator('.strip-day[data-date="2026-11-19"]').focus();
  await page.keyboard.press('Enter');
  await expect(page.locator('.strip-day[aria-pressed="true"]')).toHaveAttribute('data-date', '2026-11-19');
  expect(await page.evaluate(() => document.activeElement.dataset.date)).toBe('2026-11-19');

  await page.getByRole('button', { name: 'Today', exact: true }).click();
  await page.getByLabel('Ate Tofu bowl').focus();
  await page.keyboard.press('Space');
  await expect(page.getByTestId('kcal')).toHaveText('520');
  expect(await page.evaluate(() => document.activeElement.getAttribute('aria-label'))).toBe('Ate Tofu bowl');

  await page.getByRole('button', { name: 'Add training' }).click();
  for (let i = 0; i < 25; i++) {
    await page.keyboard.press('Tab');
    expect(await page.evaluate(() => !!document.activeElement.closest('.sheet'))).toBe(true);
  }
  await page.keyboard.press('Shift+Tab');
  expect(await page.evaluate(() => !!document.activeElement.closest('.sheet'))).toBe(true);
  await page.keyboard.press('Escape');
  expect(await page.evaluate(() => document.activeElement.textContent)).toContain('Add training');
});

// ---------- Polish from the UI review ----------

test('Light mode: text colours read at 4.5:1 on cards and the page', async ({ page }) => {
  await page.emulateMedia({ colorScheme: 'light' });
  await open(page, 'seed=demo&layout=new');
  const ratios = await page.evaluate(() => {
    const css = getComputedStyle(document.documentElement);
    const rgb = (v) => { const c = document.createElement('canvas').getContext('2d'); c.fillStyle = v; const h = c.fillStyle.slice(1); return [0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16) / 255); };
    const lum = (v) => { const [r, g, b] = rgb(v).map((x) => (x <= 0.03928 ? x / 12.92 : ((x + 0.055) / 1.055) ** 2.4)); return 0.2126 * r + 0.7152 * g + 0.0722 * b; };
    const cr = (a, b) => { const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p); return (x + 0.05) / (y + 0.05); };
    const t = (n) => css.getPropertyValue(n).trim();
    const out = {};
    for (const n of ['--muted', '--accent', '--energy', '--fiber', '--protein', '--over', '--warn', '--good']) out[n] = Math.min(cr(t(n), t('--surface')), cr(t(n), t('--bg')));
    return out;
  });
  for (const [n, r] of Object.entries(ratios)) expect(r, n).toBeGreaterThanOrEqual(4.5);
});

test('Phone Log: Food right after the totals; toasts clear the Add button; 40 px targets', async ({ page }) => {
  await page.setViewportSize({ width: 360, height: 740 });
  await open(page, 'seed=demo&layout=new&hhmm=12:00');
  const y = async (sel) => (await page.locator(sel).boundingBox()).y;
  expect(await y('[aria-label="Food logged"]')).toBeLessThan(await y('[aria-label="Planned meals"]'));
  expect(await y('[aria-label="Energy and protein"]')).toBeLessThan(await y('[aria-label="Food logged"]'));
  await page.getByLabel('Ate Tofu bowl').check();
  await page.evaluate(() => { const t = document.createElement('div'); t.className = 'toast'; t.textContent = 'A message long enough to span the whole width of a phone screen'; document.getElementById('toasts').append(t); });
  const toast = await page.locator('.toast').last().boundingBox();
  const fab = await page.locator('#open-add').boundingBox();
  expect(toast.y + toast.height).toBeLessThanOrEqual(fab.y);
  for (const b of await page.locator('#screen-today button.icon.sm:visible, #screen-today .chip:visible').all()) {
    expect((await b.boundingBox()).height).toBeGreaterThanOrEqual(40);
  }
});

test('Training: chart date labels never touch, and the log sits beside the summary on desktop', async ({ page }) => {
  await page.setViewportSize({ width: 375, height: 800 });
  await open(page, 'seed=rich&layout=new');
  await tab(page, 'training').click();
  await expect(page.locator('#screen-training .chart-label').first()).toBeVisible();
  const boxes = await page.locator('#screen-training .chart-label').evaluateAll((els) => els.map((e) => e.getBBox()).map((b) => [b.x, b.x + b.width]));
  for (let i = 1; i < boxes.length; i++) expect(boxes[i][0], `label ${i}`).toBeGreaterThan(boxes[i - 1][1] + 2);
  await page.setViewportSize({ width: 1280, height: 900 });
  const summary = await page.locator('.card[aria-label="Last four weeks"]').boundingBox();
  const log = await page.locator('.card[aria-label="Training log"]').boundingBox();
  expect(log.x).toBeGreaterThan(summary.x + summary.width - 1);
  expect(log.width).toBeLessThan(900);
});

test('Plan: meals say kcal, day cards in a row are the same height', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await open(page, 'seed=rich&layout=new&hhmm=13:10');
  await tab(page, 'plan').click();
  await expect(page.locator('.plan-meal').first()).toContainText('kcal');
  const cards = await page.locator('.plan-day').evaluateAll((els) => els.map((e) => [Math.round(e.getBoundingClientRect().top), Math.round(e.getBoundingClientRect().height)]));
  const rows = {};
  for (const [top, hgt] of cards) (rows[top] ||= new Set()).add(hgt);
  for (const set of Object.values(rows)) expect(set.size).toBe(1);
});

test('History: nutrient bars line up across groups; day rows read their numbers', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await open(page, 'seed=rich&layout=new');
  await tab(page, 'history').click();
  const tracks = await page.locator('.nut-groups .pct-track').evaluateAll((els) => els.map((e) => { const r = e.getBoundingClientRect(); return [Math.round(r.left), Math.round(r.width)]; }));
  expect(tracks.length).toBeGreaterThan(3);
  expect(new Set(tracks.map(([l]) => l)).size).toBe(1);
  expect(new Set(tracks.map(([, w]) => w)).size).toBe(1);
  await expect(page.locator('.day-row').first()).toHaveAttribute('aria-label', /kcal, .* g protein/);
});

test('Log meals show a fiber subtotal; the header keeps its height on Settings', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await open(page, 'seed=rich&layout=new&hhmm=20:00');
  // The fourth cell of a meal's header is its fiber: the sum of its rows' fiber.
  const sums = await page.locator('.food-group').evaluateAll((gs) => gs.map((g) => {
    const head = g.querySelector('.food-head').children[3].textContent.trim();
    const rows = [...g.querySelectorAll('.food-row:not(.food-head)')].map((r) => parseFloat(r.children[3]?.textContent)).filter((x) => !Number.isNaN(x));
    return [head, rows.length ? Math.round(rows.reduce((a, b) => a + b, 0) * 10) / 10 : null];
  }));
  const withFiber = sums.filter(([, sum]) => sum !== null);
  expect(withFiber.length).toBeGreaterThan(0);
  for (const [head, sum] of withFiber) expect(Number(head)).toBeCloseTo(sum, 0);
  const h1 = (await page.locator('header').first().boundingBox()).height;
  await page.locator('#open-settings').click();
  const h2 = (await page.locator('header').first().boundingBox()).height;
  expect(h2).toBe(h1);
});

test('The app opens in the Log · Plan · Kitchen · Meds · Training · History layout', async ({ page }) => {
  await page.goto('/?seed=demo');
  await expect(page.locator('body[data-ready="1"]')).toBeAttached();
  await expect(page.locator('#title')).toHaveText('Log');
  await expect(page.locator('nav.tabs button[data-tab]')).toHaveText([/Log/, /Plan/, /Kitchen/, /Meds/, /Training/, /History/]);
  await expect(page.locator('.layout-switcher')).toHaveCount(0);
});

test('Desktop sidebar shows the app icon beside the name at the bottom; narrow sidebar shows just the icon', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await open(page, 'seed=demo&layout=new');
  const icon = page.locator('nav.tabs .brand-icon');
  await expect(icon).toBeVisible();
  expect(await icon.evaluate((i) => i.complete && i.naturalWidth > 0)).toBe(true);
  const ib = await icon.boundingBox();
  const tb = await page.locator('nav.tabs .brand span').boundingBox();
  expect(Math.abs(ib.y + ib.height / 2 - (tb.y + tb.height / 2))).toBeLessThan(4);
  expect(tb.x).toBeGreaterThan(ib.x + ib.width);
  // At the foot of the sidebar, below every tab.
  const nav = await page.locator('nav.tabs').boundingBox();
  const lastTab = await page.locator('nav.tabs button[data-tab]').last().boundingBox();
  expect(ib.y).toBeGreaterThan(lastTab.y + lastTab.height);
  expect(nav.y + nav.height - (ib.y + ib.height)).toBeLessThan(40);
  await page.setViewportSize({ width: 860, height: 800 });
  await expect(icon).toBeVisible();
  await expect(page.locator('nav.tabs .brand span')).toBeHidden();
  await page.setViewportSize({ width: 375, height: 800 });
  await expect(icon).toBeHidden();
});

test('History "Day by day": a day opens its nutrients in place, not the Log', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await open(page, 'seed=rich&layout=new&hhmm=20:00');
  await tab(page, 'history').click();
  await page.locator('details > summary', { hasText: 'Day by day' }).click();
  const row = page.locator('.day-row').first();
  await expect(row).toHaveAttribute('aria-expanded', 'false');
  await row.click();
  await expect(page.locator('#title')).toHaveText('History'); // stayed here
  await expect(row).toHaveAttribute('aria-expanded', 'true');
  const panel = page.locator('.day-nutrients').first();
  await expect(panel.locator('.nut-group h3').first()).toBeVisible();
  await expect(panel.locator('.pct-row').first()).toContainText('%');
  expect(await panel.locator('.pct-row').count()).toBeGreaterThan(5);
  await row.click();
  await expect(panel).toBeHidden();
  await expect(row).toHaveAttribute('aria-expanded', 'false');
  // A day with only typed-in numbers says so instead of showing empty bars.
  const typed = page.locator('.day-row', { hasText: /kcal/ }).last();
  await typed.click();
  await expect(page.locator('.day-nutrients:not(.hidden)')).toBeVisible();
});

test('Sleep on the Log: bed and wake times save, show the length, and can be cleared', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await open(page, 'seed=demo&layout=new&hhmm=09:00');
  const card = page.locator('.card[aria-label="Sleep"]');
  await expect(card).toContainText('How did you sleep last night?');
  await page.locator('#sleep-bed').fill('23:30');
  await page.locator('#sleep-bed').blur();
  expect(await page.evaluate(() => window.__mock.sleep['2026-11-22'])).toBeUndefined(); // waits for both
  await page.locator('#sleep-wake').fill('07:00');
  await page.locator('#sleep-wake').blur();
  await expect(page.getByTestId('sleep-length')).toHaveText('7 h 30 min asleep');
  expect(await page.evaluate(() => window.__mock.sleep['2026-11-22'])).toMatchObject({ bed: '23:30', wake: '07:00' });
  // Another day shows its own night; coming back shows this one again.
  await page.getByLabel('Previous day').click();
  await expect(page.locator('#sleep-bed')).toHaveValue('');
  await expect(page.getByTestId('sleep-length')).toHaveText('Not logged');
  await page.locator('.strip-day.is-today').click();
  await expect(page.locator('#sleep-bed')).toHaveValue('23:30');
  // The same time twice is refused with a message.
  await page.locator('#sleep-wake').fill('23:30');
  await page.locator('#sleep-wake').blur();
  await expect(card.locator('.error')).toContainText("can't be the same");
  await card.getByRole('button', { name: 'Clear sleep' }).click();
  await expect(page.getByTestId('sleep-length')).toHaveText('How did you sleep last night?');
  expect(await page.evaluate(() => window.__mock.sleep['2026-11-22'])).toBeUndefined();
});

test('Sleep in History: average tile, nightly chart, and on each day', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await open(page, 'seed=rich&layout=new');
  await tab(page, 'history').click();
  const tile = page.locator('.stat-tile', { hasText: 'Avg sleep' });
  await expect(tile).toContainText(/\d(\.\d)? h/);
  await expect(tile).toContainText(/bed \d\d:\d\d · up \d\d:\d\d/);
  await expect(page.locator('.card[aria-label="Sleep"] .chart svg')).toBeVisible();
  await expect(page.locator('.insights')).toContainText(/Sleeping \d h/);
  await page.locator('details > summary', { hasText: 'Day by day' }).click();
  const row = page.locator('.day-row').nth(1);
  await expect(row).toContainText(/slept \d h/);
  await row.click();
  await expect(page.locator('.day-nutrients:not(.hidden) .day-sleep')).toContainText(/in bed \d\d:\d\d, up \d\d:\d\d/);
});

test('Plan: the floating + adds foods to any day of the next two weeks', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await open(page, 'seed=demo&layout=new&hhmm=12:00');
  await tab(page, 'plan').click();
  await page.locator('#open-plan-add').click();
  const panel = page.locator('.overlay-sheet', { has: page.locator('#plan-search') });
  await expect(panel.locator('.plan-day-chips .chip')).toHaveCount(14);
  await panel.locator('.plan-day-chips .chip[data-v="2026-11-28"]').click();
  await expect(page.getByTestId('plan-add-day')).toContainText('nothing planned yet');
  await page.locator('#plan-search').fill('lentils');
  await panel.locator('.food-hit').first().click();
  await panel.locator('.food-picked .chip[data-v="Dinner"]').click();
  await page.locator('#plan-search-grams').fill('150');
  await panel.getByRole('button', { name: 'Add to plan' }).click();
  await expect(panel.locator('[role="status"]').last()).toContainText('· Dinner');
  await expect(page.getByTestId('plan-add-day')).toContainText('1 planned');
  // The panel stays open for the next one; the board behind it already shows the meal.
  await expect(page.locator('#plan-search')).toBeVisible();
  await page.keyboard.press('Escape');
  await page.getByRole('button', { name: '14 days' }).click();
  await expect(page.locator('[data-plan-day="2026-11-28"]')).toContainText('· 150 g');
});

test('Plan: a shopping day shows its trip with Open trip; the Log shows Open list on the day', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await open(page, 'seed=rich&layout=new&hhmm=09:00');
  await tab(page, 'plan').click();
  // The seed has Trip 1 on the day after tomorrow.
  const tile = page.locator('[data-shop-day="2026-11-24"]');
  await expect(tile).toContainText('Shopping · Trip 1');
  await expect(tile).toContainText(/\d+ items to buy · CHF \d+\.\d\d/);
  // Make today a Trip 2 day.
  await page.getByLabel('Make Today a shopping day').click();
  await page.getByRole('dialog').getByRole('button', { name: 'Trip 2' }).click();
  await expect(page.locator('[data-shop-day="2026-11-22"]')).toContainText('Trip 2');
  await page.locator('[data-shop-day="2026-11-22"]').getByRole('button', { name: 'Open trip' }).click();
  await expect(page.locator('#title')).toHaveText('Kitchen');
  await expect(page.locator('[data-part="Shopping"]')).toHaveAttribute('aria-pressed', 'true');
  await expect(page.locator('[data-trip="2"]')).toHaveAttribute('aria-pressed', 'true');
  // On the Log, today: Shopping today with Open list.
  await tab(page, 'today').click();
  const today = page.locator('.card[aria-label="Shopping today"]');
  await expect(today).toContainText('Shopping today · Trip 2');
  await today.getByRole('button', { name: 'Open list' }).click();
  await expect(page.locator('[data-trip="2"]')).toHaveAttribute('aria-pressed', 'true');
  // Another day on the Log doesn't show it; clearing the day removes it.
  await tab(page, 'plan').click();
  await page.getByLabel('No shopping on Today').click();
  await expect(page.locator('[data-shop-day="2026-11-22"]')).toHaveCount(0);
  await tab(page, 'today').click();
  await expect(page.locator('.card[aria-label="Shopping today"]')).toBeHidden();
});

test('Draft lines work like a food diary: change the amount, unit, meal or the food itself', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await open(page, 'seed=demo&layout=new&hhmm=19:00');
  await page.locator('#open-add').click();
  await page.locator('#describe').fill('a bowl of lentils');
  await page.getByRole('button', { name: 'Work out the calories' }).click();
  const row = page.locator('#draft [data-draft="0"]');
  await expect(row).toContainText('USDA');
  await expect(row).toContainText('amount guessed');
  const kcalOf = async () => Number((await row.locator('.draft-num').textContent()).match(/^(\d+) kcal/)[1]);
  const before = await kcalOf();
  // Halve the grams: numbers come back from the database, the guess note goes.
  const amount = row.locator('.draft-amount');
  const grams = Number(await amount.inputValue());
  await amount.fill(String(grams / 2));
  await amount.press('Tab');
  await expect.poll(kcalOf).toBe(Math.round(before / 2));
  await expect(row).not.toContainText('amount guessed');
  // The food's own portions are units too.
  const units = await row.locator('.draft-unit option').allTextContents();
  expect(units[0]).toBe('g');
  if (units.length > 1) {
    await row.locator('.draft-unit').selectOption({ index: 1 });
    await expect(amount).toHaveValue('1');
  }
  await row.locator('.draft-meal').selectOption('Lunch');
  // Swap the food for another match.
  await row.locator('.draft-food').click();
  const sheet = page.getByRole('dialog', { name: /Choose the food/ });
  await sheet.locator('#draft-food-q').fill('chickpeas');
  await expect(sheet.locator('.food-results').last().locator('.food-hit').first()).toContainText(/chickpea/i);
  await sheet.locator('.food-results').last().locator('.food-hit').first().click();
  await expect(row.locator('.draft-food')).toContainText('Chickpeas');
  await expect(row.locator('.draft-meal')).toHaveValue('Lunch');
  await page.locator('#draft').getByRole('button', { name: 'Save' }).click();
  const saved = await page.evaluate(() => Object.values(window.__mock.log).find((l) => /Chickpeas/.test(l.name)));
  expect(saved.meal).toBe('Lunch');
  expect(saved.foodRef).toMatch(/^usda:/);
});
