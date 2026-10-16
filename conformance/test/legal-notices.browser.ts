import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');

for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: bundled licences work offline with touch and keyboard`, { timeout: 60_000 }, async () => {
    const port = await startMockServer(11002, { safetyDisclaimerAccepted: true });
    const endpoint = getMockBootstrap(port);
    const browser = await engine.launch();
    try {
      const page = await browser.newPage({ hasTouch: true });
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
      const button = section.getByRole('button');
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
      assert.equal(await button.getAttribute('aria-expanded'), 'false');
      await button.tap();
      await section.locator('#license-notices').waitFor();
      const text = await section.locator('#license-notices').innerText();
      assert.match(text, /GNU AFFERO GENERAL PUBLIC LICENSE/);
      assert.match(text, /Volodymyr Agafonkin/);
      assert.match(text, /Cole Bemis/);
      assert.match(text, /Luis Prado/);
      assert.match(text, /Apache License/);
      assert.equal(await button.getAttribute('aria-expanded'), 'true');
      await button.focus();
      await page.keyboard.press('Enter');
      await section.locator('#license-notices').waitFor({ state: 'detached' });
      await page.keyboard.press('Space');
      await section.locator('#license-notices').waitFor();
      assert.deepEqual(externalRequests, []);
    } finally {
      await browser.close();
      await stopMockServer();
    }
  });
}
