import { test, expect } from '@playwright/test';

async function open(page, query = 'seed=demo') {
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
    await page.getByLabel('Delete Skyr · 200 g').click();
    await expect(page.getByTestId('kcal')).toHaveText('248');
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
    await page.locator('#describe').fill('200g chicken breast, mystery stew');
    await page.getByRole('button', { name: 'Work out the calories' }).click();
    const draft = page.locator('#draft');
    await expect(draft.getByTestId('provider')).toHaveText('Claude worked it out');
    await expect(draft.locator('[data-draft]')).toHaveCount(2);
    await expect(draft.getByTestId('draft-total')).toHaveText('730 kcal · 82 g protein');
    await draft.getByLabel('Remove mystery stew').click();
    await expect(draft.getByTestId('draft-total')).toHaveText('330 kcal · 62 g protein');
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
    await expect(page.getByLabel('Log Egg again')).toBeVisible();
    await page.getByLabel('Log Egg again').click();
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

test('Settings: validation, AI test, learned foods, backup and restore', async ({ page }) => {
  await open(page);
  await page.getByLabel('Settings').click();
  await page.locator('#set-weight').fill('20');
  await page.getByRole('button', { name: 'Save targets' }).click();
  await expect(page.locator('.sheet .error').first()).toHaveText('Weight must be between 30 and 300 kg.');
  await page.locator('#set-weight').fill('80');
  await page.getByRole('button', { name: 'Save targets' }).click();
  await expect(page.locator('.toast').last()).toHaveText('Targets saved');

  await expect(page.locator('#ai-key')).toHaveAttribute('type', 'password');
  await page.locator('#ai-key').fill('test-key');
  await page.locator('#ai-gmodel').fill('bad model!');
  await page.getByRole('button', { name: 'Test the AI' }).click();
  await expect(page.getByTestId('ai-status')).toContainText('Model names may only use');
  await page.locator('#ai-gmodel').fill('gemini-flash-latest');
  await page.getByRole('button', { name: 'Test the AI' }).click();
  await expect(page.getByTestId('ai-status')).toHaveText('Claude answered. AI is working.');
  expect(await page.evaluate(() => window.__mock.ai.geminiKey)).toBe('test-key');

  await page.getByLabel('Forget Protein pudding').click();
  await expect(page.locator('#learned')).toContainText('Foods the AI works out');

  await page.getByRole('button', { name: 'Back up…' }).click();
  await expect(page.locator('.toast').last()).toHaveText('Backup saved');
  await page.getByRole('button', { name: 'Restore / import…' }).click();
  await expect(page.locator('.toast').last()).toHaveText('Imported 3 records');
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
  await page.getByLabel('Settings').click();
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
  await expect(page.locator('.card .error').first()).toHaveText('Name the item.');
  await page.evaluate(() => (window.__mock.aiAvailable = false));
  await page.getByLabel('Settings').click();
  await page.getByRole('button', { name: 'Test the AI' }).click();
  await expect(page.getByTestId('ai-status')).toContainText('No AI is set up');
});

test('All buttons have accessible names', async ({ page }) => {
  await open(page);
  for (const t of ['today', 'add', 'meds', 'stock', 'week', 'shop']) {
    await tab(page, t).click();
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
  await page.getByLabel('Settings').click();
  const sw = await page.evaluate(() => document.querySelector('.sheet').scrollWidth);
  expect(sw).toBeLessThanOrEqual(360);
});

test.describe('dark mode', () => {
  test.use({ colorScheme: 'dark' });
  test('follows the system', async ({ page }) => {
    await open(page);
    const bg = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);
    expect(bg).toBe('rgb(21, 24, 26)');
    const accent = await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue('--accent').trim());
    expect(accent.toUpperCase()).toBe('#6FB57F');
  });
});

test('light mode colours', async ({ page }) => {
  await page.emulateMedia({ colorScheme: 'light' });
  await open(page);
  expect(await page.evaluate(() => getComputedStyle(document.body).backgroundColor)).toBe('rgb(242, 243, 240)');
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
  const dlg = await page.locator('.sheet').boundingBox();
  expect(dlg.y).toBeGreaterThan(10); // centered dialog, not a bottom sheet
  expect(dlg.y + dlg.height).toBeLessThan(800 - 10);
});
