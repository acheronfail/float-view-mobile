import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: diagnostics preview and manual reporting`, { timeout: 60_000 }, async () => {
    const port = await startMockServer(11002, { safetyDisclaimerAccepted: true });
    const endpoint = getMockBootstrap(port);
    const browser = await engine.launch();
    try {
      const page = await browser.newPage({
        viewport: { width: name === 'webkit' ? 390 : 1280, height: 844 },
        hasTouch: true,
      });
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.goto(`${endpoint.origin}/#/settings`);
      const open = page.getByRole('button', { name: 'Export diagnostics', exact: true });
      await open.waitFor();
      await open.click({ button: 'right' });
      assert.equal(await page.getByRole('button', { name: 'Mark a problem', exact: true }).count(), 0);
      await open.focus();
      await page.keyboard.press('Enter');
      await page.getByRole('button', { name: 'Preview recent context', exact: true }).click();
      await page.getByLabel('Diagnostic report preview').waitFor();
      assert.match(await page.getByLabel('Diagnostic report preview').innerText(), /Excludes routes/);
      const requests: string[] = [];
      page.on('request', (r) => {
        if (r.url().includes('github.com')) requests.push(r.url());
      });
      await page.getByRole('button', { name: 'Save/share ZIP', exact: true }).click();
      const link = page.getByRole('link', { name: 'GitHub Issues', exact: true });
      await link.waitFor();
      assert.equal(await link.getAttribute('href'), 'https://github.com/acheronfail/float-view-mobile/issues');
      assert.equal(requests.length, 0);
      assert.match(await page.locator('#diagnostics').innerText(), /manually write an issue/);
      await page.screenshot({ path: `/tmp/float-view-diagnostics-${name}.png`, fullPage: true });
      await page.getByRole('button', { name: 'Clear retained diagnostics', exact: true }).tap();
      await page.getByRole('status').filter({ hasText: 'Retained diagnostics cleared' }).waitFor();
      assert.equal(await page.getByLabel('Diagnostic report preview').count(), 0);
    } finally {
      await browser.close();
      await stopMockServer();
    }
  });
}
