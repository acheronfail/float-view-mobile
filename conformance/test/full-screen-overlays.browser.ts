import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: full-screen panels, dismissal and nested confirmations`, { timeout: 60_000 }, async () => {
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
      await client.waitForState((s) => s.activeConnection?.state === 'ready');
      const page = await browser.newPage({ hasTouch: true, reducedMotion: 'reduce' });
      page.setDefaultTimeout(5000);
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      const errors: string[] = [];
      page.on('pageerror', (e) => errors.push(e.message));
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        server.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          const state =
            message.event === 'state.changed' ? message.data : message.result?.activeConnection ? message.result : null;
          if (state?.activeConnection) {
            state.activeConnection.bms = {
              status: 'attached',
              connection: { kind: 'can', canId: 10 },
              candidates: [],
              latestSnapshot: null,
            };
          }
          socket.send(JSON.stringify(message));
        });
      });
      await page.goto(`${endpoint.origin}/#/live`);
      const menu = page.getByRole('button', { name: 'Board actions', exact: true });
      const actionOverlay = page.getByRole('dialog', { name: 'Board actions', exact: true });
      for (const viewport of [
        { width: 390, height: 844 },
        { width: 1280, height: 800 },
      ]) {
        await page.setViewportSize(viewport);
        for (const [label, title] of [
          ['Configure Refloat', 'Configure Refloat'],
          ['Tunes…', 'Tunes'],
          ['BMS', 'Battery management system'],
          ['Configure board', 'Board configuration'],
        ]) {
          await menu.tap();
          await actionOverlay.getByRole('button', { name: label, exact: true }).tap();
          const panel = page.getByRole('dialog', { name: title, exact: true });
          await panel.waitFor();
          await actionOverlay.waitFor({ state: 'hidden' });
          assert.deepEqual(await panel.boundingBox(), { x: 0, y: 0, ...viewport });
          const done = panel.getByRole('button', { name: 'Done', exact: true });
          assert.equal(await done.evaluate((el) => el === document.activeElement), true);
          await done.click({ button: 'right' });
          assert.equal(await panel.isVisible(), true, 'non-primary activation must leave the panel open');
          await done.evaluate((el) =>
            el.addEventListener(
              'pointerdown',
              (event) => {
                el.setAttribute('data-test-pointer', String((event as PointerEvent).pointerId));
              },
              { once: true }
            )
          );
          await done.hover();
          await page.mouse.down();
          await done.dispatchEvent('pointercancel', {
            pointerId: Number(await done.getAttribute('data-test-pointer')),
          });
          await page.mouse.up();
          assert.equal(await panel.isVisible(), true, 'cancelled activation must leave the panel open');
          await page.keyboard.press('Shift+Tab');
          assert.equal(
            await panel.evaluate((el) => el.contains(document.activeElement)),
            true,
            'focus stays inside the panel, including panels with only a Done control'
          );
          await page.keyboard.press('Tab');
          assert.equal(await done.evaluate((el) => el === document.activeElement), true);
          const scroll = panel.locator('div.overflow-y-auto');
          const headerY = (await done.boundingBox())!.y;
          await scroll.evaluate((el) => {
            el.scrollTop = el.scrollHeight;
          });
          assert.equal((await done.boundingBox())!.y, headerY, 'header stays visible while content scrolls');
          if (title === 'Configure Refloat') {
            const footer = panel.getByRole('button', { name: 'Write configuration' }).locator('..');
            const footerBox = (await footer.boundingBox())!;
            assert.equal(Math.round(footerBox.y + footerBox.height), viewport.height, 'actions cover the bottom edge');
            assert.ok(footerBox.height <= 80, 'actions remain compact');
            assert.notEqual(
              await panel.locator('header').evaluate((el) => getComputedStyle(el).boxShadow),
              'none',
              'header separates from the list'
            );
            const output = process.env.FLOAT_VIEW_SCREENSHOTS;
            if (output && name === 'chromium') {
              await mkdir(output, { recursive: true });
              await scroll.evaluate((el) => {
                el.scrollTop = Math.min(300, el.scrollHeight - el.clientHeight);
              });
              await page.evaluate(() => {
                document.documentElement.dataset.theme = 'dark';
              });
              await page.screenshot({ path: `${output}/refloat-configure-${viewport.width}.png` });
              await page.evaluate(() => {
                document.documentElement.dataset.theme = 'light';
              });
            }
            const controls = panel.getByRole('button', { name: 'Section and search controls', exact: true });
            if ((await controls.getAttribute('aria-expanded')) === 'false') await controls.tap();
            const search = panel.getByRole('searchbox');
            const source = panel.getByText('Source:', { exact: false });
            assert.equal(await source.isVisible(), true);
            const expandedY = (await scroll.boundingBox())!.y;
            await controls.click({ button: 'right' });
            await controls.dispatchEvent('pointerdown', { button: 0, isPrimary: false, pointerId: 2 });
            await controls.dispatchEvent('pointerup', { button: 0, isPrimary: false, pointerId: 2 });
            assert.equal(await controls.getAttribute('aria-expanded'), 'true');
            await controls.evaluate((el) =>
              el.addEventListener(
                'pointerdown',
                (event) => el.setAttribute('data-test-pointer', String((event as PointerEvent).pointerId)),
                { once: true }
              )
            );
            await controls.hover();
            await page.mouse.down();
            await controls.dispatchEvent('pointercancel', {
              pointerId: Number(await controls.getAttribute('data-test-pointer')),
            });
            await page.mouse.up();
            assert.equal(await controls.getAttribute('aria-expanded'), 'true');
            await search.fill('Duty');
            await controls.tap();
            assert.equal(await controls.getAttribute('aria-expanded'), 'false', 'touch collapses exactly once');
            assert.equal(await search.isVisible(), false);
            assert.equal(await source.isVisible(), false, 'collapse hides the source and changed-count row');
            assert.ok(expandedY - (await scroll.boundingBox())!.y > 100, 'collapse frees space for fields');
            assert.match(await controls.innerText(), /Filtered/);
            assert.equal(
              await panel.getByRole('combobox', { name: 'Section', exact: true }).locator('option:checked').innerText(),
              'Balance'
            );
            if (output && name === 'chromium') {
              await page.screenshot({ path: `${output}/refloat-configure-collapsed-${viewport.width}.png` });
            }
            await controls.focus();
            await page.keyboard.press('Enter');
            assert.equal(await controls.getAttribute('aria-expanded'), 'true');
            assert.equal(await source.isVisible(), true);
            assert.equal(await search.inputValue(), 'Duty', 'collapse preserves the filter');
            await search.fill('');
            const sections = panel.getByRole('group', { name: 'Section', exact: true });
            await sections.getByRole('button', { name: 'Startup defaults', exact: true }).tap();
            await controls.focus();
            await page.keyboard.press('Space');
            assert.equal(await controls.getAttribute('aria-expanded'), 'false');
            assert.equal(
              await panel.getByRole('combobox', { name: 'Section', exact: true }).locator('option:checked').innerText(),
              'Startup defaults'
            );
            await controls.dispatchEvent('click', { detail: 0 });
            assert.equal(await controls.getAttribute('aria-expanded'), 'true');
            assert.equal(
              await sections
                .getByRole('button', { name: 'Startup defaults', exact: true })
                .getAttribute('aria-pressed'),
              'true',
              'collapse preserves the selected section'
            );
            await sections.getByRole('button', { name: 'Balance', exact: true }).tap();
          }
          if (viewport.width === 390) await done.tap();
          else await page.keyboard.press('Escape');
          await panel.waitFor({ state: 'hidden' });
          assert.equal(await menu.evaluate((el) => el === document.activeElement), true);
        }
      }
      await menu.tap();
      await actionOverlay.getByRole('button', { name: 'Configure Refloat', exact: true }).tap();
      const editor = page.getByRole('dialog', { name: 'Configure Refloat', exact: true });
      await editor.getByRole('spinbutton').first().fill('0.85');
      await editor.getByRole('button', { name: 'Done', exact: true }).tap();
      await editor.waitFor({ state: 'hidden' });
      await menu.tap();
      await actionOverlay.getByRole('button', { name: 'Configure Refloat', exact: true }).tap();
      await editor.getByRole('spinbutton').first().waitFor();
      assert.equal(await editor.getByRole('spinbutton').first().inputValue(), '0.85', 'Done retains the draft');
      await editor.getByRole('button', { name: 'Write configuration', exact: true }).tap();
      const confirmation = page.getByRole('alertdialog', {
        name: 'Write persistent Refloat configuration?',
        exact: true,
      });
      await confirmation.waitFor();
      await page.keyboard.press('Escape');
      await confirmation.waitFor({ state: 'hidden' });
      assert.equal(await editor.isVisible(), true, 'Escape cancels the confirmation without closing the editor');
      await editor.getByRole('button', { name: 'Done', exact: true }).tap();
      await editor.waitFor({ state: 'hidden' });
      const battery = page.getByRole('button', { name: 'Open battery management system', exact: true });
      await battery.focus();
      await page.keyboard.press('Enter');
      const bms = page.getByRole('dialog', { name: 'Battery management system', exact: true });
      await bms.waitFor();
      await page.keyboard.press('Escape');
      await bms.waitFor({ state: 'hidden' });
      assert.equal(
        await battery.evaluate((el) => el === document.activeElement),
        true,
        'BMS returns focus to its widget opener'
      );
      assert.deepEqual(errors, []);
    } finally {
      await browser.close();
      await client.close();
      await stopMockServer();
    }
  });
}
