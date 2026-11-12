import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit, type Locator } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: representative preview taps, holds and isolated example actions`, { timeout: 90_000 }, async () => {
    const browser = await engine.launch();
    const endpoint = getMockBootstrap(await startMockServer(0, { safetyDisclaimerAccepted: true }));
    const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
    try {
      const id = (await client.request('state.get', {})).savedDevices[0]!.savedDeviceId;
      const profile = await client.request('boardProfile.get', { savedDeviceId: id });
      await client.request('boardProfile.update', {
        savedDeviceId: id,
        expectedRevision: profile.revision,
        patch: {
          lastLiveView: 'custom',
          liveWidgetSlots: ['headlights', 'leds', 'themeToggle', 'distanceM', 'motorCurrentA'],
          customDashboard: {
            version: 1,
            widgets: [
              { id: 'headlights', kind: 'control', control: 'headlights', size: 'compact' },
              { id: 'duplicate', kind: 'control', control: 'headlights', size: 'compact' },
              { id: 'leds', kind: 'control', control: 'leds', size: 'compact' },
              { id: 'theme', kind: 'action', action: 'themeToggle', size: 'compact' },
              { id: 'reading', kind: 'reading', field: 'motorCurrentA', presentation: 'number', size: 'compact' },
              { id: 'marker', kind: 'action', action: 'rideMarker', size: 'compact' },
            ],
          },
        },
      });
      await client.request('bluetooth.requestPermission', {});
      await client.request('bluetooth.scan', { timeoutMs: 100 });
      await client.request('device.connect', { deviceId: 'mock-board', trackingMode: 'saved' });
      await client.waitForState((s) => s.activeConnection?.state === 'ready');
      const page = await browser.newPage({
        viewport: { width: 390, height: 844 },
        hasTouch: true,
        colorScheme: 'light',
        reducedMotion: 'reduce',
      });
      page.setDefaultTimeout(7000);
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      const requests: { method: string; params: Record<string, unknown> }[] = [];
      const errors: string[] = [];
      page.on('pageerror', (e) => errors.push(e.message));
      await page.route(`${endpoint.origin}/`, (r) => r.fulfill({ contentType: 'text/html', body: html }));
      await page.route(`${endpoint.origin}/api/bootstrap`, (r) =>
        r.fulfill({ json: { webSocketUrl: endpoint.webSocketUrl, platform: 'web' } })
      );
      await page.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        socket.onMessage((raw) => {
          requests.push(JSON.parse(String(raw)));
          server.send(raw);
        });
        server.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          const state =
            message.event === 'state.changed' ? message.data : message.result?.savedDevices ? message.result : null;
          if (state?.activeConnection) state.activeConnection.savedDeviceId = 'another-board';
          socket.send(JSON.stringify(message));
        });
      });
      const custom = page.getByRole('region', { name: 'Custom dashboard', exact: true });
      const customLights = custom.getByRole('switch', { name: 'Headlights', exact: true });
      const theme = page.getByRole('button', { name: 'Light / dark mode', exact: true });
      const toolbar = custom.getByRole('toolbar', { name: 'Edit dashboard', exact: true });
      const themeWrites = () => requests.filter((r) => r.method === 'settings.update').length;
      async function switchView(label: 'Live View' | 'Custom View') {
        await page.getByRole('button', { name: 'View options', exact: true }).tap();
        await page.getByRole('button', { name: `Switch to ${label}`, exact: true }).tap();
      }
      async function hold(target: Locator) {
        await target.scrollIntoViewIfNeeded();
        const box = (await target.boundingBox())!;
        await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
        await page.mouse.down();
        await page.waitForTimeout(650);
        await page.mouse.up();
      }
      async function invalidPointers(target: Locator) {
        await target.evaluate((node) => {
          const owner = node.closest('article') ?? node;
          const capture = owner.setPointerCapture;
          owner.setPointerCapture = () => {};
          const pointer = { bubbles: true, pointerId: 77, pointerType: 'touch', button: 0, clientX: 30, clientY: 30 };
          try {
            node.dispatchEvent(new PointerEvent('pointerdown', { ...pointer, isPrimary: false }));
            node.dispatchEvent(new PointerEvent('pointerup', { ...pointer, isPrimary: false }));
            node.dispatchEvent(new PointerEvent('pointerdown', { ...pointer, isPrimary: true }));
            node.dispatchEvent(new PointerEvent('pointercancel', { ...pointer, isPrimary: true }));
            node.dispatchEvent(new PointerEvent('pointerup', { ...pointer, isPrimary: true }));
            node.dispatchEvent(new PointerEvent('pointerdown', { ...pointer, isPrimary: true }));
            node.dispatchEvent(new PointerEvent('pointermove', { ...pointer, isPrimary: true, clientY: 80 }));
            node.dispatchEvent(new PointerEvent('pointerup', { ...pointer, isPrimary: true, clientY: 80 }));
          } finally {
            owner.setPointerCapture = capture;
          }
        });
      }
      async function back() {
        await page.getByRole('button', { name: 'View options', exact: true }).tap();
        await page.getByRole('button', { name: 'Back', exact: true }).tap();
      }
      await page.goto(`${endpoint.origin}/#/sessions/boards/${id}/views`);
      await customLights.first().waitFor();
      assert.equal(await toolbar.count(), 0, 'Custom starts in viewing mode');
      assert.equal(await customLights.first().getAttribute('aria-checked'), 'false');
      await customLights.first().tap();
      assert.deepEqual(await customLights.evaluateAll((nodes) => nodes.map((n) => n.getAttribute('aria-checked'))), [
        'true',
        'true',
      ]);
      assert.equal(await toolbar.count(), 0, 'a light tap does not enter editing');
      await customLights.last().focus();
      await page.keyboard.press('Enter');
      assert.deepEqual(await customLights.evaluateAll((nodes) => nodes.map((n) => n.getAttribute('aria-checked'))), [
        'false',
        'false',
      ]);
      await customLights.last().tap();
      await custom.locator('[data-dashboard-widget="reading"]').tap();
      assert.equal(await toolbar.count(), 0, 'a reading tap does not enter editing');
      await invalidPointers(customLights.first());
      assert.equal(await customLights.first().getAttribute('aria-checked'), 'true');
      assert.equal(await toolbar.count(), 0);
      const beforeTheme = themeWrites();
      await theme.tap();
      await page.waitForFunction(() => document.documentElement.dataset.theme === 'dark');
      assert.equal(themeWrites(), beforeTheme + 1, 'one tap changes the real app theme once');
      await switchView('Live View');
      const live = page.getByRole('region', { name: 'Live board widgets', exact: true });
      const headlights = live.getByRole('switch', { name: 'Headlights', exact: true });
      const leds = live.getByRole('switch', { name: 'LEDs', exact: true });
      await headlights.waitFor();
      assert.equal(await headlights.getAttribute('aria-checked'), 'true', 'lighting examples survive a view switch');
      await headlights.tap();
      await leds.tap();
      assert.equal(await headlights.getAttribute('aria-checked'), 'false');
      assert.equal(await leds.getAttribute('aria-checked'), 'false');
      const distance = live.getByRole('button', { name: /^Box 4:/ });
      await distance.tap();
      assert.match((await distance.getAttribute('aria-label'))!, /Odometer/);
      assert.equal(
        (await client.request('boardProfile.get', { savedDeviceId: id })).values.liveWidgetSlots?.[3],
        'distanceM'
      );
      await distance.focus();
      await page.keyboard.press('Shift+Enter');
      const distancePicker = page.getByRole('dialog', { name: 'Choose a value', exact: true });
      assert.equal(
        await distancePicker.getByRole('button', { name: 'Odometer', exact: true }).getAttribute('aria-pressed'),
        'true'
      );
      await distancePicker.getByRole('button', { name: 'Ride distance', exact: true }).tap();
      assert.match((await distance.getAttribute('aria-label'))!, /Ride distance/);
      await live.getByRole('button', { name: /^Choose value for box 5:/ }).tap();
      assert.equal(
        await page.getByRole('dialog', { name: 'Choose a value' }).count(),
        0,
        'numeric short tap does not edit'
      );
      await invalidPointers(headlights);
      assert.equal(await headlights.getAttribute('aria-checked'), 'false');
      await hold(headlights);
      const picker = page.getByRole('dialog', { name: 'Choose a value', exact: true });
      await picker.waitFor();
      await picker.getByRole('button', { name: 'Cancel', exact: true }).tap();
      assert.equal(await headlights.getAttribute('aria-checked'), 'false', 'holding configures without toggling');
      await hold(theme);
      await picker.waitFor();
      await picker.getByRole('button', { name: 'Cancel', exact: true }).tap();
      assert.equal(themeWrites(), beforeTheme + 1, 'holding theme does not activate it');
      await theme.focus();
      await page.keyboard.press('Enter');
      await page.waitForFunction(() => document.documentElement.dataset.theme === 'light');
      assert.equal(themeWrites(), beforeTheme + 2);
      await switchView('Custom View');
      await customLights.first().waitFor();
      assert.equal(await customLights.first().getAttribute('aria-checked'), 'false');
      assert.equal(
        await custom.getByRole('switch', { name: 'LEDs', exact: true }).getAttribute('aria-checked'),
        'false'
      );
      assert.equal(await custom.getByRole('button', { name: 'Save ride marker', exact: true }).isDisabled(), true);
      await invalidPointers(theme);
      await hold(theme);
      await toolbar.waitFor();
      assert.equal(themeWrites(), beforeTheme + 2);
      await custom.getByRole('button', { name: /^Configure Light \/ dark mode,/ }).tap();
      const configuration = page.getByRole('dialog', { name: 'Configure widget', exact: true });
      await configuration.waitFor();
      assert.equal(
        await configuration.getByLabel('Widget preview', { exact: true }).getByRole('button').isDisabled(),
        true
      );
      await configuration.getByRole('button', { name: 'Cancel', exact: true }).tap();
      await toolbar.getByRole('button', { name: 'Cancel', exact: true }).tap();
      await toolbar.waitFor({ state: 'detached' });
      await customLights.first().waitFor();
      assert.equal(await toolbar.count(), 0, 'clean Cancel returns to viewing without leaving the preview');
      assert.equal(themeWrites(), beforeTheme + 2);
      await hold(theme);
      await toolbar.waitFor();
      const source = (await custom.locator('[data-dashboard-widget="theme"]').boundingBox())!;
      const target = (await custom.locator('[data-dashboard-widget="headlights"]').boundingBox())!;
      await page.mouse.move(source.x + source.width / 2, source.y + source.height / 2);
      await page.mouse.down();
      await page.waitForTimeout(350);
      await page.mouse.move(target.x + target.width / 2, target.y + target.height / 2, { steps: 8 });
      await page.mouse.up();
      assert.equal(themeWrites(), beforeTheme + 2, 'dragging theme never activates it');
      assert.equal(
        await custom.locator('[data-dashboard-widget]').first().getAttribute('data-dashboard-widget'),
        'theme'
      );
      await toolbar.getByRole('button', { name: 'Cancel', exact: true }).tap();
      await page.getByRole('button', { name: 'Discard changes', exact: true }).tap();
      await toolbar.waitFor({ state: 'detached' });
      await hold(customLights.first());
      await toolbar.waitFor();
      await custom.getByRole('button', { name: 'Remove Actual motor current', exact: true }).tap();
      await toolbar.getByRole('button', { name: 'Cancel', exact: true }).tap();
      await page.getByRole('button', { name: 'Discard changes', exact: true }).tap();
      await customLights.first().waitFor();
      assert.equal(await custom.locator('[data-dashboard-widget="reading"]').count(), 1);
      await hold(customLights.first());
      await toolbar.waitFor();
      await custom.getByRole('button', { name: 'Remove Actual motor current', exact: true }).tap();
      await toolbar.getByRole('button', { name: 'Save dashboard', exact: true }).tap();
      await toolbar.waitFor({ state: 'detached' });
      await customLights.first().waitFor();
      assert.equal(await toolbar.count(), 0, 'Save returns to viewing without leaving the preview');
      assert.equal(await custom.locator('[data-dashboard-widget="reading"]').count(), 0);
      await customLights.first().tap();
      assert.equal(await customLights.first().getAttribute('aria-checked'), 'true', 'controls resume after saving');
      await back();
      await page.getByRole('button', { name: 'Board actions', exact: true }).tap();
      await page.getByRole('button', { name: 'Customise views', exact: true }).tap();
      await customLights.first().waitFor();
      assert.equal(
        await customLights.first().getAttribute('aria-checked'),
        'false',
        'example state resets when reopening'
      );
      assert.equal(
        await custom.getByRole('switch', { name: 'LEDs', exact: true }).getAttribute('aria-checked'),
        'true'
      );
      assert.equal((await client.request('boardProfile.get', { savedDeviceId: id })).values.lastLiveView, 'custom');
      assert.equal(
        requests.some(
          (r) => r.method.startsWith('refloat.') || ['device.connect', 'session.marker.add'].includes(r.method)
        ),
        false
      );
      for (const request of requests.filter((r) => r.method === 'settings.update'))
        assert.deepEqual(Object.keys(request.params.patch as object), ['themePreference']);
      assert.deepEqual(errors, []);
    } finally {
      client.close();
      await browser.close();
      await stopMockServer();
    }
  });
}
