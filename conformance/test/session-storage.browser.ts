import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(
    `${name}: session storage footers share settings styling and update after deletion`,
    { timeout: 60_000 },
    async () => {
      const port = await startMockServer(11002);
      const endpoint = getMockBootstrap(port);
      const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
      const browser = await engine.launch();
      try {
        const page = await browser.newPage({ viewport: { width: 390, height: 844 }, hasTouch: true });
        if (name === 'chromium')
          await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
        const errors: string[] = [];
        page.on('pageerror', (error) => errors.push(error.message));
        await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
        await page.goto(`${endpoint.origin}/#/settings`);
        const version = page.locator('footer[aria-label="Application version"]');
        await version.waitFor();
        const footerClass = await version.getAttribute('class');
        await page.goto(`${endpoint.origin}/#/sessions`);
        const storage = page.locator('footer[aria-label="Session storage"]');
        await storage.getByText('Sessions on disk · 308.0 KB', { exact: true }).waitFor();
        assert.equal(await storage.getAttribute('class'), footerClass);
        const state = await client.request('state.get', {});
        const board = state.savedDevices[0]!;
        await page.goto(`${endpoint.origin}/#/sessions/boards/${board.savedDeviceId}`);
        await storage.getByText('Sessions on disk · 308.0 KB', { exact: true }).waitFor();
        assert.equal(await storage.getAttribute('class'), footerClass);
        await client.request('session.delete', { sessionId: 'session-1' });
        await storage.getByText('Sessions on disk · 154.0 KB', { exact: true }).waitFor();
        await page.goto(`${endpoint.origin}/#/sessions`);
        await storage.getByText('Sessions on disk · 154.0 KB', { exact: true }).waitFor();
        await client.request('session.delete', { sessionId: 'session-2' });
        await storage.getByText('Sessions on disk · 0 B', { exact: true }).waitFor();
        assert.deepEqual(errors, []);
      } finally {
        await browser.close();
        client.close();
        await stopMockServer();
      }
    }
  );
}
