import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';
const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: resume setting and End ride work by pointer and keyboard`, { timeout: 60_000 }, async () => {
    const port = await startMockServer(11002, { safetyDisclaimerAccepted: true });
    const endpoint = getMockBootstrap(port);
    const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
    const browser = await engine.launch();
    try {
      await client.request('bluetooth.requestPermission', {});
      await client.request('bluetooth.scan', { timeoutMs: 100 });
      const scanned = await client.waitForState((s) => s.discoveredDevices.length > 0);
      await client.request('device.connect', {
        deviceId: scanned.discoveredDevices[0]!.deviceId,
        trackingMode: 'saved',
      });
      const ready = await client.waitForState((s) => s.activeConnection?.state === 'ready');
      const sessionId = ready.activeConnection!.activeSession!.sessionId;
      const page = await browser.newPage({ hasTouch: true, reducedMotion: 'reduce' });
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      page.setDefaultTimeout(8000);
      const errors: string[] = [];
      page.on('pageerror', (e) => errors.push(e.message));
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.route('https://tile.openstreetmap.org/**', (route) => route.abort());
      const sent: string[] = [];
      page.on('websocket', (socket) =>
        socket.on('framesent', ({ payload }) => {
          const m = JSON.parse(String(payload));
          sent.push(m.method);
        })
      );
      await page.goto(`${endpoint.origin}/#/live`);
      await page
        .getByRole('button', { name: 'Board actions', exact: true })
        .click()
        .catch(async (e) => {
          console.error(await page.locator('body').innerText(), errors);
          throw e;
        });
      await page.getByRole('button', { name: 'End ride', exact: true }).waitFor();
      assert.ok(
        await page.locator('button[aria-label="Disconnect"]').evaluate((button) => {
          const end = document.querySelector('button[aria-label="End ride"]');
          return !!end && !!(button.compareDocumentPosition(end) & Node.DOCUMENT_POSITION_FOLLOWING);
        })
      );
      await page.getByRole('button', { name: 'Disconnect', exact: true }).click();
      const confirm = page.getByRole('alertdialog', { name: 'Disconnect board?', exact: true });
      await confirm.getByRole('button', { name: 'Disconnect', exact: true }).click();
      const card = page.getByRole('complementary', { name: 'Paused ride' });
      await card.waitFor();
      assert.equal((await client.request('state.get', {})).pausedRide?.sessionId, sessionId);
      const homeEnd = card.getByRole('button', { name: 'End ride', exact: true });
      await homeEnd.tap();
      const homeConfirmation = page.getByRole('alertdialog', { name: 'End ride?', exact: true });
      await homeConfirmation.waitFor();
      assert.equal(sent.filter((m) => m === 'session.end').length, 0);
      await homeConfirmation.getByRole('button', { name: 'Cancel', exact: true }).click();
      await homeConfirmation.waitFor({ state: 'hidden' });
      assert.equal((await client.request('state.get', {})).pausedRide?.sessionId, sessionId);
      assert.ok(await homeEnd.evaluate((button) => button === document.activeElement));
      await page.goto(`${endpoint.origin}/#/settings`);
      const window = page.getByRole('spinbutton', { name: 'Resume ride within (minutes)' });
      await window.fill('0');
      await window.blur();
      await page.getByText('Enter a whole number from 1 to 1,440 minutes.').waitFor();
      await window.fill('120');
      await window.blur();
      await page.waitForFunction(
        () =>
          document.querySelector<HTMLInputElement>('input[aria-describedby="ride-resume-help ride-resume-error"]')
            ?.value === '120'
      );
      await client.waitForState((s) => s.settings.rideResumeWindowMinutes === 120);
      await page.goto(`${endpoint.origin}/#/sessions/${sessionId}`);
      const end = page
        .getByRole('complementary', { name: 'Paused ride' })
        .getByRole('button', { name: 'End ride', exact: true });
      await end.waitFor();
      await end.dispatchEvent('pointerdown', { pointerId: 2, pointerType: 'touch', isPrimary: false, button: 0 });
      assert.equal((await client.request('state.get', {})).pausedRide?.sessionId, sessionId);
      await end.focus();
      await page.keyboard.press('Enter');
      const pausedConfirmation = page.getByRole('alertdialog', { name: 'End ride?', exact: true });
      await pausedConfirmation.waitFor();
      assert.equal(sent.filter((m) => m === 'session.end').length, 0);
      await pausedConfirmation.getByRole('button', { name: 'End ride', exact: true }).focus();
      await page.keyboard.press('Enter');
      await client.waitForState((s) => s.pausedRide === null);
      assert.equal(sent.filter((m) => m === 'session.end').length, 1);
      assert.equal((await client.request('session.get', { sessionId })).summary.state, 'completed');
      await client.request('bluetooth.scan', { timeoutMs: 100 });
      await client.waitForState((s) => s.discoveredDevices.length > 0);
      await client.request('device.connect', {
        deviceId: scanned.discoveredDevices[0]!.deviceId,
        trackingMode: 'saved',
      });
      const next = await client.waitForState((s) => s.activeConnection?.state === 'ready');
      await page.goto(`${endpoint.origin}/#/live`);
      await page.getByRole('button', { name: 'Board actions', exact: true }).click();
      await page.getByRole('button', { name: 'End ride', exact: true }).click();
      const ending = page.getByRole('alertdialog', { name: 'End ride?', exact: true });
      await ending.waitFor();
      assert.equal(
        (await client.request('state.get', {})).activeConnection?.activeSession?.sessionId,
        next.activeConnection!.activeSession!.sessionId
      );
      assert.equal(sent.filter((m) => m === 'session.end').length, 1);
      await ending.getByRole('button', { name: 'Cancel', exact: true }).click();
      await ending.waitFor({ state: 'hidden' });
      assert.equal(
        (await client.request('state.get', {})).activeConnection?.activeSession?.sessionId,
        next.activeConnection!.activeSession!.sessionId
      );
      await page.getByRole('button', { name: 'Board actions', exact: true }).click();
      await page.getByRole('button', { name: 'End ride', exact: true }).click();
      await ending.getByRole('button', { name: 'End ride', exact: true }).tap();
      await client.waitForState((s) => s.activeConnection === null);
      assert.equal((await client.request('state.get', {})).pausedRide, null);
      assert.equal(sent.filter((m) => m === 'session.end').length, 2);
      assert.deepEqual(errors, []);
    } finally {
      client.close();
      await browser.close();
      await stopMockServer();
    }
  });
}
