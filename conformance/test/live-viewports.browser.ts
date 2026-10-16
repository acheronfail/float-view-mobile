import assert from 'node:assert/strict';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';
import { liveViewports } from './support/live-viewports.ts';
import type { BmsSnapshot } from '../../frontend/src/lib/backend/contract.ts';
import { writeLiveViewportGallery } from './support/live-viewport-gallery.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
const output = path.resolve(process.env.FLOAT_VIEW_VIEWPORT_OUTPUT ?? 'generated/live-viewports');
const expectedDetailColumns: Record<string, number> = {
  'iphone-13': 1,
  'fold-portrait': 2,
  'square-medium': 2,
  'square-compact': 2,
  'fold-wide': 3,
  'square-large': 3,
};
const longName = 'The Extremely Long Named Trail Board — Weekend Adventure Edition';

for (const [engineName, engine] of Object.entries({ chromium, webkit })) {
  test(`${engineName}: live viewport matrix`, { timeout: 180_000 }, async () => {
    const port = await startMockServer(0);
    const endpoint = getMockBootstrap(port);
    const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
    const browser = await engine.launch();
    const failures: string[] = [];
    const results: unknown[] = [];
    await mkdir(path.join(output, engineName), { recursive: true });
    try {
      await client.request('bluetooth.requestPermission', {});
      await client.request('bluetooth.scan', { timeoutMs: 100 });
      const scanned = await client.waitForState((s) => s.discoveredDevices.length > 0);
      await client.request('device.connect', {
        deviceId: scanned.discoveredDevices[0]!.deviceId,
        trackingMode: 'saved',
      });
      await client.waitForState((s) => s.activeConnection?.state === 'ready' && !!s.activeConnection.latestSample);
      const page = await browser.newPage({
        hasTouch: true,
        isMobile: true,
        deviceScaleFactor: 1,
        reducedMotion: 'reduce',
      });
      page.setDefaultTimeout(5000);
      if (engineName === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      page.on('pageerror', (error) => failures.push(error.message));
      let details = false;
      let withBms = false;
      let guest = false;
      let boardName = longName;
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
          if (state?.activeConnection?.latestSample) {
            const active = state.activeConnection;
            active.displayName = boardName;
            if (details && active.firmware) {
              active.firmware.hardwareName = 'THOR300_ExtendedHardwareName_ForNarrowTelemetryCards';
              active.controllerConnection = { kind: 'can', canId: 1 };
              active.transportFirmware = {
                hardwareName: 'VESC Express T',
                firmwareName: 'LongBluetoothGatewayFirmwareNameWithoutSpaces',
                vescVersion: '6.06',
                testVersion: 0,
              };
            }
            active.trackingMode = guest ? 'guest' : 'saved';
            if (guest) {
              active.savedDeviceId = null;
              active.activeSession = null;
              active.liveWidgetSlots = state.settings.liveWidgetSlots;
            }
            // Fix visible values while preserving authoritative revision, sequence and freshness.
            Object.assign(active.latestSample, {
              speedKph: 24.3,
              dutyCycle: 0.42,
              packVoltageV: 58.4,
              controllerTempC: 38.2,
              motorTempC: 41.7,
              ridingElapsedMs: 754000,
              distanceM: 4567,
              odometerM: 123456,
              batteryCurrentA: 490 / 58.4,
            });
            active.bms = {
              status: withBms ? 'attached' : 'not_found',
              connection: withBms ? { kind: 'can', canId: 42 } : null,
              candidates: [],
              latestSnapshot: withBms
                ? ({
                    observedAt: new Date().toISOString(),
                    totalVoltageV: 58.4,
                    chargeVoltageV: 67.2,
                    inputCurrentA: 8.4,
                    inputIcCurrentA: 8.4,
                    sessionAmpHoursAh: 0,
                    sessionWattHoursWh: 0,
                    cellVoltagesV: [3.65, 3.66],
                    balancing: [false, false],
                    temperaturesC: [43, 26.1],
                    icTemperatureC: 43,
                    humiditySensorTemperatureC: 35.1,
                    humidityPercent: 40,
                    highestCellTemperatureC: 26.1,
                    highestCircuitryTemperatureC: 43,
                    stateOfCharge: 0.68,
                    stateOfHealth: 1,
                    reportedCanId: 42,
                    lifetimeChargeAmpHoursAh: 0,
                    lifetimeChargeWattHoursWh: 0,
                    lifetimeDischargeAmpHoursAh: 0,
                    lifetimeDischargeWattHoursWh: 0,
                    pressurePa: 101325,
                    dataVersion: 1,
                    status: null,
                  } satisfies BmsSnapshot)
                : null,
            };
          }
          socket.send(JSON.stringify(message));
        });
      });
      await page.goto(`${endpoint.origin}/#/live`);
      const live = page.getByRole('region', { name: 'Live board widgets', exact: true });
      await live.waitFor();
      for (const colorScheme of ['light', 'dark'] as const) {
        await page.emulateMedia({ colorScheme });
        for (const viewport of liveViewports) {
          await page.setViewportSize(viewport.viewport);
          for (const scenario of ['saved', 'bms', 'guest'] as const) {
            withBms = scenario === 'bms';
            guest = scenario === 'guest';
            boardName = scenario === 'bms' ? 'ExtremelyLongBoardNameWithoutSpacesToStressHeaderTruncation' : longName;
            await page.waitForFunction(
              ({ bms, guest, title }) => {
                return (
                  document.querySelector('h1')?.textContent === title &&
                  !!document.querySelector('[aria-label="Board actions"]')?.classList.contains('text-warning') ===
                    guest &&
                  !!document.querySelector('[aria-label="Battery cells temperature"]') === bms
                );
              },
              { bms: withBms, guest, title: boardName }
            );
            await live.evaluate((element) => (element.scrollTop = 0));
            const metrics = await page.evaluate(() => {
              const rect = (selector: string) => {
                const el = document.querySelector(selector)!;
                const { x, y, width, height, bottom, right } = el.getBoundingClientRect();
                return { x, y, width, height, bottom, right };
              };
              const live = document.querySelector('[aria-label="Live board widgets"]')!;
              return {
                viewport: { width: innerWidth, height: innerHeight },
                horizontalOverflow: document.documentElement.scrollWidth > innerWidth,
                live: {
                  ...rect('[aria-label="Live board widgets"]'),
                  scrollHeight: live.scrollHeight,
                  clientHeight: live.clientHeight,
                },
                header: rect('main header'),
                numbers: rect('[aria-label="Live number widgets"]'),
                gauge: rect('[aria-labelledby="gauge-title gauge-description"]'),
                centre: rect('[aria-label^="Choose value for box 5"]'),
                batteryBar: rect('[aria-labelledby="power-title"] [role="progressbar"]'),
                battery: rect('[aria-labelledby="power-title"]'),
                temperatures: rect('[aria-labelledby="temperatures-title"]'),
                controls: [...document.querySelectorAll('main header button')]
                  .filter((el) => !el.closest('dialog'))
                  .map((el) => {
                    const { x, y, width, height, right, bottom } = el.getBoundingClientRect();
                    return { label: el.getAttribute('aria-label'), x, y, width, height, right, bottom };
                  }),
              };
            });
            const key = `${viewport.id}-${colorScheme}-${scenario}`;
            await page.screenshot({ path: path.join(output, engineName, `${key}.png`) });
            const issues: string[] = [];
            if (metrics.horizontalOverflow) issues.push('horizontal overflow');
            const shortFallback = viewport.viewport.height <= 320;
            if (!shortFallback && metrics.live.scrollHeight > metrics.live.clientHeight + 1)
              issues.push(`unexpected live scrolling (${metrics.live.scrollHeight}/${metrics.live.clientHeight}px)`);
            if (!shortFallback && metrics.battery.bottom > metrics.live.y + metrics.live.clientHeight + 1)
              issues.push('battery falls below visible live view');
            if (metrics.batteryBar.height < 20 || metrics.batteryBar.bottom + 20 > metrics.battery.bottom + 1)
              issues.push('battery bar clipped');
            if (metrics.centre.height < 44 || metrics.centre.bottom > metrics.gauge.bottom + 1)
              issues.push('centre value clipped or too small');
            const sections = [metrics.numbers, metrics.gauge, metrics.temperatures, metrics.battery];
            for (let i = 0; i < sections.length; i++)
              for (let j = i + 1; j < sections.length; j++) {
                const a = sections[i]!;
                const b = sections[j]!;
                if (
                  Math.min(a.right, b.right) - Math.max(a.x, b.x) > 1 &&
                  Math.min(a.bottom, b.bottom) - Math.max(a.y, b.y) > 1
                )
                  issues.push(`widgets overlap (${i}/${j})`);
              }
            // On exceptionally short screens, every reading must remain reachable by vertical scrolling.
            await page.locator('[aria-labelledby="power-title"]').scrollIntoViewIfNeeded();
            const batteryVisible = await page.locator('[aria-labelledby="power-title"]').evaluate((el) => {
              const box = el.getBoundingClientRect();
              const scroll = document.querySelector('[aria-label="Live board widgets"]')!.getBoundingClientRect();
              return box.top >= scroll.top - 1 && box.bottom <= scroll.bottom + 1;
            });
            if (!batteryVisible) issues.push('battery cannot be scrolled fully into view');
            if (metrics.live.scrollHeight > metrics.live.clientHeight + 1)
              await page.screenshot({ path: path.join(output, engineName, `${key}-scrolled.png`) });
            const centre = page.getByRole('button', { name: /^Choose value for box 5:/ });
            await centre.scrollIntoViewIfNeeded();
            const centreReachable = await centre.evaluate((el) => {
              const rect = el.getBoundingClientRect();
              const hit = document.elementFromPoint(rect.x + rect.width / 2, rect.y + rect.height / 2);
              return hit === el || (hit !== null && el.contains(hit));
            });
            if (!centreReachable) issues.push('centre value touch target is covered');
            await live.evaluate((element) => (element.scrollTop = 0));
            if (metrics.gauge.height < 160) issues.push(`gauge too small (${metrics.gauge.height}px)`);
            for (const control of metrics.controls) {
              if (
                control.width < 44 ||
                control.height < 44 ||
                control.x < 0 ||
                control.right > metrics.viewport.width ||
                control.bottom > metrics.viewport.height
              )
                issues.push(`header control squeezed/clipped: ${control.label}`);
            }
            if (scenario === 'saved') {
              await page.getByRole('button', { name: 'Ride location: off', exact: true }).tap();
              const dialog = page.getByRole('dialog', { name: 'Ride location', exact: true });
              await dialog.waitFor();
              const bounds = (await dialog.boundingBox())!;
              if (
                bounds.x < 0 ||
                bounds.y < 0 ||
                bounds.x + bounds.width > viewport.viewport.width + 1 ||
                bounds.y + bounds.height > viewport.viewport.height + 1
              )
                issues.push('location panel clipped');
              await dialog.getByRole('button', { name: 'Done' }).tap();
              await dialog.waitFor({ state: 'hidden' });
            }
            results.push({ key, label: viewport.label, viewport: viewport.viewport, metrics, issues });
            for (const issue of issues) failures.push(`${key}: ${issue}`);
          }
        }
      }
      // Exercise the compact marker target as well as its geometry.
      guest = false;
      withBms = false;
      await page.setViewportSize({ width: 320, height: 568 });
      const capture = page.getByRole('button', { name: 'Save ride marker', exact: true });
      await page.waitForFunction(
        () => document.querySelector<HTMLButtonElement>('[aria-label="Save ride marker"]')?.disabled === false
      );
      const active = (await client.request('state.get', {})).activeConnection!;
      const sessionId = active.activeSession!.sessionId;
      const before = await client.request('session.get', { sessionId });
      await capture.tap();
      await capture.tap();
      await page.getByRole('status').filter({ hasText: 'Marker saved' }).waitFor();
      assert.equal((await client.request('session.get', { sessionId })).markerCount, before.markerCount + 1);

      details = true;
      await page.getByRole('button', { name: 'Board actions', exact: true }).tap();
      await page.getByRole('button', { name: 'Detailed View', exact: true }).tap();
      const detailView = page.getByRole('region', { name: 'Detailed board telemetry', exact: true });
      await detailView.getByText('THOR300_ExtendedHardwareName_ForNarrowTelemetryCards', { exact: true }).waitFor();
      const detailResults: unknown[] = [];
      for (const colorScheme of ['light', 'dark'] as const) {
        await page.emulateMedia({ colorScheme });
        for (const viewport of liveViewports) {
          await page.setViewportSize(viewport.viewport);
          boardName =
            viewport.group === 'foldables' ? 'ExtremelyLongBoardNameWithoutSpacesToStressHeaderTruncation' : longName;
          await page.waitForFunction((title) => document.querySelector('h1')?.textContent === title, boardName);
          const scroller = detailView.locator('..');
          await scroller.evaluate((el) => (el.scrollTop = 0));
          const metrics = await detailView.evaluate((el) => {
            const rect = (element: Element) => {
              const { x, y, width, height, right, bottom } = element.getBoundingClientRect();
              return { x, y, width, height, right, bottom };
            };
            const cards = [...el.children].filter((node) => node.tagName === 'SECTION');
            const rows = [...el.querySelectorAll('dl > div')];
            return {
              horizontalOverflow:
                document.documentElement.scrollWidth > innerWidth || el.scrollWidth > el.clientWidth + 1,
              columns: new Set(
                [...el.querySelector('[aria-label="Board state"] dl')!.children].map((row) =>
                  Math.round(row.getBoundingClientRect().x)
                )
              ).size,
              cards: cards.map((card) => ({ label: card.getAttribute('aria-label'), ...rect(card) })),
              rows: rows.map((row) => {
                const label = row.querySelector('dt')!;
                const value = row.querySelector('dd')!;
                return {
                  label: label.textContent,
                  row: rect(row),
                  term: rect(label),
                  value: rect(value),
                  clipped: [row, label, value].some(
                    (node) => node.scrollWidth > node.clientWidth + 1 || node.scrollHeight > node.clientHeight + 1
                  ),
                };
              }),
            };
          });
          const issues: string[] = [];
          if (metrics.horizontalOverflow) issues.push('detailed view has horizontal overflow');
          const expected = expectedDetailColumns[viewport.id];
          if (expected !== undefined && metrics.columns !== expected)
            issues.push(`expected ${expected} columns, got ${metrics.columns}`);

          for (const row of metrics.rows) {
            if (row.row.width > 576 + 1) issues.push(`label/value pair too wide: ${row.label}`);
            if (
              row.clipped ||
              row.term.right > row.value.x + 1 ||
              row.value.right > row.row.right + 1 ||
              row.term.bottom > row.row.bottom + 1 ||
              row.value.bottom > row.row.bottom + 1
            )
              issues.push(`clipped/overlapping detail row: ${row.label}`);
          }
          const key = `${viewport.id}-${colorScheme}-detailed`;
          await page.screenshot({ path: path.join(output, engineName, `${key}.png`) });
          // The view deliberately scrolls. Inspect and capture its final row too.
          const last = detailView.locator('dd').last();
          await last.scrollIntoViewIfNeeded();
          const reachedEnd = await last.evaluate((el) => {
            const box = el.getBoundingClientRect();
            return box.y >= 0 && box.bottom <= innerHeight;
          });
          if (!reachedEnd) issues.push('last detailed value cannot be reached');
          await page.screenshot({ path: path.join(output, engineName, `${key}-end.png`) });
          detailResults.push({ key, label: viewport.label, viewport: viewport.viewport, metrics, issues });
          for (const issue of issues) failures.push(`${key}: ${issue}`);
        }
      }
      await writeFile(path.join(output, `${engineName}-detailed.json`), JSON.stringify(detailResults, null, 2));
      await writeFile(path.join(output, `${engineName}.json`), JSON.stringify(results, null, 2));
      console.log(
        `${engineName}: captured ${results.length} normal + ${detailResults.length} detailed renders in ${output}`
      );
      await writeLiveViewportGallery(browser, output, engineName);
      assert.deepEqual(failures, []);
    } finally {
      await browser.close();
      client.close();
      await stopMockServer();
    }
  });
}
