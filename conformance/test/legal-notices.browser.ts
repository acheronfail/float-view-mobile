import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');

for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: licences open in a separate scrollable route offline`, { timeout: 60_000 }, async () => {
    const port = await startMockServer(11002, { safetyDisclaimerAccepted: true });
    const endpoint = getMockBootstrap(port);
    const browser = await engine.launch();
    try {
      const page = await browser.newPage({ hasTouch: true, viewport: { width: 390, height: 700 } });
      if (name === 'chromium') {
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      }
      const externalRequests: string[] = [];
      await page.route('**/*', async (route) => {
        if (route.request().url().startsWith(endpoint.origin)) await route.continue();
        else {
          externalRequests.push(route.request().url());
          await route.abort();
        }
      });
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.goto(`${endpoint.origin}/#/settings`);
      const section = page.getByRole('region', { name: 'Open source and licences' });
      await section.waitFor();
      assert.match(await section.innerText(), /Copyright © 2026 acheronfail/);
      assert.match(await section.innerText(), /Luis Prado/);
      assert.equal(await section.getByRole('link', { name: 'Privacy policy', exact: true }).getAttribute('href'),
        'https://github.com/acheronfail/float-view-mobile/blob/master/PRIVACY.md');
      assert.equal(await section.getByRole('link', { name: 'Support', exact: true }).getAttribute('href'),
        'https://github.com/acheronfail/float-view-mobile/issues');
      const button = section.getByRole('link', { name: 'View licences and notices' });
      for (const pointer of [
        { button: 2, isPrimary: true },
        { button: 0, isPrimary: false },
      ]) {
        await button.dispatchEvent('pointerdown', { ...pointer, pointerId: 1 });
        await button.dispatchEvent('pointerup', { ...pointer, pointerId: 1 });
      }
      await button.dispatchEvent('pointerdown', { button: 0, isPrimary: true, pointerId: 2 });
      await button.dispatchEvent('pointercancel', { pointerId: 2 });
      await button.dispatchEvent('pointerup', { button: 0, isPrimary: true, pointerId: 2 });
      assert.ok(page.url().endsWith('#/settings'));
      assert.equal(await page.getByRole('region', { name: 'Project licence', exact: true }).count(), 0);
      await button.tap();
      await page.getByRole('heading', { name: 'Licences and notices', exact: true }).waitFor();
      assert.ok(page.url().endsWith('#/settings/licences'));
      const text = await page.locator('main').innerText();
      assert.match(text, /GNU AFFERO GENERAL PUBLIC LICENSE/);
      assert.match(text, /Volodymyr Agafonkin/);
      assert.match(text, /Cole Bemis/);
      assert.match(text, /Luis Prado/);
      assert.match(text, /Apache License/);
      assert.match(text, /Rust Standard Library/);
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
      await page.evaluate(() => window.scrollTo(0, 2000));
      assert.ok(await page.evaluate(() => window.scrollY > 0));
      const back = page.getByRole('link', { name: 'Settings', exact: true });
      await back.focus();
      await page.keyboard.press('Enter');
      await section.waitFor();
      assert.ok(page.url().endsWith('#/settings'));
      // Keyboard opens the route too; native Back returns to Settings.
      await button.focus();
      await page.keyboard.press('Enter');
      await page.getByRole('heading', { name: 'Licences and notices', exact: true }).waitFor();
      await page.evaluate(() => window.dispatchEvent(new Event('floatview:back')));
      await section.waitFor();
      assert.deepEqual(externalRequests, []);
    } finally {
      await browser.close();
      await stopMockServer();
    }
  });
}
