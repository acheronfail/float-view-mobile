import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: session board cards keep equal heights across summary lengths`, { timeout: 60_000 }, async () => {
    const endpoint = getMockBootstrap(await startMockServer(11002, { safetyDisclaimerAccepted: true }));
    const browser = await engine.launch();
    try {
      const page = await browser.newPage({
        viewport: { width: 390, height: 844 },
        hasTouch: true,
        reducedMotion: 'reduce',
      });
      page.setDefaultTimeout(5000);
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        server.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          const state = message.result?.permissions
            ? message.result
            : message.event === 'state.changed'
              ? message.data
              : null;
          if (state) {
            const board = state.savedDevices[0];
            state.savedDevices = [0, 1, 2].map((index) => ({
              ...board,
              savedDeviceId: index === 0 ? board.savedDeviceId : `board-${index}`,
              displayName: index === 2 ? 'A very long board name that must not make its card taller' : `Board ${index}`,
              summary: {
                ...board.summary,
                sessionCount: [0, 12, 12345][index],
                totalRidingElapsedMs: [0, 59 * 60000, 12345 * 3600000][index],
                totalDistanceM: [0, 12345, 123456789][index],
              },
            }));
          }
          socket.send(JSON.stringify(message));
        });
      });
      await page.goto(`${endpoint.origin}/#/sessions`);
      const cards = page.getByRole('region', { name: 'Saved boards', exact: true }).getByRole('link');
      await cards.nth(2).waitFor();
      for (const width of [320, 390, 768, 1280]) {
        await page.setViewportSize({ width, height: 844 });
        const sizes = await cards.evaluateAll((elements) =>
          elements.map((element) => {
            const rect = element.getBoundingClientRect();
            return {
              height: rect.height,
              scrollWidth: element.scrollWidth,
              clientWidth: element.clientWidth,
              stats: [...element.querySelectorAll('dd')].map((stat) => stat.getBoundingClientRect().height),
            };
          })
        );
        assert.equal(new Set(sizes.map((size) => size.height)).size, 1, `equal card heights at ${width}px`);
        assert.equal(new Set(sizes.flatMap((size) => size.stats)).size, 1, `single-line stats at ${width}px`);
        assert.ok(
          sizes.every((size) => size.scrollWidth === size.clientWidth),
          `no horizontal overflow at ${width}px`
        );
      }
      assert.deepEqual(await cards.first().locator('dt').allTextContents(), ['Sessions', 'Riding time', 'Distance']);
      await cards.first().click({ button: 'right' });
      assert.ok(page.url().endsWith('#/sessions'));
      await cards.first().focus();
      await page.keyboard.press('Enter');
      await page.waitForURL('**/#/sessions/boards/*');
      await page.goto(`${endpoint.origin}/#/sessions`);
      await cards.first().tap();
      await page.waitForURL('**/#/sessions/boards/*');
    } finally {
      await browser.close();
      await stopMockServer();
    }
  });
}
