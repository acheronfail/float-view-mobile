import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';
import { defaultBoardAppearance, type CustomDashboard } from '../../frontend/src/lib/backend/contract.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: board-scoped offline view editing, previews and guarded saves`, { timeout: 120_000 }, async () => {
    const browser = await engine.launch();
    const endpoint = getMockBootstrap(await startMockServer(0, { safetyDisclaimerAccepted: true }));
    const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
    try {
      const initial = await client.request('state.get', {});
      const id = initial.savedDevices[0]!.savedDeviceId;
      await client.request('savedDevice.update', {
        savedDeviceId: id,
        expectedRevision: 0,
        customDisplayName: 'Trail Blazer',
      });
      let profile = await client.request('boardProfile.get', { savedDeviceId: id });
      const dashboard: CustomDashboard = {
        version: 1,
        widgets: [
          { id: 'speed', kind: 'visual', visual: 'speedGauge', size: { columns: 4, rows: 3 } },
          { id: 'battery', kind: 'visual', visual: 'battery', size: 'medium' },
          { id: 'temperatures', kind: 'visual', visual: 'temperatures', size: 'medium' },
          {
            id: 'current',
            kind: 'reading',
            field: 'motorCurrentA',
            presentation: 'trend',
            size: { columns: 4, rows: 2 },
          },
          { id: 'lights', kind: 'control', control: 'headlights', size: 'compact' },
          { id: 'theme', kind: 'action', action: 'themeToggle', size: 'compact' },
          { id: 'marker', kind: 'action', action: 'rideMarker', size: { columns: 2, rows: 1 } },
        ],
      };
      await client.request('boardProfile.update', {
        savedDeviceId: id,
        expectedRevision: profile.revision,
        patch: {
          lastLiveView: 'custom',
          customDashboard: dashboard,
          appearance: { ...defaultBoardAppearance, rail: 3, fender: 2, leftTop: '#ed5555', rightTop: '#ed5555' },
        },
      });
      const page = await browser.newPage({
        viewport: { width: 390, height: 844 },
        hasTouch: true,
        reducedMotion: 'reduce',
      });
      page.setDefaultTimeout(7000);
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      const errors: string[] = [];
      const requests: { method: string; params: Record<string, any> }[] = [];
      let failLoad = false;
      let failSave: 'unavailable' | 'conflict' | null = null;
      let otherBoard = false;
      let healthyLive = false;
      page.on('pageerror', (e) => errors.push(e.message));
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.route(`${endpoint.origin}/api/bootstrap`, (route) =>
        route.fulfill({ json: { webSocketUrl: endpoint.webSocketUrl, platform: 'web' } })
      );
      await page.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        socket.onMessage((raw) => {
          const request = JSON.parse(String(raw));
          requests.push(request);
          if (
            (failLoad && request.method === 'boardProfile.get') ||
            (failSave && request.method === 'boardProfile.update')
          ) {
            socket.send(
              JSON.stringify({
                id: request.id,
                error: {
                  code: failSave ?? 'unavailable',
                  message: failSave === 'conflict' ? 'Profile changed' : 'Storage unavailable',
                },
              })
            );
            failLoad = false;
            failSave = null;
          } else server.send(raw);
        });
        server.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          const state =
            message.event === 'state.changed' ? message.data : message.result?.savedDevices ? message.result : null;
          // Exercise an imported board with no platform association, even during another connection.
          if (state) {
            state.savedDevices.forEach((b: any) => {
              if (b.savedDeviceId === id) b.bluetoothIdentifier = null;
            });
            if (healthyLive && state.activeConnection) {
              // Compare layout with healthy Live; a real safety banner intentionally reserves space.
              const active = state.activeConnection;
              active.connectionWarning = null;
              if (active.latestSample)
                Object.assign(active.latestSample, {
                  fatalErrorLatched: false,
                  firmwareFault: null,
                  imuTimeoutActive: false,
                  lastFirmwareFault: null,
                  faultHistory: [],
                });
              if (active.refloatControls)
                Object.assign(active.refloatControls, {
                  fatalErrorLatched: false,
                  packageEnabled: true,
                });
            }
            if (otherBoard && state.activeConnection) {
              state.activeConnection.savedDeviceId = 'another-board';
              state.activeConnection.displayName = 'Other board';
              if (state.activeConnection.latestSample) state.activeConnection.latestSample.speedKph = 55;
            }
          }
          socket.send(JSON.stringify(message));
        });
      });
      const output = new URL(`../generated/board-views-editor/${name}/`, import.meta.url);
      await mkdir(new URL('journey/', output), { recursive: true });
      async function shot(filename: string) {
        await page.waitForTimeout(150); // Capture the settled paint, not the first frame of a new view.
        await page.screenshot({ animations: 'disabled', path: new URL(`journey/${filename}.png`, output).pathname });
      }
      async function switchView(label: 'Live View' | 'Custom View', keyboard = false) {
        await page.getByRole('button', { name: 'View options', exact: true }).tap();
        const menu = page.locator('[data-header-menu]');
        assert.deepEqual(
          (await menu.getByRole('button').allTextContents()).map((s) => s.trim()),
          [`Switch to ${label}`, 'Back']
        );
        assert.equal(await page.getByRole('dialog', { name: 'Board actions', exact: true }).count(), 0);
        const action = menu.getByRole('button', { name: `Switch to ${label}`, exact: true });
        if (keyboard) {
          await action.focus();
          await page.keyboard.press('Enter');
        } else await action.tap();
      }
      async function editCustom() {
        const custom = page.getByRole('region', { name: 'Custom dashboard', exact: true });
        await custom
          .getByRole('button', { name: /^Arrange / })
          .first()
          .focus();
        await page.keyboard.press('Enter');
        await custom.getByRole('button', { name: 'Save dashboard', exact: true }).waitFor();
      }
      async function backToBoard() {
        await page.getByRole('button', { name: 'View options', exact: true }).tap();
        await page.locator('[data-header-menu]').getByRole('button', { name: 'Back', exact: true }).tap();
        await page.getByRole('region', { name: 'Board summary', exact: true }).waitFor();
      }
      const open = async (captureMenu = false) => {
        await page.getByRole('button', { name: 'Board actions', exact: true }).tap();
        if (captureMenu) {
          const entries = (await page.locator('[data-header-menu]').getByRole('button').allTextContents()).map((s) =>
            s.trim()
          );
          assert.ok(entries.indexOf('Configure board') < entries.indexOf('Customise views'));
          await shot('03-board-actions');
        }
        await page.getByRole('button', { name: 'Customise views', exact: true }).tap();
        await page.getByRole('region', { name: 'Customise board views', exact: true }).waitFor();
        await page
          .getByRole('region', { name: 'Custom dashboard', exact: true })
          .locator('[data-dashboard-widget]')
          .first()
          .waitFor();
        assert.equal(
          await page.getByText('Custom View · Preview values', { exact: true }).count(),
          1,
          'opens the saved default'
        );
        if (captureMenu) {
          await shot('03b-default-custom');
          await page.getByRole('button', { name: 'View options', exact: true }).tap();
          await shot('03c-preview-menu');
          await page.keyboard.press('Escape');
        }
        await switchView('Live View');
        await page.getByRole('region', { name: 'Live board widgets', exact: true }).waitFor();
      };
      await page.goto(`${endpoint.origin}/#/sessions`);
      await page.getByRole('region', { name: 'Saved boards', exact: true }).waitFor();
      await shot('01-saved-boards');
      await page
        .getByRole('region', { name: 'Saved boards', exact: true })
        .getByRole('link', { name: /Trail Blazer/ })
        .tap();
      await page.getByRole('region', { name: 'Board summary', exact: true }).waitFor();
      await shot('02-board-history');
      await open(true);
      assert.equal(await page.getByRole('heading', { name: 'Trail Blazer', exact: true }).count(), 1);
      assert.equal(await page.getByRole('heading', { name: 'Trail Blazer', level: 1, exact: true }).count(), 1);
      assert.equal(await page.locator('[data-board-icon]').count(), 0, 'editor reserves no space for a board card');
      assert.equal(
        await page.getByRole('group', { name: 'View to customise' }).count(),
        0,
        'no view selector consumes dashboard space'
      );
      assert.equal(await page.getByText('Live View · Preview values', { exact: true }).count(), 1);
      assert.equal(
        requests.some((r) => ['bluetooth.scan', 'device.connect', 'boardProfile.update'].includes(r.method)),
        false
      );
      await page.waitForTimeout(100); // Let the freshly mounted preview paint before capturing it.
      await page.screenshot({ animations: 'disabled', path: new URL('live-phone.png', output).pathname });
      await shot('04-live-editor');
      const box = page.getByRole('button', { name: /^Choose value for box 1:/ });
      await box.click({ button: 'right' });
      await box.evaluate((node) => {
        const capture = node.setPointerCapture;
        node.setPointerCapture = () => {};
        const pointer = { bubbles: true, pointerId: 9, pointerType: 'touch', isPrimary: false, button: 0 };
        try {
          node.dispatchEvent(new PointerEvent('pointerdown', pointer));
          node.dispatchEvent(new PointerEvent('pointerup', pointer));
          node.dispatchEvent(new PointerEvent('pointerdown', { ...pointer, isPrimary: true }));
          node.dispatchEvent(new PointerEvent('pointercancel', { ...pointer, isPrimary: true }));
          node.dispatchEvent(new PointerEvent('pointerup', { ...pointer, isPrimary: true }));
        } finally {
          node.setPointerCapture = capture;
        }
      });
      assert.equal(await page.getByRole('dialog', { name: 'Choose a value' }).count(), 0);
      await box.tap();
      assert.equal(await page.getByRole('dialog', { name: 'Choose a value' }).count(), 0);
      await box.focus();
      await page.keyboard.press('Shift+Enter');
      const picker = page.getByRole('dialog', { name: 'Choose a value' });
      await picker.getByRole('searchbox', { name: 'Search values', exact: true }).fill('motor current');
      await shot('05-live-value-picker');
      failSave = 'unavailable';
      await picker.getByRole('button', { name: 'Actual motor current', exact: true }).tap();
      await picker.getByText('Storage unavailable', { exact: true }).waitFor();
      await picker.getByRole('button', { name: 'Actual motor current', exact: true }).focus();
      await page.keyboard.press('Enter');
      await picker.waitFor({ state: 'detached' });
      profile = await client.request('boardProfile.get', { savedDeviceId: id });
      assert.equal(profile.values.liveWidgetSlots?.[0], 'motorCurrentA');
      assert.deepEqual(profile.values.customDashboard, dashboard);
      await shot('06-live-choice-saved');
      await switchView('Custom View');
      const custom = page.getByRole('region', { name: 'Custom dashboard', exact: true });
      await editCustom();
      assert.equal(
        await page
          .getByText('Hold a widget or the background to arrange your dashboard.', { exact: true })
          .getAttribute('class'),
        'sr-only'
      );
      await page.waitForTimeout(100);
      await page.screenshot({ animations: 'disabled', path: new URL('custom-phone.png', output).pathname });
      await shot('07-custom-editor');
      await custom.getByRole('button', { name: 'Add widget', exact: true }).tap();
      await page.getByRole('searchbox', { name: 'Search widgets', exact: true }).fill('pack voltage');
      await shot('08-add-widget');
      await page.getByRole('button', { name: 'Pack voltage', exact: true }).tap();
      const addDialog = page.getByRole('dialog', { name: 'Add widget', exact: true });
      await addDialog.getByLabel('Content', { exact: true }).waitFor();
      await addDialog.getByRole('button', { name: 'Bar', exact: true }).tap();
      await addDialog.getByRole('button', { name: '2 columns by 1 row', exact: true }).tap();
      await shot('09-configure-widget');
      await addDialog.getByRole('button', { name: 'Add widget', exact: true }).tap();
      await custom.getByRole('button', { name: /^Configure Pack voltage,/ }).waitFor();
      await custom.locator('.dashboard-scroll').evaluate((node) => {
        node.scrollTop = node.scrollHeight;
      });
      await shot('10-custom-staged');
      await custom.getByRole('button', { name: 'Save dashboard', exact: true }).tap();
      await custom
        .getByRole('button', { name: /^Arrange / })
        .first()
        .waitFor();
      await backToBoard();
      await shot('11-saved-return');
      const savedDashboard = (await client.request('boardProfile.get', { savedDeviceId: id })).values.customDashboard;
      assert.ok(
        savedDashboard?.widgets.some(
          (w) => w.kind === 'reading' && w.field === 'packVoltageV' && w.presentation === 'bar'
        )
      );
      await open();
      await switchView('Custom View');
      await editCustom();
      await custom.getByRole('button', { name: /^Configure Pack voltage,/ }).waitFor();
      await custom.locator('.dashboard-scroll').evaluate((node) => {
        node.scrollTop = node.scrollHeight;
      });
      await shot('12-custom-reopened');
      await page.setViewportSize({ width: 1280, height: 900 });
      await page.screenshot({ path: new URL('custom-desktop.png', output).pathname });
      await page.setViewportSize({ width: 320, height: 740 });
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
      await page.setViewportSize({ width: 390, height: 844 });
      for (const label of ['Headlights', 'Light / dark mode', 'Save ride marker']) {
        await custom.getByRole('button', { name: new RegExp(`^Configure ${label},`) }).tap();
        await page
          .getByRole('dialog', { name: 'Configure widget', exact: true })
          .getByRole('button', { name: 'Cancel', exact: true })
          .tap();
      }
      await custom.getByRole('button', { name: 'Remove Headlights', exact: true }).tap();
      await switchView('Live View');
      await page.getByRole('alertdialog', { name: 'Discard dashboard changes?', exact: true }).waitFor();
      await page
        .getByRole('alertdialog', { name: 'Discard dashboard changes?', exact: true })
        .getByRole('button', { name: 'Cancel', exact: true })
        .tap();
      await page.evaluate(() => window.dispatchEvent(new Event('floatview:back', { cancelable: true })));
      await page.getByRole('alertdialog', { name: 'Discard dashboard changes?', exact: true }).waitFor();
      await page
        .getByRole('alertdialog', { name: 'Discard dashboard changes?', exact: true })
        .getByRole('button', { name: 'Cancel', exact: true })
        .tap();
      await page.getByRole('button', { name: 'View options', exact: true }).tap();
      await page.locator('[data-header-menu]').getByRole('button', { name: 'Back', exact: true }).tap();
      await page
        .getByRole('alertdialog', { name: 'Discard dashboard changes?', exact: true })
        .getByRole('button', { name: 'Cancel', exact: true })
        .tap();
      // A connection change must preserve the explicit target and dirty draft.
      otherBoard = true;
      await client.request('bluetooth.requestPermission', {});
      await client.request('bluetooth.scan', { timeoutMs: 100 });
      await client.request('device.connect', { deviceId: 'mock-board', trackingMode: 'saved' });
      await client.waitForState((s) => s.activeConnection?.state === 'ready');
      assert.equal(await custom.getByRole('button', { name: 'Remove Headlights', exact: true }).count(), 0);
      assert.equal(await page.getByRole('heading', { name: 'Trail Blazer', exact: true }).count(), 1);
      failSave = 'conflict';
      await custom.getByRole('button', { name: 'Save dashboard', exact: true }).tap();
      await custom
        .getByText('This board profile changed. Reload current values before saving again.', { exact: true })
        .waitFor();
      assert.equal(await custom.getByRole('button', { name: 'Save dashboard', exact: true }).isDisabled(), true);
      await custom.getByRole('button', { name: 'Reload current values', exact: true }).tap();
      await page.getByRole('button', { name: 'Discard changes', exact: true }).tap();
      await page
        .getByRole('alertdialog', { name: 'Discard dashboard changes?', exact: true })
        .waitFor({ state: 'detached' });
      await custom.getByText('Loading dashboard…', { exact: true }).waitFor({ state: 'hidden' });
      await custom
        .getByRole('button', { name: /^Arrange / })
        .first()
        .waitFor();
      await editCustom();
      await custom.getByRole('button', { name: 'Remove Headlights', exact: true }).waitFor();
      await custom.getByRole('button', { name: 'Remove Headlights', exact: true }).tap();
      await client.request('device.disconnect', {});
      await custom.getByRole('button', { name: 'Save dashboard', exact: true }).tap();
      await custom
        .getByRole('button', { name: /^Arrange / })
        .first()
        .waitFor();
      await backToBoard();
      profile = await client.request('boardProfile.get', { savedDeviceId: id });
      assert.equal(
        profile.values.customDashboard?.widgets.some((w) => w.id === 'lights'),
        false
      );
      assert.equal(profile.values.lastLiveView, 'custom');
      assert.equal(profile.values.liveWidgetSlots?.[0], 'motorCurrentA');
      assert.equal(
        requests.filter((r) => r.method === 'boardProfile.update').every((r) => r.params.savedDeviceId === id),
        true
      );
      assert.equal(
        requests.some((r) => ['refloat.control.set', 'session.marker.add', 'settings.update'].includes(r.method)),
        false
      );
      failLoad = true;
      await page.getByRole('button', { name: 'Board actions', exact: true }).tap();
      await page.getByRole('button', { name: 'Customise views', exact: true }).tap();
      await page.getByText('Storage unavailable', { exact: true }).waitFor();
      await page.getByRole('button', { name: 'Retry loading', exact: true }).tap();
      await page
        .getByRole('region', { name: 'Custom dashboard', exact: true })
        .locator('[data-dashboard-widget]')
        .first()
        .waitFor();
      await switchView('Live View');
      // Board/Connect offers the actual views; the preview editor is only available through Sessions.
      await page.getByRole('button', { name: 'View options', exact: true }).tap();
      await page.locator('[data-header-menu]').getByRole('button', { name: 'Back', exact: true }).tap();
      otherBoard = false;
      healthyLive = true;
      await client.request('device.connect', { deviceId: 'mock-board', trackingMode: 'saved' });
      await client.waitForState((s) => s.activeConnection?.state === 'ready');
      await page.goto(`${endpoint.origin}/#/live`);
      await page.getByRole('button', { name: 'Board actions', exact: true }).tap();
      const connectedMenu = page.getByRole('dialog', { name: 'Board actions', exact: true });
      const connectedActions = (await connectedMenu.getByRole('button').allTextContents()).map((s) => s.trim());
      assert.ok(connectedActions.includes('Configure board'));
      assert.equal(connectedActions.includes('Customise views'), false);
      await connectedMenu.getByRole('button', { name: 'Custom View', exact: true }).tap();
      await custom.locator('[data-dashboard-widget]').first().waitFor();
      assert.equal(await page.getByRole('region', { name: 'Customise board views', exact: true }).count(), 0);
      await page.getByRole('button', { name: 'Board actions', exact: true }).tap();
      await connectedMenu.getByRole('button', { name: 'Live View', exact: true }).tap();
      const live = page.getByRole('region', { name: 'Live board widgets', exact: true });
      await live.waitFor();
      const connectedBounds = (await live.boundingBox())!;
      await page.goto(`${endpoint.origin}/#/sessions`);
      await page
        .getByRole('region', { name: 'Saved boards', exact: true })
        .getByRole('link', { name: /Trail Blazer/ })
        .tap();
      await page.getByRole('region', { name: 'Board summary', exact: true }).waitFor();
      await page.getByRole('button', { name: 'Board actions', exact: true }).tap();
      await page.getByRole('button', { name: 'Customise views', exact: true }).tap();
      await page.getByText('Live View · Preview values', { exact: true }).waitFor();
      await live.waitFor();
      const previewBounds = (await live.boundingBox())!;
      for (const dimension of ['x', 'y', 'width', 'height'] as const)
        assert.ok(
          Math.abs(previewBounds[dimension] - connectedBounds[dimension]) < 1,
          `preview preserves Live viewport ${dimension}: ${JSON.stringify({ previewBounds, connectedBounds })}`
        );
      await backToBoard();
      assert.equal((await client.request('state.get', {})).activeConnection?.state, 'ready');
      await page.getByRole('button', { name: 'Board actions', exact: true }).tap();
      await page.getByRole('button', { name: 'Customise views', exact: true }).tap();
      await live.waitFor();
      await switchView('Custom View');
      await editCustom();
      await client.request('device.disconnect', {});
      await custom.getByRole('button', { name: 'Remove Battery', exact: true }).tap();
      await custom.getByRole('button', { name: 'Save dashboard', exact: true }).tap();
      await custom
        .getByRole('button', { name: /^Arrange / })
        .first()
        .waitFor();
      await backToBoard();
      // Unconfigured dashboards preview the selected board's Live choices without persisting defaults.
      profile = await client.request('boardProfile.get', { savedDeviceId: id });
      await client.request('boardProfile.update', {
        savedDeviceId: id,
        expectedRevision: profile.revision,
        patch: { customDashboard: null, lastLiveView: 'custom' },
      });
      const writesBeforeDefault = requests.filter((r) => r.method === 'boardProfile.update').length;
      await open();
      await switchView('Custom View', true);
      await editCustom();
      await custom.getByRole('button', { name: /^Configure Actual motor current,/ }).waitFor();
      assert.equal(requests.filter((r) => r.method === 'boardProfile.update').length, writesBeforeDefault);
      await page.getByRole('button', { name: 'View options', exact: true }).tap();
      await page.locator('[data-header-menu]').getByRole('button', { name: 'Back', exact: true }).tap();
      for (const preference of [null, 'detailed'] as const) {
        profile = await client.request('boardProfile.get', { savedDeviceId: id });
        await client.request('boardProfile.update', {
          savedDeviceId: id,
          expectedRevision: profile.revision,
          patch: { lastLiveView: preference },
        });
        await page.getByRole('button', { name: 'Board actions', exact: true }).tap();
        await page.getByRole('button', { name: 'Customise views', exact: true }).tap();
        await page.getByRole('region', { name: 'Live board widgets', exact: true }).waitFor();
        assert.equal((await client.request('boardProfile.get', { savedDeviceId: id })).values.lastLiveView, preference);
        await page.getByRole('button', { name: 'View options', exact: true }).tap();
        await page.locator('[data-header-menu]').getByRole('button', { name: 'Back', exact: true }).tap();
      }
      await page.getByRole('button', { name: 'Board actions', exact: true }).tap();
      await page.getByRole('button', { name: 'Customise views', exact: true }).tap();
      await page.getByRole('region', { name: 'Live board widgets', exact: true }).waitFor();
      await switchView('Custom View');
      await editCustom();
      await custom.getByRole('button', { name: 'Remove Battery', exact: true }).tap();
      await client.request('savedDevice.delete', { savedDeviceId: id });
      await page.getByRole('heading', { name: 'Sessions', exact: true }).waitFor();
      assert.equal(await custom.count(), 0);
      assert.deepEqual(errors, []);
    } finally {
      client.close();
      await browser.close();
      await stopMockServer();
    }
  });
}
