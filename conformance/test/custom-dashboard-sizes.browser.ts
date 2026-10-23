import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit, type Locator } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { liveWidgetFields, type DashboardWidget } from '../../frontend/src/lib/backend/contract.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: widget size and title audit`, { timeout: 240_000 }, async () => {
    const browser = await engine.launch();
    const port = await startMockServer(0, { safetyDisclaimerAccepted: true });
    const endpoint = getMockBootstrap(port);
    const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
    try {
      await client.request('bluetooth.requestPermission', {});
      await client.request('bluetooth.scan', { timeoutMs: 100 });
      const scanned = await client.waitForState((s) => s.discoveredDevices.length > 0);
      await client.request('device.connect', {
        deviceId: scanned.discoveredDevices[0]!.deviceId,
        trackingMode: 'saved',
      });
      const ready = await client.waitForState(
        (s) => s.activeConnection?.state === 'ready' && !!s.activeConnection.latestSample
      );
      const id = ready.activeConnection!.savedDeviceId!;
      const page = await browser.newPage({ viewport: { width: 390, height: 844 }, reducedMotion: 'reduce' });
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.route(`${endpoint.origin}/api/bootstrap`, (route) =>
        route.fulfill({ json: { webSocketUrl: endpoint.webSocketUrl, platform: 'web' } })
      );
      let withBms = false;
      await page.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        socket.onMessage((raw) => server.send(raw));
        server.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          const state =
            message.event === 'state.changed' ? message.data : message.result?.activeConnection ? message.result : null;
          if (state?.activeConnection?.latestSample) state.activeConnection.latestSample.charging = withBms;
          if (withBms && state?.activeConnection)
            state.activeConnection.bms = {
              ...state.activeConnection.bms,
              status: 'attached',
              connection: { kind: 'can', canId: 42 },
              candidates: [],
              latestSnapshot: null,
            };
          socket.send(JSON.stringify(message));
        });
      });
      const custom = page.getByRole('region', { name: 'Custom dashboard', exact: true });
      async function show(widgets: DashboardWidget[]) {
        const profile = await client.request('boardProfile.get', { savedDeviceId: id });
        await client.request('boardProfile.update', {
          savedDeviceId: id,
          expectedRevision: profile.revision,
          patch: { customDashboard: { version: 1, widgets }, lastLiveView: 'custom' },
        });
        if (page.url() === 'about:blank') await page.goto(`${endpoint.origin}/#/live`);
        else await page.reload();
        await custom.locator('[data-dashboard-widget]').last().waitFor();
      }
      const auditFailures: string[] = [];
      async function audit(context: string, scope: Locator = custom.locator('[data-dashboard-widget]')) {
        const failures = await scope.evaluateAll((cards) =>
          cards.flatMap((card) => {
            const root = card.querySelector<HTMLElement>('.custom-widget-content')!;
            const bounds = root.getBoundingClientRect();
            const problems: string[] = [];
            if (root.scrollHeight > root.clientHeight + 1 || root.scrollWidth > root.clientWidth + 1)
              problems.push(
                `content overflow ${root.scrollWidth}x${root.scrollHeight}/${root.clientWidth}x${root.clientHeight}`
              );
            for (const el of root.querySelectorAll<HTMLElement>(
              'h2,h3,[role="progressbar"],.power-value,.pack-voltage,.temperature-label,.chart-y-axis span,.chart-time-axis span'
            )) {
              const r = el.getBoundingClientRect();
              if (!r.width || !r.height) continue;
              if (
                r.left < bounds.left - 1 ||
                r.right > bounds.right + 1 ||
                r.top < bounds.top - 1 ||
                r.bottom > bounds.bottom + 1 ||
                el.scrollWidth > el.clientWidth + 1
              )
                problems.push(`${el.tagName} ${el.textContent?.trim()} clipped`);
            }
            return problems.map((problem) => `${card.getAttribute('data-dashboard-widget')}: ${problem}`);
          })
        );
        auditFailures.push(...failures.map((failure) => `${context}: ${failure}`));
      }
      const base: DashboardWidget[] = [
        ...(['battery', 'temperatures', 'footpads', 'speedGauge'] as const).map((visual): DashboardWidget => ({
          id: visual,
          kind: 'visual',
          visual,
          size: 'compact',
        })),
        ...(['headlights', 'leds'] as const).map((control): DashboardWidget => ({
          id: control,
          kind: 'control',
          control,
          size: 'compact',
        })),
        ...(['number', 'bar', 'gauge', 'trend'] as const).map((presentation): DashboardWidget => ({
          id: presentation,
          kind: 'reading',
          field: presentation === 'bar' || presentation === 'gauge' ? 'bmsTotalVoltageV' : 'bmsCellVoltageSpreadV',
          presentation,
          size: 'compact',
        })),
      ];
      let compactMetrics: Record<string, number> | undefined;
      for (const width of [280, 390, 844, 1280, 2560]) {
        await page.setViewportSize({ width, height: width === 844 ? 390 : 900 });
        for (const bms of [false, true]) {
          withBms = bms;
          for (let columns = 1; columns <= 4; columns++)
            for (let rows = 1; rows <= 4; rows++) {
              await show(base.map((widget) => ({ ...widget, size: { columns, rows } })));
              await audit(`${width}px, BMS ${bms}, ${columns}x${rows}`);
              if (width === 390 && !bms && columns === 4 && rows <= 2) {
                const metrics = await custom.evaluate((root) => {
                  const height = (selector: string) => root.querySelector(selector)!.getBoundingClientRect().height;
                  const font = (selector: string) =>
                    parseFloat(getComputedStyle(root.querySelector(selector)!).fontSize);
                  return {
                    temperatureBar: height('[data-dashboard-widget="temperatures"] [role="progressbar"]'),
                    temperatureReading: font('[data-dashboard-widget="temperatures"] .temperature-label'),
                    batteryBar: height('[data-dashboard-widget="battery"] [role="progressbar"]'),
                    batteryReading: font('[data-dashboard-widget="battery"] .power-value'),
                    control: height('[data-dashboard-widget="headlights"] .lighting-switch'),
                    footpads: height('[data-dashboard-widget="footpads"] .footpad-bar'),
                    number: font('[data-dashboard-widget="number"] .font-mono'),
                    barReading: font('[data-dashboard-widget="bar"] .font-mono'),
                    gauge: height('[data-dashboard-widget="gauge"] svg'),
                    speedGauge: height('[data-dashboard-widget="speedGauge"] svg'),
                    chart: height('[data-dashboard-widget="trend"] .telemetry-chart-frame'),
                  };
                });
                if (rows === 2) {
                  const chartLayout = await custom.locator('[data-dashboard-widget="trend"]').evaluate((card) => {
                    const y = card.querySelector<HTMLElement>('.chart-y-axis')!;
                    const x = card.querySelector<HTMLElement>('.chart-time-axis')!;
                    const plot = card.querySelector('.telemetry-chart-frame')!.getBoundingClientRect();
                    return {
                      labelSize: parseFloat(getComputedStyle(y).fontSize),
                      outsidePlot:
                        y.getBoundingClientRect().right <= plot.left + 1 &&
                        x.getBoundingClientRect().top >= plot.bottom - 1,
                      trace: parseFloat(getComputedStyle(card.querySelector('.telemetry-chart-trace')!).strokeWidth),
                    };
                  });
                  assert.ok(
                    chartLayout.labelSize >= 10 && chartLayout.outsidePlot && chartLayout.trace >= 2,
                    'chart labels remain readable outside a stronger trace'
                  );
                }
                if (rows === 1) compactMetrics = metrics;
                else
                  for (const [part, value] of Object.entries(metrics))
                    assert.ok(
                      value > compactMetrics![part]! * 1.2,
                      `${part} should visibly grow from 4x1 to 4x2: ${compactMetrics![part]} -> ${value}`
                    );
              }

              if (columns === 2 && rows === 2 && bms) {
                const output = new URL(`../generated/custom-dashboard-sizes/${name}/`, import.meta.url);
                await mkdir(output, { recursive: true });
                await page.screenshot({ path: new URL(`${width}.png`, output).pathname });
              }
            }
        }
      }
      const fields = liveWidgetFields.filter((field) => !['empty', 'headlights', 'leds'].includes(field));
      await page.setViewportSize({ width: 390, height: 900 });
      for (let offset = 0; offset < fields.length; offset += 24) {
        await show(
          fields.slice(offset, offset + 24).map((field): DashboardWidget => ({
            id: field,
            kind: 'reading',
            field: field as Extract<DashboardWidget, { kind: 'reading' }>['field'],
            presentation: 'number',
            size: { columns: 1, rows: 1 },
          }))
        );
        await audit(`all value labels ${offset}`);
      }
      withBms = false;
      await show(base);
      const customStyles = await custom.locator('.live-widget-title').evaluateAll((els) =>
        els.map((el) => {
          const s = getComputedStyle(el);
          return {
            color: s.color,
            size: s.fontSize,
            weight: s.fontWeight,
            tracking: s.letterSpacing,
            align: s.textAlign,
            casing: s.textTransform,
          };
        })
      );
      const numberStyle = await custom
        .locator('[data-dashboard-widget="number"] .font-mono')
        .evaluate((el) => ({ size: getComputedStyle(el).fontSize, align: getComputedStyle(el).textAlign }));
      assert.deepEqual(numberStyle, { size: '14px', align: 'center' }, 'compact number matches normal Live typography');
      await custom.getByRole('button', { name: 'Arrange Battery', exact: true }).press('Enter');
      for (const panel of ['Battery', 'Temperatures']) {
        await custom.getByRole('button', { name: new RegExp(`^Configure ${panel},`) }).click();
        assert.equal(await page.getByRole('combobox', { name: 'Width', exact: true }).count(), 0);
        for (const [columns, rows] of [
          [1, 1],
          [2, 1],
          [4, 1],
          [1, 2],
          [2, 2],
          [4, 2],
        ]) {
          await page
            .getByRole('button', {
              name: `${columns} ${columns === 1 ? 'column' : 'columns'} by ${rows} ${rows === 1 ? 'row' : 'rows'}`,
              exact: true,
            })
            .click();
          await audit(`${panel} preview ${columns}x${rows}`, page.getByLabel('Widget preview', { exact: true }));
        }
        await page.getByRole('button', { name: 'Cancel', exact: true }).click();
      }
      await custom.getByRole('button', { name: 'Cancel', exact: true }).click();
      await show(
        base.map((widget) => (widget.id === 'battery' ? { ...widget, size: { columns: 1, rows: 4 } } : widget))
      );
      await custom.getByRole('button', { name: 'Arrange Battery', exact: true }).press('Enter');
      await custom.getByRole('button', { name: /^Configure Battery,/ }).click();
      assert.equal(
        await page.getByRole('button', { name: '1 column by 4 rows', exact: true }).getAttribute('aria-pressed'),
        'true',
        'existing non-preset dimensions remain explicit'
      );
      await page.getByRole('button', { name: 'Cancel', exact: true }).click();
      await custom.getByRole('button', { name: 'Cancel', exact: true }).click();
      await page.getByRole('button', { name: 'Board actions', exact: true }).click();
      await page.getByRole('button', { name: 'Live View', exact: true }).click();
      const normalTitle = page
        .getByRole('region', { name: 'Live board widgets', exact: true })
        .locator('.live-widget-title')
        .first();
      await normalTitle.waitFor({ timeout: 5000 }).catch(async (error) => {
        console.error(await page.locator('body').innerText());
        throw error;
      });
      const titleStyle = await normalTitle.evaluate((el) => {
        const s = getComputedStyle(el);
        return {
          color: s.color,
          size: s.fontSize,
          weight: s.fontWeight,
          tracking: s.letterSpacing,
          align: s.textAlign,
          casing: s.textTransform,
        };
      });
      for (const style of customStyles)
        assert.deepEqual(style, titleStyle, 'all custom titles share the normal Live box styling');
      await page.setViewportSize({ width: 1600, height: 1000 });
      assert.ok(
        await page.locator('.app-content').evaluate((el) => el.getBoundingClientRect().width <= 1024),
        'normal Live retains its existing desktop width cap'
      );
      assert.deepEqual(auditFailures, []);
    } finally {
      client.close();
      await browser.close();
      await stopMockServer();
    }
  });
}
