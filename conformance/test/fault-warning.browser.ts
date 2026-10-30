import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');

test('live fault warning survives a transient fault and clears only with its latch', { timeout: 90_000 }, async () => {
  const port = await startMockServer(11002, { safetyDisclaimerAccepted: true });
  const endpoint = getMockBootstrap(port);
  const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
  const browser = await chromium.launch();
  try {
    await client.request('bluetooth.requestPermission', {});
    await client.request('bluetooth.scan', { timeoutMs: 100 });
    const scanned = await client.waitForState((s) => s.discoveredDevices.length > 0);
    await client.request('device.connect', { deviceId: scanned.discoveredDevices[0]!.deviceId, trackingMode: 'guest' });
    await client.waitForState((s) => s.activeConnection?.state === 'ready');
    const page = await browser.newPage({
      viewport: { width: 320, height: 640 },
      hasTouch: true,
      reducedMotion: 'reduce',
    });
    page.setDefaultTimeout(5000);
    await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
    const errors: string[] = [];
    page.on('pageerror', (error) => errors.push(error.message));
    let phase: string = 'active';
    let disabled = false;
    let imuTimeout = false;
    let timeoutHistory = false;
    await page.route(`${endpoint.origin}/api/bootstrap`, (route) =>
      route.fulfill({ json: { webSocketUrl: endpoint.webSocketUrl, platform: 'web' } })
    );
    await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
    await page.routeWebSocket('**/api/ws?*', (socket) => {
      const server = socket.connectToServer();
      server.onMessage((raw) => {
        const message = JSON.parse(String(raw));
        const state =
          message.event === 'state.changed' ? message.data : message.result?.activeConnection ? message.result : null;
        const active = state?.activeConnection;
        if (active?.latestSample) {
          Object.assign(active.latestSample, {
            firmwareFault: phase === 'active' ? 'absoluteOverCurrent' : null,
            imuTimeoutActive: imuTimeout,
            faultHistory: [],
            imuTimeoutHistory: timeoutHistory
              ? [
                  { active: true, elapsedMs: 100, observedElapsedMs: 100 },
                  { active: false, elapsedMs: 200, observedElapsedMs: 200 },
                ]
              : [],
            stopCondition: 'imuTimeout',
            fatalErrorLatched: phase !== 'cleared',
            lastFirmwareFault:
              phase === 'unknown' ? { code: 255, reason: 'unknown' } : { code: 4, reason: 'absoluteOverCurrent' },
            alertReason: 'firmwareFault',
            packageState: disabled ? 'disabled' : 'ready',
            rideState: 'idle',
          });
          Object.assign(active.refloatControls, {
            packageEnabled: !disabled,
            fatalErrorLatched: phase !== 'cleared',
            fatalErrorActive: phase === 'active',
          });
        }
        socket.send(JSON.stringify(message));
      });
    });
    await page.goto(`${endpoint.origin}/#/live`);
    const warning = page.getByRole('alert').filter({ hasText: 'Firmware fault' });
    await warning.waitFor();
    assert.match(await warning.innerText(), /Absolute Over Current/);
    assert.match(await warning.innerText(), /controller fault/i);
    phase = 'latched';
    await page.waitForFunction(() =>
      [...document.querySelectorAll('[role="alert"]')].some((el) => el.textContent?.includes('latched'))
    );
    disabled = true;
    await page.getByRole('button', { name: 'Enable Refloat', exact: true }).waitFor();
    for (const viewport of [
      { width: 320, height: 640 },
      { width: 390, height: 844 },
      { width: 1280, height: 900 },
    ]) {
      await page.setViewportSize(viewport);
      const layout = await warning.evaluate((el) => {
        const container = el.closest('.live-warning-layer')!.parentElement!.getBoundingClientRect();
        const banner = el.getBoundingClientRect();
        const button = document.querySelector('[aria-label="Enable Refloat"]')!.getBoundingClientRect();
        return {
          fits: banner.left >= container.left && banner.right <= container.right && button.bottom <= container.bottom,
          overflow: document.documentElement.scrollWidth > innerWidth,
        };
      });
      assert.equal(layout.fits, true, 'fault and disabled warnings fit inside the gauge');
      assert.equal(layout.overflow, false);
    }
    await page.setViewportSize({ width: 320, height: 640 });
    await mkdir(new URL('../generated/', import.meta.url), { recursive: true });
    await page.screenshot({ path: fileURLToPath(new URL('../generated/fault-warning-320.png', import.meta.url)) });
    phase = 'unknown';
    await page.waitForFunction(() => document.body.textContent?.includes('code 255'));
    imuTimeout = true;
    const timeout = page.getByRole('alert').filter({ hasText: 'IMU timeout' });
    await timeout.waitFor();
    await warning.waitFor();
    await page.reload();
    await timeout.waitFor();
    await warning.waitFor();
    phase = 'cleared';
    await warning.waitFor({ state: 'hidden' });
    await timeout.waitFor();
    imuTimeout = false;
    await timeout.waitFor({ state: 'hidden' });
    const menu = page.getByRole('button', { name: 'Board actions', exact: true });
    await menu.tap();
    const actions = page.getByRole('dialog', { name: 'Board actions', exact: true });
    await actions.waitFor();
    const historyAction = page.getByRole('button', { name: 'Faults and errors…', exact: true });
    assert.equal(await historyAction.count(), 0, 'healthy inactive timeout does not establish history');
    await page.keyboard.press('Escape');
    await actions.waitFor({ state: 'hidden' });
    timeoutHistory = true;
    await page.reload();
    // Wait for the restored live snapshot before tapping a header that may
    // still be replaced during initial connection rendering.
    await page.getByRole('button', { name: 'Enable Refloat', exact: true }).waitFor();
    await menu.tap();
    await actions.waitFor();
    await historyAction.tap();
    const history = page.getByRole('dialog', { name: 'Faults and errors', exact: true });
    await history.waitFor();
    assert.match(await history.innerText(), /IMU timeout cleared/);
    assert.match(await history.innerText(), /Guest history/);
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    client.close();
    await stopMockServer();
  }
});
