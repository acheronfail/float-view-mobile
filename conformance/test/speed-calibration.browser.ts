import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import {
  startMockServer,
  stopMockServer,
  getMockBootstrap,
  simulateMockCalibrationLocation,
  simulateMockSpeedCalibrationResult,
  simulateMockSpeedCalibrationProgress,
} from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(
    `${name}: calibration is explicit, accessible and keeps unsupported recommendations`,
    { timeout: 90_000 },
    async () => {
      const port = await startMockServer(0, { safetyDisclaimerAccepted: true });
      const endpoint = getMockBootstrap(port);
      const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
      const browser = await engine.launch();
      try {
        await client.request('bluetooth.scan', { timeoutMs: 100 });
        const scanned = await client.waitForState((s) => s.discoveredDevices.length > 0);
        await client.request('device.connect', {
          deviceId: scanned.discoveredDevices[0]!.deviceId,
          trackingMode: 'saved',
        });
        await client.waitForState((s) => s.activeConnection?.state === 'ready');
        simulateMockCalibrationLocation();
        const page = await browser.newPage({
          viewport: { width: 390, height: 844 },
          hasTouch: true,
          reducedMotion: 'reduce',
        });
        page.setDefaultTimeout(5000);
        if (name === 'chromium')
          await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
        const methods: string[] = [];
        const errors: string[] = [];
        page.on('pageerror', (e) => errors.push(e.message));
        await page.route(`${endpoint.origin}/`, (r) => r.fulfill({ contentType: 'text/html', body: html }));
        await page.route(`${endpoint.origin}/api/bootstrap`, (r) =>
          r.fulfill({ json: { webSocketUrl: endpoint.webSocketUrl, platform: 'web' } })
        );
        await page.routeWebSocket('**/api/ws?*', (socket) => {
          const server = socket.connectToServer();
          socket.onMessage((raw) => {
            const v = JSON.parse(String(raw));
            if (v.method) methods.push(v.method);
            server.send(raw);
          });
          server.onMessage((raw) => socket.send(raw));
        });
        await page.goto(`${endpoint.origin}/#/live`);
        await page.getByRole('button', { name: 'Board actions', exact: true }).click();
        await page.getByRole('button', { name: 'Calibrate speed…', exact: true }).click();
        const panel = page.getByRole('dialog', { name: 'Calibrate speed', exact: true });
        await panel.waitFor();
        assert.match(await panel.innerText(), /We’ll compare GPS and board speed to recommend a wheel diameter/);
        assert.doesNotMatch(await panel.innerText(), /Motor poles|gearing|phone away|flat route/);
        const start = panel.getByRole('button', { name: 'Start calibration', exact: true });
        assert.equal(methods.includes('speedCalibration.start'), false, 'opening never starts capture');
        await start.dispatchEvent('pointerdown', { button: 2, isPrimary: true, pointerId: 9 });
        await start.dispatchEvent('pointerup', { button: 2, isPrimary: true, pointerId: 9 });
        assert.equal(methods.includes('speedCalibration.start'), false);
        await start.dispatchEvent('pointerdown', { button: 0, isPrimary: false, pointerId: 10 });
        await start.dispatchEvent('pointerup', { button: 0, isPrimary: false, pointerId: 10 });
        assert.equal(methods.includes('speedCalibration.start'), false);
        const box = (await start.boundingBox())!;
        await start.evaluate((el) =>
          el.addEventListener(
            'pointerdown',
            (event) => {
              el.dataset.activePointer = String((event as PointerEvent).pointerId);
            },
            { once: true }
          )
        );
        await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
        await page.mouse.down();
        const pointerId = Number(await start.getAttribute('data-active-pointer'));
        await start.dispatchEvent('pointercancel', { isPrimary: true, pointerId });
        await page.mouse.up();
        assert.equal(methods.includes('speedCalibration.start'), false);
        await start.tap();
        await panel.getByRole('region', { name: 'Speed comparison' }).waitFor();
        await panel.getByRole('heading', { name: 'Waiting for GPS', exact: true }).waitFor();
        assert.doesNotMatch(await panel.innerText(), /Ride steadily|phone away|minute|clear view/);
        assert.equal(methods.filter((m) => m === 'speedCalibration.start').length, 1);
        assert.equal(await panel.getByRole('button', { name: 'Finish calibration', exact: true }).count(), 0);
        const progress = panel.getByRole('progressbar', { name: 'Calibration progress' });
        assert.equal(await progress.getAttribute('aria-valuenow'), '0');
        assert.match(await panel.innerText(), /Approximate progress/);
        assert.doesNotMatch(await panel.innerText(), /Time remaining|3:00/);
        assert.doesNotMatch(await panel.innerText(), /Measurements kept|Finishes automatically|Comparisons pause/);
        simulateMockSpeedCalibrationProgress('collecting', 32, 70);
        await panel.getByRole('heading', { name: 'Measuring', exact: true }).waitFor();
        await client.waitForState((s) => s.speedCalibration.acceptedSamples === 32);
        assert.equal(await progress.getAttribute('aria-valuenow'), '70');
        simulateMockSpeedCalibrationProgress('speedChanging', 32, 70);
        await page.waitForTimeout(300);
        assert.equal(
          await panel.getByRole('heading', { name: 'Measuring', exact: true }).isVisible(),
          true,
          'brief pauses do not flicker'
        );
        simulateMockSpeedCalibrationProgress('collecting', 33, 74);
        await client.waitForState((s) => s.speedCalibration.acceptedSamples === 33);
        simulateMockSpeedCalibrationProgress('speedChanging', 33, 74);
        const changing = panel.getByRole('heading', { name: 'Keep speed steady', exact: true });
        await changing.waitFor();
        assert.equal(await progress.getAttribute('aria-valuenow'), '74');
        assert.equal(await progress.locator('div').evaluate((el) => getComputedStyle(el).transitionProperty), 'none');
        assert.equal((await client.request('state.get', {})).speedCalibration.acceptedSamples, 33);
        assert.doesNotMatch(await panel.innerText(), /Measurements kept|Comparisons pause|Finishes automatically/);
        assert.ok((await changing.evaluate((el) => parseFloat(getComputedStyle(el).fontSize))) >= 30);
        assert.ok(
          (await panel
            .locator('[data-calibration-speed]')
            .evaluate((el) => parseFloat(getComputedStyle(el).fontSize))) >= 64
        );
        for (const width of [320, 390, 768]) {
          await page.setViewportSize({ width, height: 844 });
          assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
        }
        await page.setViewportSize({ width: 390, height: 844 });
        if (name === 'chromium')
          await page.screenshot({ path: '/tmp/float-view-speed-calibration-collecting.png', fullPage: true });
        for (const themePreference of ['light', 'dark'] as const) {
          const state = await client.request('state.get', {});
          await client.request('settings.update', {
            expectedRevision: state.settings.revision,
            patch: { themePreference },
          });
          await page.waitForFunction((theme) => document.documentElement.dataset.theme === theme, themePreference);
          const colors = await progress.evaluate((el) => ({
            track: getComputedStyle(el).backgroundColor,
            fill: getComputedStyle(el.querySelector('div')!).backgroundColor,
          }));
          assert.notEqual(colors.fill, colors.track);
          assert.notEqual(colors.fill, 'rgba(0, 0, 0, 0)');
          if (name === 'chromium')
            await page.screenshot({
              path: `/tmp/float-view-calibration-progress-${themePreference}.png`,
              fullPage: true,
            });
        }
        await panel.getByRole('button', { name: 'Close', exact: true }).click();
        await page.getByRole('button', { name: 'Board actions', exact: true }).click();
        await page.getByRole('button', { name: 'Calibrate speed…', exact: true }).click();
        await panel.getByRole('heading', { name: 'Keep speed steady', exact: true }).waitFor();
        assert.equal(await progress.getAttribute('aria-valuenow'), '74', 'progress survives view recreation');
        assert.equal(
          methods.filter((m) => m === 'speedCalibration.start').length,
          1,
          'closing and reopening keeps capture'
        );
        // Cancellation is the only collection action; keyboard activation ends it once.
        await panel.getByRole('button', { name: 'Cancel', exact: true }).focus();
        await page.keyboard.press('Enter');
        await panel.getByRole('heading', { name: 'Match board speed to GPS' }).waitFor();
        assert.equal(methods.filter((m) => m === 'speedCalibration.cancel').length, 1);
        await panel.getByRole('button', { name: 'Start calibration', exact: true }).tap();
        await panel.getByRole('region', { name: 'Speed comparison' }).waitFor();
        assert.equal(await progress.getAttribute('aria-valuenow'), '0', 'a new attempt resets progress');
        simulateMockSpeedCalibrationResult(268.8, false);
        await panel.getByRole('heading', { name: 'Measuring', exact: true }).waitFor();
        assert.match(await panel.innerText(), /More measurements may improve precision/);
        assert.equal((await client.request('state.get', {})).speedCalibration.status, 'refining');
        await panel.getByRole('button', { name: 'Close', exact: true }).click();
        await page.getByRole('button', { name: 'Board actions', exact: true }).click();
        await page.getByRole('button', { name: 'Calibrate speed…', exact: true }).click();
        await panel.getByRole('heading', { name: 'Measuring', exact: true }).waitFor();
        simulateMockSpeedCalibrationProgress('gpsUncertain', 40, 100);
        await panel.getByRole('heading', { name: 'GPS speed uncertain', exact: true }).waitFor();
        assert.equal(await panel.getByRole('button', { name: 'Finish calibration', exact: true }).isEnabled(), true);
        assert.ok((await panel.getByRole('status').getAttribute('class'))?.includes('bg-warning/10'));
        if (name === 'chromium')
          await page.screenshot({ path: '/tmp/float-view-calibration-refining-gps-dark.png', fullPage: true });
        simulateMockSpeedCalibrationProgress('lowSpeed', 40, 100);
        await panel.getByRole('heading', { name: 'Speed too low', exact: true }).waitFor();
        assert.match(await panel.innerText(), /Minimum 7\.2 km\/h/);
        assert.equal(await panel.getByRole('button', { name: 'Finish calibration', exact: true }).isEnabled(), true);
        if (name === 'chromium')
          await page.screenshot({ path: '/tmp/float-view-calibration-refining-low-speed-dark.png', fullPage: true });
        simulateMockSpeedCalibrationProgress('ready', 41, 100);
        await panel.getByRole('heading', { name: 'Measuring', exact: true }).waitFor();
        assert.ok((await panel.getByRole('status').getAttribute('class'))?.includes('bg-accent/10'));

        for (const width of [320, 390, 768]) {
          await page.setViewportSize({ width, height: 844 });
          assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
        }
        await page.setViewportSize({ width: 390, height: 844 });
        if (name === 'chromium')
          await page.screenshot({ path: '/tmp/float-view-calibration-refining.png', fullPage: true });
        for (const themePreference of ['light', 'dark'] as const) {
          const state = await client.request('state.get', {});
          await client.request('settings.update', {
            expectedRevision: state.settings.revision,
            patch: { themePreference },
          });
          await page.waitForFunction((theme) => document.documentElement.dataset.theme === theme, themePreference);
          if (name === 'chromium')
            await page.screenshot({
              path: `/tmp/float-view-calibration-refining-${themePreference}.png`,
              fullPage: true,
            });
        }
        await panel.getByRole('button', { name: 'Finish calibration', exact: true }).tap();
        assert.equal(methods.filter((m) => m === 'speedCalibration.finish').length, 1);
        await panel.getByRole('heading', { name: 'Recommended wheel diameter' }).waitFor();
        assert.equal((await client.request('state.get', {})).speedCalibration.progressPercent, 100);
        assert.equal(await progress.count(), 0);
        assert.equal(await panel.getByRole('button', { name: /^Apply / }).count(), 0);
        assert.match(await panel.innerText(), /268\.8/);
        assert.match(await panel.innerText(), /VESC Tool/);
        assert.equal(methods.includes('speedCalibration.apply'), false);
        for (const width of [320, 390, 768]) {
          await page.setViewportSize({ width, height: 844 });
          assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
        }
        if (name === 'chromium')
          await page.screenshot({ path: '/tmp/float-view-speed-calibration.png', fullPage: true });
        await panel.getByRole('button', { name: 'Done', exact: true }).click();
        await page.getByRole('button', { name: 'Board actions', exact: true }).click();
        await page.getByRole('button', { name: 'Calibrate speed…', exact: true }).click();
        await panel.getByRole('heading', { name: 'Recommended wheel diameter' }).waitFor();
        assert.equal(
          methods.filter((m) => m === 'speedCalibration.start').length,
          2,
          'reopening never replays capture'
        );
        assert.deepEqual(errors, []);
      } finally {
        await browser.close();
        await client.close();
        await stopMockServer();
      }
    }
  );
}
