import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: saved discoveries sort first with distinct cards`, { timeout: 60_000 }, async () => {
    const port = await startMockServer(11002, { safetyDisclaimerAccepted: true });
    const endpoint = getMockBootstrap(port);
    const browser = await engine.launch();
    try {
      for (const colorScheme of ['light', 'dark'] as const) {
        const page = await browser.newPage({ colorScheme, hasTouch: true, viewport: { width: 390, height: 844 } });
        page.setDefaultTimeout(5000);
        if (name === 'chromium')
          await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
        await page.routeWebSocket(/\/api\/ws/, (socket) => {
          const server = socket.connectToServer();
          server.onMessage((raw) => {
            const message = JSON.parse(String(raw));
            const state = message.result?.permissions
              ? message.result
              : message.event === 'state.changed'
                ? message.data
                : null;
            if (state) {
              state.discoveredDevices = [
                { deviceId: 'nearby', name: 'Nearby board', savedDeviceId: null, rssi: -50 },
                { deviceId: 'saved', name: 'Bluetooth UART', savedDeviceId: 'saved-1', rssi: -80 },
              ];
              state.savedDevices = [{ ...state.savedDevices[0], savedDeviceId: 'saved-1', displayName: 'My board' }];
            }
            socket.send(JSON.stringify(message));
          });
        });
        await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
        await page.goto(endpoint.origin);
        const cards = page.getByRole('region', { name: 'VESC devices' }).getByRole('button');
        await cards.first().getByText('Saved board', { exact: true }).waitFor();
        assert.match(await cards.nth(0).innerText(), /My board/);
        assert.match(await cards.nth(1).innerText(), /Nearby board/);
        const styles = await cards.evaluateAll((elements) =>
          elements.map((element) => {
            const style = getComputedStyle(element);
            const rect = element.getBoundingClientRect();
            return {
              background: style.backgroundColor,
              border: style.borderColor,
              right: rect.right,
              width: element.clientWidth,
              content: element.scrollWidth,
            };
          })
        );
        const [saved, nearby] = styles;
        assert.ok(saved && nearby);
        assert.notEqual(saved.background, nearby.background);
        assert.notEqual(saved.border, nearby.border);
        for (const style of styles) {
          assert.ok(style.right <= 390);
          assert.ok(style.content <= style.width, 'card content fits the mobile viewport');
        }
        await page.close();
      }
    } finally {
      await browser.close();
      await stopMockServer();
    }
  });
}
