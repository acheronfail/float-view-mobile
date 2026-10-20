import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: Refloat edits and position survive a destroyed frontend`, { timeout: 60000 }, async () => {
    const port = await startMockServer(11002, { safetyDisclaimerAccepted: true });
    const endpoint = getMockBootstrap(port);
    const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
    const browser = await engine.launch();
    try {
      await client.request('bluetooth.requestPermission', {});
      await client.request('bluetooth.scan', { timeoutMs: 100 });
      const scan = await client.waitForState((s) => s.discoveredDevices.length > 0);
      await client.request('device.connect', { deviceId: scan.discoveredDevices[0]!.deviceId, trackingMode: 'saved' });
      const state = await client.waitForState((s) => s.activeConnection?.state === 'ready');
      const deviceId = state.activeConnection!.deviceId;
      const context = await browser.newContext({ viewport: { width: 390, height: 600 }, hasTouch: true });
      if (name === 'chromium') await context.grantPermissions(['local-network-access'], { origin: endpoint.origin });
      await context.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      const mutations: string[] = [];
      const reads: Array<{ source: string; refresh?: boolean }> = [];
      const errors: string[] = [];
      await context.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        server.onMessage((raw) => socket.send(raw));
        socket.onMessage((raw) => {
          const request = JSON.parse(String(raw));
          if (request.method === 'refloat.configuration.read') reads.push(request.params);
          if (request.method === 'refloat.configuration.write' || request.method === 'refloat.tune.apply')
            mutations.push(request.method);
          server.send(raw);
        });
      });
      let page = await context.newPage();
      page.setDefaultTimeout(5000);
      page.on('pageerror', (e) => errors.push(e.message));
      await page.goto(`${endpoint.origin}/`);
      await page.getByRole('button', { name: 'Board actions', exact: true }).tap();
      await page.getByRole('button', { name: 'Configure Refloat', exact: true }).tap();
      const number = () => page.getByRole('spinbutton', { name: 'Duty Tiltback', exact: true });
      await number().fill('0.85');
      await page.getByRole('heading', { name: 'Duty Pushback', level: 4, exact: true }).waitFor();
      const controls = () => page.getByRole('button', { name: 'Section and search controls', exact: true });
      const section = () => page.locator('select[aria-label="Section"]');
      assert.equal(await controls().getAttribute('aria-expanded'), 'false');
      const balanceKey = await section().inputValue();
      const startupKey = await section()
        .locator('option')
        .filter({ hasText: 'Startup defaults' })
        .getAttribute('value');
      assert.ok(startupKey);
      // Native select popups are outside Playwright's page surface; use its selection API.
      await section().selectOption(startupKey);
      await page.getByRole('heading', { name: 'Lighting · Startup defaults', exact: true }).waitFor();
      const lightsHeading = page.getByRole('heading', { name: 'Lights', level: 4, exact: true });
      const headlightsHeading = page.getByRole('heading', { name: 'Headlights', level: 4, exact: true });
      await lightsHeading.waitFor();
      await headlightsHeading.waitFor();
      for (const width of [390, 1024]) {
        await page.setViewportSize({ width, height: 600 });
        const lights = await lightsHeading.boundingBox();
        const headlights = await headlightsHeading.boundingBox();
        assert.ok(lights && headlights);
        assert.ok(Math.abs(lights.x - headlights.x) < 1);
        assert.ok(Math.abs(lights.width - headlights.width) < 1);
        const grid = await lightsHeading.locator('..').boundingBox();
        assert.ok(grid);
        assert.ok(Math.abs(lights.width - grid.width) < 1, 'subsection headings must span the control grid');
      }
      await page.setViewportSize({ width: 390, height: 600 });
      assert.equal(await controls().getAttribute('aria-expanded'), 'false');
      await section().selectOption(balanceKey);
      await number().waitFor();
      assert.equal(await section().inputValue(), balanceKey);
      assert.equal(await controls().getAttribute('aria-expanded'), 'false');
      await controls().dispatchEvent('pointerdown', { button: 0, isPrimary: false, pointerId: 2 });
      await controls().dispatchEvent('pointerup', { button: 0, isPrimary: false, pointerId: 2 });
      assert.equal(await controls().getAttribute('aria-expanded'), 'false');
      await controls().tap();
      assert.equal(await controls().getAttribute('aria-expanded'), 'true');
      await page.getByRole('button', { name: 'Startup defaults', exact: true }).tap();
      await page
        .getByRole('group', { name: 'LEDs On (persistent startup default)', exact: true })
        .getByRole('button', { name: 'Off', exact: true })
        .tap();
      await page.getByRole('searchbox').fill('Headlights');
      await headlightsHeading.waitFor();
      assert.equal(await lightsHeading.count(), 0);
      await page.getByRole('searchbox').fill('on');
      await lightsHeading.waitFor();
      await headlightsHeading.waitFor();
      await page.getByRole('button', { name: 'Section and search controls', exact: true }).tap();
      await page.setViewportSize({ width: 390, height: 320 });
      const scroller = () =>
        page
          .locator('div.overflow-y-auto')
          .filter({ has: page.getByRole('heading', { name: 'Lighting · Startup defaults', exact: true }) });
      await scroller().evaluate((el) => {
        el.scrollTop = 60;
      });
      const waitDraft = async (predicate: (draft: NonNullable<Awaited<ReturnType<typeof readDraft>>>) => boolean) => {
        for (let i = 0; i < 100; i++) {
          const draft = await readDraft();
          if (draft && predicate(draft)) return draft;
          await new Promise((resolve) => setTimeout(resolve, 20));
        }
        throw new Error('Draft was not saved');
      };
      const readDraft = () => client.request('refloat.configuration.draft.get', { deviceId });
      const saved = await waitDraft(
        (d) =>
          d.values['leds.on']?.value === false &&
          d.position.search === 'on' &&
          !d.position.controlsExpanded &&
          d.position.scrollTop > 0
      );
      await page.close();
      page = await context.newPage();
      page.setDefaultTimeout(5000);
      page.on('pageerror', (e) => errors.push(e.message));
      await page.setViewportSize({ width: 390, height: 320 });
      await page.goto(`${endpoint.origin}/`);
      await page.getByRole('dialog', { name: 'Configure Refloat', exact: true }).waitFor();
      await page.getByText('Draft restored · 2 changes', { exact: true }).waitFor();
      assert.equal(
        await page
          .getByRole('button', { name: 'Section and search controls', exact: true })
          .getAttribute('aria-expanded'),
        'false'
      );
      assert.ok(Math.abs((await scroller().evaluate((el) => el.scrollTop)) - saved.position.scrollTop) < 2);
      // A secondary pointer must not activate the collapse control; keyboard activates once.
      await page
        .getByRole('button', { name: 'Section and search controls', exact: true })
        .dispatchEvent('pointerdown', { button: 2, isPrimary: true });
      assert.equal(
        await page
          .getByRole('button', { name: 'Section and search controls', exact: true })
          .getAttribute('aria-expanded'),
        'false'
      );
      await page.setViewportSize({ width: 390, height: 600 });
      await page.getByRole('button', { name: 'Section and search controls', exact: true }).focus();
      await page.keyboard.press('Enter');
      assert.equal(await page.getByRole('searchbox').inputValue(), 'on');
      assert.equal(
        await page.getByRole('button', { name: 'Startup defaults', exact: true }).getAttribute('aria-pressed'),
        'true'
      );
      await page.getByRole('searchbox').fill('');
      await page.getByRole('button', { name: 'Balance', exact: true }).tap();
      assert.equal(await number().inputValue(), '0.85');
      await number().fill('');
      await waitDraft((d) => d.inputs.tiltback_duty === '');
      await page.close();
      page = await context.newPage();
      page.setDefaultTimeout(5000);
      page.on('pageerror', (e) => errors.push(e.message));
      await page.goto(`${endpoint.origin}/`);
      await number().waitFor();
      assert.equal(await number().inputValue(), '');
      assert.equal(await page.getByRole('button', { name: 'Write configuration', exact: true }).isDisabled(), true);
      assert.equal(reads.length, 3);
      assert.ok(
        reads.every((read) => read.refresh === false),
        'restoration requested fresh board reads'
      );
      await page.getByRole('button', { name: 'Read current', exact: true }).tap();
      await waitDraft(
        (d) => Object.keys(d.inputs).length === 0 && d.values.tiltback_duty?.value === 0.8 && d.source === 'current'
      );
      assert.equal(await number().inputValue(), '0.8');
      await number().fill('0.86');
      await page.getByRole('button', { name: 'Read defaults', exact: true }).tap();
      await waitDraft(
        (d) => Object.keys(d.inputs).length === 0 && d.values.tiltback_duty?.value === 0.8 && d.source === 'default'
      );
      assert.equal(await number().inputValue(), '0.8');
      assert.deepEqual(reads.slice(-2), [
        { source: 'current', refresh: true },
        { source: 'default', refresh: true },
      ]);
      assert.equal(await page.getByRole('button', { name: 'Discard changes', exact: true }).count(), 0);
      await page.getByRole('button', { name: 'Done', exact: true }).tap();
      assert.equal(await page.getByRole('dialog', { name: /Discard/ }).count(), 0);
      assert.deepEqual(mutations, []);
      assert.deepEqual(errors, []);
    } finally {
      await browser.close();
      client.close();
      await stopMockServer();
    }
  });
  test(`${name}: configuration write failures stay visible in the confirmation`, { timeout: 60000 }, async () => {
    const port = await startMockServer(11002, { safetyDisclaimerAccepted: true });
    const endpoint = getMockBootstrap(port);
    const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
    const browser = await engine.launch();
    try {
      await client.request('bluetooth.requestPermission', {});
      await client.request('bluetooth.scan', { timeoutMs: 100 });
      const scan = await client.waitForState((s) => s.discoveredDevices.length > 0);
      await client.request('device.connect', { deviceId: scan.discoveredDevices[0]!.deviceId, trackingMode: 'saved' });
      await client.waitForState((s) => s.activeConnection?.state === 'ready');
      const context = await browser.newContext({ viewport: { width: 390, height: 600 }, hasTouch: true });
      if (name === 'chromium') await context.grantPermissions(['local-network-access'], { origin: endpoint.origin });
      await context.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      const current = await client.request('refloat.configuration.read', { source: 'current', refresh: true });
      let failWrite = true;
      let writes = 0;
      await context.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        server.onMessage((raw) => socket.send(raw));
        socket.onMessage((raw) => {
          const request = JSON.parse(String(raw));
          if (request.method === 'refloat.configuration.write') {
            writes++;
            if (failWrite) {
              socket.send(
                JSON.stringify({
                  id: request.id,
                  error: { code: 'invalid_state', message: 'Configuration test failure' },
                })
              );
              return;
            }
            socket.send(
              JSON.stringify({
                id: request.id,
                result: { ...current, revision: current.revision + 1, values: request.params.values },
              })
            );
            return;
          }
          server.send(raw);
        });
      });
      const page = await context.newPage();
      page.setDefaultTimeout(5000);
      const errors: string[] = [];
      page.on('pageerror', (error) => errors.push(error.message));
      await page.goto(`${endpoint.origin}/`);
      await page.getByRole('button', { name: 'Board actions', exact: true }).tap();
      await page.getByRole('button', { name: 'Configure Refloat', exact: true }).tap();
      const input = page.getByRole('spinbutton', { name: 'Duty Tiltback', exact: true });
      await input.fill('0.85');
      await page.getByRole('button', { name: 'Section and search controls', exact: true }).tap();
      await page.setViewportSize({ width: 390, height: 400 });
      const scroller = page.locator('div.overflow-y-auto').filter({ has: input });
      await page.waitForFunction(() => {
        const input = document.querySelector('input[type=number]');
        const scroller = input?.closest('div.overflow-y-auto');
        return scroller && scroller.scrollHeight > scroller.clientHeight;
      });
      await scroller.evaluate((el) => {
        el.scrollTop = el.scrollHeight;
      });
      const scrollTop = await scroller.evaluate((el) => el.scrollTop);
      assert.ok(scrollTop > 0);
      const open = page.getByRole('button', { name: 'Write configuration', exact: true });
      await open.tap();
      const dialog = page.getByRole('alertdialog', { name: 'Write persistent Refloat configuration?' });
      const confirm = dialog.getByRole('button', { name: 'Write and verify', exact: true });
      await confirm.dispatchEvent('pointerdown', { button: 0, isPrimary: false, pointerId: 2 });
      await confirm.dispatchEvent('pointerup', { button: 0, isPrimary: false, pointerId: 2 });
      assert.equal(writes, 0);
      await confirm.tap();
      const alert = dialog.getByRole('alert');
      await alert.waitFor();
      assert.equal(await alert.textContent(), 'Configuration test failure');
      const bounds = await alert.boundingBox();
      assert.ok(bounds && bounds.y >= 0 && bounds.y + bounds.height <= 400, 'write error is outside the viewport');
      assert.equal(await scroller.evaluate((el) => el.scrollTop), scrollTop);
      assert.equal(await confirm.isEnabled(), true);
      await dialog.getByRole('button', { name: 'Cancel', exact: true }).tap();
      await dialog.waitFor({ state: 'hidden' });
      assert.equal(await input.inputValue(), '0.85');
      await page.setViewportSize({ width: 390, height: 600 });
      await open.tap();
      assert.equal(await dialog.getByRole('alert').count(), 0, 'reopening retained the old error');
      await confirm.tap();
      await alert.waitFor();
      failWrite = false;
      await confirm.focus();
      await page.keyboard.press('Enter');
      await dialog.waitFor({ state: 'hidden' });
      assert.equal(writes, 3, 'pointer or keyboard activation duplicated a write');
      assert.equal(await open.isDisabled(), true, 'successful write did not clear the pending changes');
      assert.deepEqual(errors, []);
    } finally {
      await browser.close();
      client.close();
      await stopMockServer();
    }
  });
}
