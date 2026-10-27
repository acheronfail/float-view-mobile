import assert from 'node:assert/strict';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';
import type { BatteryWarning } from '../../frontend/src/lib/backend/contract.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
const output = path.resolve(
  process.env.FLOAT_VIEW_BATTERY_PREVIEW_OUTPUT ?? new URL('../generated/battery-warnings', import.meta.url).pathname
);
const cases = [
  { name: 'Normal', warning: null, charging: false, voltage: 75.6, level: 0.64 },
  { name: 'Charging', warning: null, charging: true, voltage: 81.2, level: 0.89 },
  { name: 'Battery low', warning: 'low', charging: false, voltage: 60.1, level: 0.01 },
  { name: 'Over voltage', warning: 'overVoltage', charging: true, voltage: 85, level: 1 },
] satisfies { name: string; warning: BatteryWarning | null; charging: boolean; voltage: number; level: number }[];

for (const [engineName, engine] of Object.entries({ chromium, webkit })) {
  test(`${engineName}: battery warning presentation and preview`, { timeout: 60_000 }, async () => {
    const port = await startMockServer(0, { safetyDisclaimerAccepted: true });
    const endpoint = getMockBootstrap(port);
    const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
    const browser = await engine.launch();
    const errors: string[] = [];
    await mkdir(output, { recursive: true });
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
        viewport: { width: 390, height: 844 },
        reducedMotion: 'no-preference',
        colorScheme: 'light',
      });
      page.setDefaultTimeout(5000);
      if (engineName === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      page.on('pageerror', (error) => errors.push(error.message));
      let scenario = cases[0]!;
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
            active.batteryWarning = scenario.warning;
            active.batteryEstimate = {
              ...active.batteryEstimate,
              level: scenario.level,
              seriesCount: 20,
              unavailableReason: null,
            };
            Object.assign(active.latestSample, {
              packVoltageV: scenario.voltage,
              charging: scenario.charging,
              firmwareFault: null,
              fatalErrorLatched: false,
            });
            state.settings.announcements.batteryChargeEnabled = false;
          }
          socket.send(JSON.stringify(message));
        });
      });
      await page.goto(`${endpoint.origin}/#/live`);
      const widget = page.locator('section[aria-labelledby="power-title"]');
      await widget.waitFor();
      const settleBars = (selector: string) =>
        page.waitForFunction(
          (selector) =>
            [...document.querySelectorAll(selector)].every((node) => {
              const target = Number(node.getAttribute('aria-valuenow'));
              const fill = node.lastElementChild!;
              const percent = (fill.getBoundingClientRect().width / node.getBoundingClientRect().width) * 100;
              return Math.abs(percent - target) < 0.1;
            }),
          selector
        );
      const cards: string[] = [];
      for (const entry of cases) {
        scenario = entry;
        await page.waitForFunction(
          ({ text, value }) => {
            const widget = document.querySelector('section[aria-labelledby="power-title"]');
            return (
              widget?.textContent?.includes(text) &&
              widget.querySelector('[role="progressbar"]')?.getAttribute('aria-valuenow') === String(value)
            );
          },
          { text: entry.name === 'Normal' ? 'Battery' : entry.name, value: Math.round(entry.level * 100) }
        );
        const track = widget.getByRole('progressbar');
        assert.equal(await track.getAttribute('aria-valuenow'), String(Math.round(entry.level * 100)));
        if (entry.warning) {
          assert.equal((await widget.locator('[role="status"]').textContent())?.trim(), entry.name);
          assert.equal(await widget.getByText('Charging', { exact: true }).count(), 0);
          const style = await track.evaluate((node) => ({
            duration: getComputedStyle(node, '::after').animationDuration,
            shadow: getComputedStyle(node, '::after').boxShadow,
            hatch: getComputedStyle(node, '::after').backgroundImage,
          }));
          assert.equal(style.duration, '3s');
          assert.notEqual(style.shadow, 'none');
          assert.equal(style.hatch.includes('repeating-linear-gradient'), entry.warning === 'overVoltage');
          await page.emulateMedia({ reducedMotion: 'reduce' });
          assert.equal(await track.evaluate((node) => getComputedStyle(node, '::after').animationName), 'none');
          await page.emulateMedia({ reducedMotion: 'no-preference' });
        }
        await settleBars('section[aria-labelledby="power-title"] [role="progressbar"]');
        await widget.screenshot({
          path: path.join(output, `${engineName}-${entry.name.replaceAll(' ', '-').toLowerCase()}.png`),
          animations: 'allow',
        });
        const markup = await widget.evaluate((node) => node.outerHTML);
        cards.push(
          `<article class="preview-card"><h1>${entry.name}</h1><div class="preview-widget">${markup.replaceAll('power-title', `power-${cards.length}`)}</div></article>`
        );
      }
      // Restore ordinary state after a warning; no stale label or hatching may remain.
      scenario = cases[0]!;
      await page.waitForFunction(
        () => !document.querySelector('section[aria-labelledby="power-title"]')?.classList.contains('battery-warning')
      );
      assert.equal(await widget.locator('[role="status"]').count(), 0);
      // Exercise the same authoritative warning in each custom battery card size.
      const ready = await client.request('state.get', {});
      const savedDeviceId = ready.activeConnection!.savedDeviceId!;
      const profile = await client.request('boardProfile.get', { savedDeviceId });
      await client.request('boardProfile.update', {
        savedDeviceId,
        expectedRevision: profile.revision,
        patch: {
          customDashboard: {
            version: 1,
            widgets: ['compact', 'medium', 'large'].map((size) => ({
              id: `battery-${size}`,
              kind: 'visual' as const,
              visual: 'battery' as const,
              size: size as 'compact' | 'medium' | 'large',
            })),
          },
        },
      });
      await page.getByRole('button', { name: 'Board actions', exact: true }).click();
      await page.getByRole('button', { name: 'Custom View', exact: true }).click();
      const custom = page.getByRole('region', { name: 'Custom dashboard', exact: true });
      for (const entry of cases.slice(2)) {
        scenario = entry;
        await page.waitForFunction(
          (label) =>
            [...document.querySelectorAll('.dashboard-power .battery-alert-label')].filter(
              (node) => node.textContent?.trim() === label
            ).length === 3,
          entry.name
        );
        for (const size of ['compact', 'medium', 'large']) {
          const card = custom.locator(`[data-dashboard-widget="battery-${size}"]`);
          assert.equal(
            await card.getByRole('progressbar').getAttribute('aria-valuenow'),
            String(Math.round(entry.level * 100))
          );
          assert.equal(
            await card.locator('.battery-alert-label').evaluate((node) => node.scrollWidth <= node.clientWidth),
            true,
            `${size} warning label fits`
          );
        }
      }
      await settleBars('.dashboard-power [role="progressbar"]');
      await custom.screenshot({ path: path.join(output, `${engineName}-custom.png`) });
      await page.emulateMedia({ colorScheme: 'dark', reducedMotion: 'reduce' });
      assert.equal(
        await custom
          .locator('[role="progressbar"]')
          .first()
          .evaluate((node) => getComputedStyle(node, '::after').animationName),
        'none'
      );
      await settleBars('.dashboard-power [role="progressbar"]');
      await custom.screenshot({ path: path.join(output, `${engineName}-custom-dark.png`) });
      await page.emulateMedia({ colorScheme: 'light', reducedMotion: 'no-preference' });
      assert.deepEqual(errors, []);
      if (engineName === 'chromium') {
        const styles = await page.locator('style').allTextContents();
        const preview = `<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>Battery widget previews</title><style>${styles.join('\n')}</style><style>body{margin:0;padding:32px;background:var(--color-background);color:var(--color-foreground)}.preview-heading{font:600 24px system-ui;margin:0 0 8px}.preview-intro{font:14px system-ui;color:var(--color-muted);margin:0 0 28px}.preview-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:20px;max-width:1080px}.preview-card{border:1px solid var(--color-border);border-radius:16px;padding:24px;background:var(--color-surface)}.preview-card>h1{font:600 15px system-ui;margin:0 0 24px}.preview-widget{height:108px}.preview-widget>section{height:100%}@media(max-width:700px){body{padding:20px}.preview-grid{grid-template-columns:1fr}}</style></head><body><h1 class="preview-heading">Live battery widget</h1><p class="preview-intro">Actual app widgets · warnings remain visible with announcements off · reduced motion supported</p><main class="preview-grid">${cards.join('')}</main></body></html>`;
        await writeFile(path.join(output, 'index.html'), preview);
        await page.goto(`file://${path.join(output, 'index.html')}`);
        await page.setViewportSize({ width: 1080, height: 560 });
        await page.screenshot({ path: path.join(output, 'preview.png'), fullPage: true, animations: 'allow' });
      }
    } finally {
      client.close();
      await browser.close();
      await stopMockServer();
    }
  });
}
