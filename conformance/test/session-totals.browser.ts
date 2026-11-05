import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: session totals and quiet Settings header`, { timeout: 60_000 }, async () => {
    const port = await startMockServer(11002, { safetyDisclaimerAccepted: true });
    const endpoint = getMockBootstrap(port);
    const browser = await engine.launch();
    try {
      for (const timezoneId of ['Australia/Adelaide', 'America/Los_Angeles']) {
        const page = await browser.newPage({ locale: 'en-US', timezoneId });
        if (name === 'chromium')
          await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
        const errors: string[] = [];
        page.on('pageerror', (error) => errors.push(error.message));
        await page.clock.install({ time: new Date('2026-01-01T00:07:00Z') });
        await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
        await page.goto(`${endpoint.origin}/#/sessions`);
        const totals = page.getByLabel('Session totals', { exact: true });
        await totals.getByText('0.60 km', { exact: true }).waitFor();
        assert.deepEqual(
          await totals
            .locator('dd')
            .allTextContents()
            .then((values) => values.map((value) => value.trim())),
          ['0.60 km', '0.0 h', '2']
        );
        const subtitle = page.locator('header time');
        await subtitle.waitFor();
        assert.equal((await subtitle.innerText()).trim(), 'Last ride was 5 minutes ago');
        assert.equal(await subtitle.getAttribute('datetime'), '2026-01-01T00:02:00.000Z');
        await page.clock.fastForward(60_000);
        assert.equal((await subtitle.innerText()).trim(), 'Last ride was 6 minutes ago');
        await page.clock.fastForward(3_600_000);
        assert.equal((await subtitle.innerText()).trim(), 'Last ride was 1 hour ago');
        await page.clock.fastForward(86_400_000);
        assert.equal((await subtitle.innerText()).trim(), 'Last ride was yesterday');
        await page.getByRole('link', { name: /^Most recent ride/ }).waitFor();
        for (const width of [320, 390, 1024]) {
          await page.setViewportSize({ width, height: 844 });
          assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
          if (process.env.SESSION_SCREENSHOTS && name === 'chromium' && width !== 320) {
            await mkdir(process.env.SESSION_SCREENSHOTS, { recursive: true });
            await page.screenshot({ path: `${process.env.SESSION_SCREENSHOTS}/sessions-${width}.png` });
          }
        }
        await page.getByRole('button', { name: 'Session actions', exact: true }).click();
        await page.getByRole('button', { name: 'Import session', exact: true }).waitFor();
        await page.goto(`${endpoint.origin}/#/settings`);
        await page.getByRole('navigation', { name: 'Settings categories' }).waitFor();
        assert.doesNotMatch(await page.locator('header').innerText(), /tracked across|Loading ride totals/);
        assert.deepEqual(errors, []);
        await page.close();
      }
    } finally {
      await browser.close();
      await stopMockServer();
    }
  });
}
