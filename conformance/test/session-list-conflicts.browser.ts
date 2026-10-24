import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  for (const scenario of ['first page', 'persistent conflict', 'later page', 'other error']) {
    test(`${name}: session list recovers safely from ${scenario}`, { timeout: 60_000 }, async () => {
      const endpoint = getMockBootstrap(await startMockServer(11002, { safetyDisclaimerAccepted: true }));
      const browser = await engine.launch();
      try {
        const page = await browser.newPage({ viewport: { width: 390, height: 844 }, hasTouch: true });
        page.setDefaultTimeout(5000);
        if (name === 'chromium')
          await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
        await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
        const cursors: (string | null)[] = [];
        let fail = true;
        await page.routeWebSocket('**/api/ws?*', (socket) => {
          const server = socket.connectToServer();
          const listIds = new Set<number>();
          socket.onMessage((raw) => {
            const message = JSON.parse(String(raw));
            // The most-recent-ride shortcut uses limit=1; exercise the board's list.
            if (message.method === 'session.list' && message.params.limit === 50) {
              cursors.push(message.params.cursor ?? null);
              const shouldFail =
                fail &&
                (scenario === 'persistent conflict' ||
                  scenario === 'other error' ||
                  (scenario === 'first page' && cursors.length === 1) ||
                  (scenario === 'later page' && cursors.length === 2));
              if (shouldFail) {
                socket.send(
                  JSON.stringify({
                    id: message.id,
                    error: {
                      code: scenario === 'other error' ? 'not_found' : 'conflict',
                      message: scenario === 'other error' ? 'Board not found' : 'Resource revision changed',
                    },
                  })
                );
                return;
              }
              listIds.add(message.id);
              // The mock does not know our deliberately stale transport cursor.
              message.params.cursor = null;
            }
            server.send(JSON.stringify(message));
          });
          server.onMessage((raw) => {
            const message = JSON.parse(String(raw));
            if (listIds.delete(message.id) && scenario === 'later page') {
              const summary = message.result.items[0];
              assert.ok(summary, 'mock provides a recorded session');
              const firstPage = cursors.length === 1;
              message.result.items = Array.from({ length: firstPage ? 50 : 1 }, (_, index) => ({
                ...summary,
                sessionId: firstPage ? `stale-${index}` : 'fresh-session',
              }));
              message.result.nextCursor = firstPage ? 'stale-cursor' : null;
            }
            socket.send(JSON.stringify(message));
          });
        });
        await page.goto(`${endpoint.origin}/#/sessions`);
        await page.getByRole('region', { name: 'Saved boards', exact: true }).getByRole('link').first().tap();
        await page.waitForURL('**/#/sessions/boards/*');
        const sessions = page.getByRole('region', { name: 'Sessions', exact: true });
        if (scenario === 'persistent conflict' || scenario === 'other error') {
          await sessions.getByRole('alert').waitFor();
          assert.equal(cursors.length, scenario === 'persistent conflict' ? 2 : 1);
          // No retry loop while the error is visible, including on another render.
          await page.setViewportSize({ width: 400, height: 844 });
          await page.evaluate(() => new Promise(requestAnimationFrame));
          assert.equal(cursors.length, scenario === 'persistent conflict' ? 2 : 1);
          fail = false;
          await sessions.getByRole('button', { name: 'Try again' }).tap();
          await sessions.getByRole('link').first().waitFor();
          assert.equal(await sessions.getByRole('alert').count(), 0);
        } else if (scenario === 'later page') {
          await sessions.locator('a[href*="stale-49"]').waitFor();
          await sessions.getByLabel('Load more sessions').scrollIntoViewIfNeeded();
          await sessions.locator('a[href*="fresh-session"]').waitFor();
          assert.deepEqual(cursors, [null, 'stale-cursor', null]);
          assert.equal(await sessions.locator('a[href*="stale-"]').count(), 0);
          assert.equal(await sessions.getByRole('link').count(), 1);
        } else {
          await sessions.getByRole('link').first().waitFor();
          assert.deepEqual(cursors, [null, null]);
          assert.equal(await sessions.getByRole('alert').count(), 0);
        }
      } finally {
        await browser.close();
        await stopMockServer();
      }
    });
  }
}
