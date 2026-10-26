import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: search live values and select with touch or keyboard`, { timeout: 30_000 }, async () => {
    const port = await startMockServer(11002, { safetyDisclaimerAccepted: true });
    const endpoint = getMockBootstrap(port);
    const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
    let browser: Awaited<ReturnType<typeof engine.launch>> | undefined;
    try {
      browser = await engine.launch();
      await client.request('bluetooth.requestPermission', {});
      await client.request('bluetooth.scan', { timeoutMs: 100 });
      const scanned = await client.waitForState((s) => s.discoveredDevices.length > 0);
      await client.request('device.connect', {
        deviceId: scanned.discoveredDevices[0]!.deviceId,
        trackingMode: 'saved',
      });
      await client.waitForState((s) => s.activeConnection?.state === 'ready');
      const page = await browser.newPage({
        viewport: { width: 390, height: 844 },
        hasTouch: true,
        reducedMotion: 'reduce',
      });
      page.setDefaultTimeout(5000);
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      const errors: string[] = [];
      page.on('pageerror', (error) => errors.push(error.message));
      let saves = 0;
      let rejectSelection = false;
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        socket.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          if (message.method === 'boardProfile.update' && message.params?.patch?.liveWidgetSlots) {
            saves++;
            if (rejectSelection) {
              socket.send(
                JSON.stringify({ id: message.id, error: { code: 'invalid_request', message: 'Test save failure' } })
              );
              return;
            }
          }
          server.send(raw);
        });
        server.onMessage((raw) => socket.send(raw));
      });
      await page.goto(`${endpoint.origin}/#/live`);
      const firstBox = page.getByRole('button', { name: 'Choose value for box 1: Empty', exact: true });
      await firstBox.focus();
      await page.keyboard.press('Enter');
      const picker = page.getByRole('dialog', { name: 'Choose a value' });
      const search = picker.getByRole('searchbox', { name: 'Search values', exact: true });
      await search.waitFor();
      const choiceCount = await picker.getByRole('button').count();
      await search.fill('  BaTtErY  ');
      assert.ok((await picker.getByRole('button').count()) < choiceCount);
      assert.equal(await picker.getByRole('button', { name: 'Battery current', exact: true }).count(), 1);
      assert.equal(await picker.getByRole('button', { name: 'Speed', exact: true }).count(), 0);
      await search.fill('battery');
      for (const theme of ['light', 'dark'] as const) {
        await page.emulateMedia({ colorScheme: theme });
        await page.waitForFunction((theme) => document.documentElement.dataset.theme === theme, theme);
        if (process.env.SCREENSHOT_DIR) {
          await mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
          await page.screenshot({ path: `${process.env.SCREENSHOT_DIR}/picker-${name}-${theme}.png` });
        }
      }
      if (process.env.SCREENSHOT_DIR) {
        await page.setViewportSize({ width: 1280, height: 800 });
        await page.screenshot({ path: `${process.env.SCREENSHOT_DIR}/picker-${name}-desktop.png` });
        await page.setViewportSize({ width: 390, height: 844 });
      }
      await search.fill('no such value');
      await picker.getByRole('status').filter({ hasText: 'No matching values.' }).waitFor();
      assert.equal(await picker.getByRole('button').count(), 1);
      await search.fill('');
      assert.equal(await picker.getByRole('button').count(), choiceCount);
      await search.fill('headlights');
      assert.equal(await picker.getByRole('button', { name: 'Headlights', exact: true }).count(), 1);
      assert.equal(saves, 0, 'filtering never saves a selection');
      await picker.getByRole('button', { name: 'Cancel', exact: true }).tap();
      await picker.waitFor({ state: 'hidden' });
      await firstBox.focus();
      await page.keyboard.press('Enter');
      assert.equal(await search.inputValue(), '', 'reopening clears the search');
      await search.fill('pack voltage');
      const voltage = picker.getByRole('button', { name: 'Pack voltage', exact: true });
      await voltage.click({ button: 'right' });
      assert.equal(saves, 0, 'secondary input never selects a result');
      rejectSelection = true;
      await voltage.tap();
      await picker.getByRole('alert').filter({ hasText: 'Test save failure' }).waitFor();
      assert.equal(await search.inputValue(), 'pack voltage', 'failed saves retain the search');
      rejectSelection = false;
      await voltage.tap();
      await picker.waitFor({ state: 'hidden' });
      assert.equal(saves, 2, 'one request per touch activation');
      const voltageBox = page.getByRole('button', { name: 'Choose value for box 1: Pack voltage', exact: true });
      await voltageBox.focus();
      await page.keyboard.press('Enter');
      await search.fill('  voltage CHARGING ');
      assert.equal(await picker.getByRole('button').count(), 2, 'search matches every word in any order');
      await picker.getByRole('button', { name: 'Charging voltage', exact: true }).focus();
      await page.keyboard.press('Enter');
      await picker.waitFor({ state: 'hidden' });
      assert.equal(saves, 3, 'one request per keyboard activation');
      assert.equal((await client.request('state.get', {})).activeConnection!.liveWidgetSlots[0], 'chargingVoltageV');
      assert.deepEqual(errors, []);
    } finally {
      await browser?.close();
      client.close();
      await stopMockServer();
    }
  });
}
