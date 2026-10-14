import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: prune cleanup progress supports touch and keyboard cancellation`, { timeout: 60_000 }, async () => {
    const port = await startMockServer(11002);
    const endpoint = getMockBootstrap(port);
    const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
    const browser = await engine.launch();
    try {
      const page = await browser.newPage({ viewport: { width: 390, height: 844 }, hasTouch: true });
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      // Hold the final mock phase on screen to represent a large ride's longer
      // cleanup. Requests and cancellation still use the real mock transport.
      await page.routeWebSocket(endpoint.webSocketUrl, (socket) => {
        const server = socket.connectToServer();
        server.onMessage((message) => {
          const value = JSON.parse(String(message));
          const operation = value.event === 'state.changed' ? value.data?.sessionOperation : undefined;
          if (operation?.kind === 'prune' && operation.status === 'completed') operation.status = 'reclaiming';
          socket.send(JSON.stringify(value));
        });
      });
      const errors: string[] = [];
      page.on('pageerror', (error) => errors.push(error.message));
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.goto(`${endpoint.origin}/#/sessions`);
      await page.locator('footer[aria-label="Session storage"]').waitFor();
      for (const activation of ['touch', 'keyboard']) {
        await client.request('session.prune.start', {
          scope: 'one',
          sessionId: 'session-1',
          targetResolutionSeconds: 5,
        });
        const status = page.getByRole('status').filter({ hasText: 'Reclaiming space…' });
        await status.waitFor();
        const cancel = status.locator('..').getByRole('button', { name: 'Cancel', exact: true });
        await cancel.click({ button: 'right' });
        assert.equal(await status.isVisible(), true);
        if (activation === 'touch') await cancel.tap();
        else {
          await cancel.focus();
          await page.keyboard.press('Enter');
        }
        await status.waitFor({ state: 'hidden' });
        assert.equal((await client.request('state.get', {})).sessionOperation?.status, 'cancelled');
      }
      assert.deepEqual(errors, []);
    } finally {
      await browser.close();
      client.close();
      await stopMockServer();
    }
  });
}
