import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import type { AppState } from '../../frontend/src/lib/backend/contract.ts';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: most recent ride stays mounted during snapshot and catalog updates`, { timeout: 60_000 }, async () => {
    const endpoint = getMockBootstrap(await startMockServer(11002, { safetyDisclaimerAccepted: true }));
    const browser = await engine.launch();
    try {
      const page = await browser.newPage();
      page.setDefaultTimeout(5000);
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      let requests = 0;
      let publishSnapshot: (catalogChanged?: boolean) => void = () => assert.fail('socket not ready');
      let releaseResponse: (() => void) | null = null;
      let holdResponse = false;
      let responseHeldResolve: () => void;
      const responseHeld = new Promise<void>((resolve) => {
        responseHeldResolve = resolve;
      });
      await page.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        let state: AppState;
        publishSnapshot = (catalogChanged = false) => {
          state = {
            ...state,
            revision: state.revision + 1,
            sessionCatalogRevision: state.sessionCatalogRevision + Number(catalogChanged),
          };
          socket.send(JSON.stringify({ event: 'state.changed', data: state }));
        };
        const listIds = new Set<string>();
        socket.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          if (message.method === 'session.list') {
            requests += 1;
            listIds.add(message.id);
          }
          server.send(raw);
        });
        server.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          if (message.result?.permissions) state = message.result;
          if (message.event === 'state.changed') state = message.data;
          if (listIds.delete(message.id)) {
            message.result.catalogRevision = state.sessionCatalogRevision;
            if (holdResponse) {
              message.result.items[0].totalElapsedMs = 42 * 60_000;
              releaseResponse = () => socket.send(JSON.stringify(message));
              responseHeldResolve();
              return;
            }
          }
          socket.send(JSON.stringify(message));
        });
      });
      await page.goto(`${endpoint.origin}/#/sessions`);
      const shortcut = page.getByRole('link', { name: /^Most recent ride/ });
      await shortcut.waitFor();
      const original = await shortcut.elementHandle();
      const initialRequests = requests;
      for (let index = 0; index < 20; index += 1) {
        publishSnapshot();
        // Allow each independent snapshot to reach Svelte before publishing the next.
        await page.evaluate(() => new Promise(requestAnimationFrame));
      }
      assert.equal(requests, initialRequests, 'unchanged catalog must not reload the shortcut');
      assert.ok(await original!.evaluate((element) => element.isConnected), 'same card remains mounted');

      holdResponse = true;
      publishSnapshot(true);
      await responseHeld;
      assert.equal(requests, initialRequests + 1, 'catalog change refreshes once');
      assert.ok(releaseResponse, 'refresh response is held');
      assert.ok(await original!.evaluate((element) => element.isConnected), 'card stays mounted while refreshing');
      (releaseResponse as () => void)();
      await shortcut.getByText(/42 min/).waitFor();
      assert.ok(await original!.evaluate((element) => element.isConnected), 'refresh preserves the card');
    } finally {
      await browser.close();
      await stopMockServer();
    }
  });

  test(`${name}: most recent ride shortcut is conditional and opens the latest ride`, { timeout: 60_000 }, async () => {
    const endpoint = getMockBootstrap(await startMockServer(11002, { safetyDisclaimerAccepted: true }));
    const browser = await engine.launch();
    try {
      const page = await browser.newPage({ viewport: { width: 390, height: 844 }, hasTouch: true });
      page.setDefaultTimeout(5000);
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      let hasRides = false;
      const requestedBoards: string[] = [];
      await page.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        socket.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          if (message.method === 'session.list') requestedBoards.push(message.params.savedDeviceId);
          server.send(raw);
        });
        server.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          const state = message.result?.permissions
            ? message.result
            : message.event === 'state.changed'
              ? message.data
              : null;
          if (state) {
            const board = state.savedDevices[0];
            state.savedDevices = [
              {
                ...board,
                savedDeviceId: 'older-board',
                displayName: 'Older board',
                summary: { ...board.summary, mostRecentSessionAt: hasRides ? '2025-01-01T00:00:00Z' : null },
              },
              {
                ...board,
                displayName: 'Latest board',
                summary: { ...board.summary, mostRecentSessionAt: hasRides ? board.summary.mostRecentSessionAt : null },
              },
            ];
          }
          socket.send(JSON.stringify(message));
        });
      });
      await page.goto(`${endpoint.origin}/#/sessions`);
      await page.getByRole('link', { name: /Latest board/ }).waitFor();
      const shortcut = page.getByRole('link', { name: /^Most recent ride/ });
      assert.equal(await shortcut.count(), 0);
      assert.deepEqual(requestedBoards, []);
      const boardList = page.getByRole('region', { name: 'Saved boards', exact: true });
      for (const width of [320, 390, 768]) {
        await page.setViewportSize({ width, height: 844 });
        const headerBox = await page.locator('header').boundingBox();
        const firstBoardBox = await boardList.getByRole('link').first().boundingBox();
        assert.ok(headerBox && firstBoardBox);
        assert.equal(
          firstBoardBox.y - (headerBox.y + headerBox.height),
          16,
          `header gap without shortcut at ${width}px`
        );
      }

      hasRides = true;
      await page.reload();
      await shortcut.waitFor();
      assert.match(await shortcut.innerText(), /Latest board/);
      assert.match((await shortcut.getAttribute('href')) ?? '', /^#\/sessions\/session-2\?board=/);
      assert.equal(await shortcut.locator('svg').count(), 0, 'shortcut has no board icon');
      assert.ok(requestedBoards.length > 0);
      assert.ok(
        requestedBoards.every((id) => id !== 'older-board'),
        'only query the latest board'
      );
      for (const width of [320, 390, 768]) {
        await page.setViewportSize({ width, height: 844 });
        const headerBox = await page.locator('header').boundingBox();
        const shortcutBox = await shortcut.boundingBox();
        const firstBoardBox = await boardList.getByRole('link').first().boundingBox();
        assert.ok(headerBox && shortcutBox && firstBoardBox);
        assert.equal(shortcutBox.y - (headerBox.y + headerBox.height), 16, `header gap at ${width}px`);
        assert.equal(firstBoardBox.y - (shortcutBox.y + shortcutBox.height), 16, `board gap at ${width}px`);
      }
      await shortcut.click({ button: 'right' });
      assert.ok(page.url().endsWith('#/sessions'));
      await shortcut.focus();
      await page.keyboard.press('Enter');
      await page.waitForURL('**/#/sessions/session-2?board=*');
      await page.goto(`${endpoint.origin}/#/sessions`);
      await shortcut.tap();
      await page.waitForURL('**/#/sessions/session-2?board=*');
    } finally {
      await browser.close();
      await stopMockServer();
    }
  });
}
