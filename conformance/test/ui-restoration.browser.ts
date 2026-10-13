import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';
import type { UiCheckpoint } from '../../frontend/src/lib/backend/contract.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: UI checkpoint restores a fresh frontend without replaying mutations`, { timeout: 60000 }, async () => {
    const port = await startMockServer(11002);
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
      const state = await client.waitForState((s) => s.activeConnection?.state === 'ready');
      const connection = state.activeConnection!;
      const context = await browser.newContext({ viewport: { width: 390, height: 844 }, hasTouch: true });
      if (name === 'chromium') await context.grantPermissions(['local-network-access'], { origin: endpoint.origin });
      await context.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      const errors: string[] = [];
      const mutations: string[] = [];
      let injectBms = false;
      let injectPending = false;
      let injectFlywheel = false;
      let failCheckpointRead = false;
      let holdCheckpointRead = false;
      let releaseRead: (() => void) | undefined;
      let heldCheckpoint: UiCheckpoint | null = null;
      await context.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        server.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          const state =
            message.event === 'state.changed' ? message.data : message.result?.activeConnection ? message.result : null;
          if (injectBms && state?.activeConnection)
            state.activeConnection.bms = {
              status: 'unavailable',
              connection: null,
              candidates: [],
              latestSnapshot: null,
            };
          if (injectPending && state?.activeConnection)
            state.activeConnection.controllerActions.pendingAction = 'shutdown';
          if (injectFlywheel && state?.activeConnection?.refloatControls)
            state.activeConnection.refloatControls.flywheelEnabled = true;
          socket.send(JSON.stringify(message));
        });
        socket.onMessage((raw) => {
          const request = JSON.parse(String(raw));
          const method = request.method;
          if (holdCheckpointRead && method === 'uiState.get') {
            releaseRead = () => socket.send(JSON.stringify({ id: request.id, result: heldCheckpoint }));
            return;
          }
          if (failCheckpointRead && method === 'uiState.get') {
            socket.send(
              JSON.stringify({ id: request.id, error: { code: 'unavailable', message: 'Temporary failure' } })
            );
            return;
          }
          if (/\.write$|\.apply$|\.update$|\.delete$|shutdown$|disconnect$/.test(method)) mutations.push(method);
          server.send(raw);
        });
      });
      let page = await context.newPage();
      page.on('pageerror', (e) => errors.push(e.message));
      page.setDefaultTimeout(5000);
      await page.goto(`${endpoint.origin}/`);
      const menu = page.getByRole('button', { name: 'Board actions', exact: true });
      await menu.tap();
      await page.getByRole('button', { name: 'Detailed View', exact: true }).tap();
      await menu.tap();
      await page.getByRole('button', { name: 'Live charts…', exact: true }).tap();
      await page.getByRole('dialog', { name: 'Live charts', exact: true }).waitFor();
      for (let i = 0; i < 50; i++) {
        if ((await client.request('uiState.get', {}))?.surface?.kind === 'liveCharts') break;
        await new Promise((resolve) => setTimeout(resolve, 20));
      }
      const captured = await client.request('uiState.get', {});
      assert.equal(captured?.live?.mode, 'detailed');
      assert.equal(captured?.surface?.kind, 'liveCharts');
      await page.close();
      page = await context.newPage();
      page.setDefaultTimeout(5000);
      page.on('pageerror', (e) => errors.push(e.message));
      await page.goto(`${endpoint.origin}/`);
      await page.getByRole('dialog', { name: 'Live charts', exact: true }).waitFor();
      assert.match(page.url(), /#\/live$/);
      await page.getByRole('button', { name: 'Done', exact: true }).tap();
      const cp: UiCheckpoint = {
        version: 1,
        route: {
          kind: 'session',
          sessionId: connection.activeSession!.sessionId,
          savedDeviceId: connection.savedDeviceId,
        },
        live: null,
        surface: null,
      };
      // Explicitly set an accepted checkpoint while the frontend is absent.
      await page.close();
      await client.request('uiState.set', { checkpoint: cp });
      page = await context.newPage();
      page.setDefaultTimeout(5000);
      page.on('pageerror', (e) => errors.push(e.message));
      await page.goto(`${endpoint.origin}/`);
      await page.getByRole('heading', { name: 'Current ride', exact: true }).waitFor();
      assert.ok(page.url().includes(encodeURIComponent(connection.activeSession!.sessionId)));
      assert.equal((await client.request('state.get', {})).activeConnection?.deviceId, connection.deviceId);
      await page.close();
      await client.request('uiState.set', {
        checkpoint: { ...cp, route: { kind: 'session', sessionId: 'removed', savedDeviceId: null } },
      });
      page = await context.newPage();
      page.setDefaultTimeout(5000);
      await page.goto(`${endpoint.origin}/`);
      await page.getByRole('heading', { name: 'Sessions', exact: true }).waitFor();
      await page.close();
      for (const surface of [
        { kind: 'bmsDetails' },
        { kind: 'liveActions' },
        { kind: 'refloatConfiguration' },
        { kind: 'refloatTunes' },
        { kind: 'boardConfiguration', savedDeviceId: connection.savedDeviceId!, batteryOnly: false },
        { kind: 'boardConfiguration', savedDeviceId: connection.savedDeviceId!, batteryOnly: true },
      ] as UiCheckpoint['surface'][]) {
        injectBms = surface?.kind === 'bmsDetails';
        await client.request('uiState.set', {
          checkpoint: {
            version: 1,
            route: { kind: 'live' },
            live: { deviceId: connection.deviceId, mode: 'detailed' },
            surface,
          },
        });
        page = await context.newPage();
        page.setDefaultTimeout(5000);
        page.on('pageerror', (e) => errors.push(e.message));
        await page.goto(`${endpoint.origin}/`);
        await page.getByRole('dialog').waitFor();
        assert.equal(await page.getByRole('dialog').count(), 1);
        await page.close();
      }
      for (const surface of [
        { kind: 'boardActions' },
        { kind: 'boardConfiguration', savedDeviceId: connection.savedDeviceId!, batteryOnly: false },
      ] as UiCheckpoint['surface'][]) {
        await client.request('uiState.set', {
          checkpoint: {
            version: 1,
            route: { kind: 'boardHistory', savedDeviceId: connection.savedDeviceId! },
            live: null,
            surface,
          },
        });
        page = await context.newPage();
        page.setDefaultTimeout(5000);
        page.on('pageerror', (e) => errors.push(e.message));
        await page.goto(`${endpoint.origin}/`);
        if (surface!.kind === 'boardActions')
          await page.getByRole('button', { name: 'Board actions', exact: true }).waitFor();
        else await page.getByRole('dialog').waitFor();
        assert.match(page.url(), /#\/sessions\/boards\//);
        if (surface!.kind === 'boardActions')
          assert.equal(
            await page.getByRole('button', { name: 'Board actions', exact: true }).getAttribute('aria-expanded'),
            'true'
          );
        await page.close();
      }
      for (const kind of ['pending', 'flywheel'] as const) {
        injectPending = kind === 'pending';
        injectFlywheel = kind === 'flywheel';
        await client.request('uiState.set', {
          checkpoint: {
            version: 1,
            route: { kind: 'live' },
            live: { deviceId: connection.deviceId, mode: 'normal' },
            surface:
              kind === 'flywheel'
                ? { kind: 'boardConfiguration', savedDeviceId: connection.savedDeviceId!, batteryOnly: false }
                : null,
          },
        });
        page = await context.newPage();
        page.setDefaultTimeout(5000);
        page.on('pageerror', (e) => errors.push(e.message));
        await page.goto(`${endpoint.origin}/`);
        if (kind === 'flywheel') {
          await page.getByRole('alertdialog', { name: 'Flywheel mode is active' }).waitFor();
          assert.equal(await page.getByRole('dialog').count(), 0);
        } else {
          await page.getByRole('button', { name: 'Board actions', exact: true }).tap();
          assert.equal(
            await page.getByRole('button', { name: 'Shut down controller', exact: true }).isDisabled(),
            true
          );
        }
        await page.close();
      }
      injectPending = false;
      injectFlywheel = false;
      const settings: UiCheckpoint = { version: 1, route: { kind: 'settings' }, live: null, surface: null };
      await client.request('uiState.set', { checkpoint: settings });
      failCheckpointRead = true;
      page = await context.newPage();
      page.setDefaultTimeout(5000);
      await page.goto(`${endpoint.origin}/`);
      await page.getByRole('alert').waitFor();
      assert.deepEqual(await client.request('uiState.get', {}), settings);
      failCheckpointRead = false;
      await page.getByRole('button', { name: 'Retry', exact: true }).tap();
      await page.getByRole('heading', { name: 'Settings', exact: true }).waitFor();
      await page.close();
      heldCheckpoint = {
        version: 1,
        route: { kind: 'live' },
        live: { deviceId: connection.deviceId, mode: 'detailed' },
        surface: { kind: 'liveCharts' },
      };
      await client.request('uiState.set', { checkpoint: heldCheckpoint });
      holdCheckpointRead = true;
      page = await context.newPage();
      page.setDefaultTimeout(5000);
      page.on('pageerror', (e) => errors.push(e.message));
      await page.goto(`${endpoint.origin}/`);
      await page.getByText('Resuming…', { exact: true }).waitFor();
      await page.getByRole('link', { name: 'sessions', exact: true }).tap();
      await page.getByRole('heading', { name: 'Sessions', exact: true }).waitFor();
      assert.ok(releaseRead);
      releaseRead!();
      await page.waitForTimeout(100);
      assert.match(page.url(), /#\/sessions$/);
      assert.equal(await page.getByRole('dialog').count(), 0);
      assert.equal(mutations.length, 0);
      assert.deepEqual(errors, []);
    } finally {
      await browser.close();
      client.close();
      await stopMockServer();
    }
  });
}
