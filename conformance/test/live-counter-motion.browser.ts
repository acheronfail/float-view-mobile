import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import type { LiveWidgetField } from '../../frontend/src/lib/backend/contract.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
const counters: LiveWidgetField[] = [
  'wattHoursDrawnWh',
  'wattHoursChargedWh',
  'wattHoursNetWh',
  'ampHoursDrawnAh',
  'ampHoursChargedAh',
  'ampHoursNetAh',
  'odometerM',
];
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(
    `${name}: cumulative readings spread updates across a second without slowing power`,
    { timeout: 30_000 },
    async () => {
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
        const savedDeviceId = ready.activeConnection!.savedDeviceId!;
        const profile = await client.request('boardProfile.get', { savedDeviceId });
        await client.request('boardProfile.update', {
          savedDeviceId,
          expectedRevision: profile.revision,
          patch: {
            lastLiveView: 'custom',
            customDashboard: {
              version: 1,
              widgets: [...counters, 'powerW' as const].map((field) => ({
                id: field,
                kind: 'reading',
                field,
                presentation: 'number',
                size: 'compact',
              })),
            },
          },
        });
        const page = await browser.newPage({ viewport: { width: 1024, height: 1000 }, reducedMotion: 'no-preference' });
        page.setDefaultTimeout(5000);
        if (name === 'chromium')
          await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
        const errors: string[] = [];
        page.on('pageerror', (e) => errors.push(e.message));
        let scale = 1;
        let available = true;
        let snapshots = 0;
        await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
        await page.routeWebSocket('**/api/ws?*', (socket) => {
          const server = socket.connectToServer();
          socket.onMessage((raw) => server.send(raw));
          server.onMessage((raw) => {
            const message = JSON.parse(String(raw));
            const state = message.event === 'state.changed' ? message.data : message.result;
            if (state?.activeConnection?.latestSample) {
              snapshots++;
              Object.assign(state.activeConnection.latestSample, {
                wattHoursDrawnWh: available ? 100 * scale : null,
                wattHoursChargedWh: available ? 20 * scale : null,
                ampHoursDrawnAh: available ? 100 * scale : null,
                ampHoursChargedAh: available ? 20 * scale : null,
                odometerM: available ? 100_000 * scale : null,
                packVoltageV: 75,
                batteryCurrentA: 10 * scale,
              });
            }
            socket.send(JSON.stringify(message));
          });
        });
        await page.goto(`${endpoint.origin}/#/live`);
        const reading = (field: LiveWidgetField) => page.locator(`[data-dashboard-widget="${field}"] .tabular-nums`);
        const value = async (field: LiveWidgetField) => Number.parseFloat(await reading(field).innerText());
        await page.waitForFunction(() =>
          document
            .querySelector('[data-dashboard-widget="wattHoursNetWh"] .tabular-nums')
            ?.textContent?.includes('80.0')
        );
        assert.equal(await value('odometerM'), 100, 'initial values appear immediately');
        const before = snapshots;
        scale = 2;
        // Power must finish its existing short tween while every cumulative value
        // is still on its way to the new target.
        await page.waitForFunction(() =>
          document.querySelector('[data-dashboard-widget="powerW"] .tabular-nums')?.textContent?.includes('+1500')
        );
        await page.waitForTimeout(200);
        for (const field of counters) {
          const initial = field.includes('Charged') ? 20 : field.includes('Net') ? 80 : 100;
          const current = await value(field);
          assert(current > initial && current < initial * 2, `${field} progresses over the slow interval: ${current}`);
        }
        await page.waitForTimeout(1100);
        assert(snapshots - before >= 5, 'repeated fast snapshots exercise unchanged counter targets');
        for (const field of counters) {
          const target = field.includes('Charged') ? 40 : field.includes('Net') ? 160 : 200;
          assert.equal(await value(field), target, `${field} reaches its target despite fast snapshots`);
        }
        available = false;
        await page.waitForFunction(() =>
          document.querySelector('[data-dashboard-widget="wattHoursNetWh"] .tabular-nums')?.textContent?.includes('—')
        );
        scale = 1;
        available = true;
        await page.waitForFunction(() =>
          document
            .querySelector('[data-dashboard-widget="wattHoursNetWh"] .tabular-nums')
            ?.textContent?.includes('80.0')
        );
        assert.equal(await value('wattHoursDrawnWh'), 100, 'recovered readings appear immediately');
        await page.emulateMedia({ reducedMotion: 'reduce' });
        scale = 3;
        await page.waitForFunction(() =>
          document.querySelector('[data-dashboard-widget="powerW"] .tabular-nums')?.textContent?.includes('+2250')
        );
        assert.equal(await value('wattHoursNetWh'), 240, 'reduced motion snaps counters to their target');
        assert.deepEqual(errors, []);
      } finally {
        client.close();
        await browser.close();
        await stopMockServer();
      }
    }
  );
}
