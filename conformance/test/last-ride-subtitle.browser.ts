import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: dynamic session and settings subtitles`, { timeout: 60_000 }, async () => {
    const port = await startMockServer(11002);
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
        const subtitle = page.locator('header time');
        await subtitle.waitFor();
        assert.equal((await subtitle.innerText()).trim(), 'Last ride 5 minutes ago');
        await page.clock.fastForward(60_000);
        assert.equal((await subtitle.innerText()).trim(), 'Last ride 6 minutes ago');
        await page.clock.fastForward(24 * 60 * 60_000);
        const expectedTime = timezoneId === 'Australia/Adelaide' ? '10:32 AM' : '4:02 PM';
        assert.equal((await subtitle.innerText()).trim(), `Last ride yesterday at ${expectedTime}`);
        assert.equal(await subtitle.getAttribute('datetime'), '2026-01-01T00:02:00.000Z');
        await page.getByRole('button', { name: 'Session actions', exact: true }).click();
        await page.getByRole('button', { name: 'Import session', exact: true }).waitFor();
        await page.goto(`${endpoint.origin}/#/settings`);
        await page.locator('header').getByText('0.60 km tracked across all rides', { exact: true }).waitFor();
        await page.getByRole('group', { name: 'Units', exact: true }).getByRole('button', { name: 'Imperial' }).click();
        await page.locator('header').getByText('0.37 mi tracked across all rides', { exact: true }).waitFor();
        await page.getByRole('group', { name: 'Units', exact: true }).getByRole('button', { name: 'Metric' }).click();
        await page.locator('header').getByText('0.60 km tracked across all rides', { exact: true }).waitFor();
        assert.deepEqual(errors, []);
        await page.close();
      }
    } finally {
      await browser.close();
      await stopMockServer();
    }
  });
}
