import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import path from 'node:path';
import { test } from 'node:test';
import { chromium, webkit, type Locator } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';
import { closedSideLayout } from './support/closed-side-layout.ts';
import type { HostLayout } from '../../frontend/src/lib/app/host-layout.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
const output = path.resolve('generated/side-controls');
const viewport = { width: 466, height: 678 };

async function insideSide(control: Locator, x = 386) {
  const bounds = await control.boundingBox();
  assert.ok(bounds);
  assert.ok(bounds.x >= x - 1 && bounds.x + bounds.width <= x + 77, JSON.stringify(bounds));
  assert.ok(bounds.y >= 181 && bounds.y + bounds.height <= 637, JSON.stringify(bounds));
  assert.ok(bounds.width >= 44 && bounds.height >= 44, JSON.stringify(bounds));
  assert.equal(
    await control.evaluate((element) => {
      const r = element.getBoundingClientRect();
      const hit = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2);
      return hit === element || element.contains(hit);
    }),
    true,
    'control is reachable at its visible centre'
  );
}

for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: measured side tabs, live actions, back links and fallback`, { timeout: 90_000 }, async () => {
    const port = await startMockServer(0, { safetyDisclaimerAccepted: true });
    const endpoint = getMockBootstrap(port);
    const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
    const browser = await engine.launch();
    try {
      await client.request('bluetooth.requestPermission', {});
      await client.request('bluetooth.scan', { timeoutMs: 100 });
      const state = await client.waitForState((s) => s.discoveredDevices.length > 0);
      await client.request('device.connect', { deviceId: state.discoveredDevices[0]!.deviceId, trackingMode: 'saved' });
      await client.waitForState((s) => s.activeConnection?.state === 'ready' && !!s.activeConnection.latestSample);
      const page = await browser.newPage({
        viewport,
        deviceScaleFactor: 1,
        hasTouch: true,
        isMobile: true,
        reducedMotion: 'reduce',
        colorScheme: 'dark',
      });
      page.setDefaultTimeout(7000);
      const errors: string[] = [];
      let sockets = 0;
      page.on('pageerror', (error) => errors.push(error.message));
      page.on('websocket', () => sockets++);
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      await page.addInitScript((layout) => {
        window.floatViewHostLayout = layout;
      }, closedSideLayout);
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.route(`${endpoint.origin}/api/bootstrap`, (route) =>
        route.fulfill({ json: { webSocketUrl: endpoint.webSocketUrl, platform: 'web' } })
      );
      await page.goto(`${endpoint.origin}/#/sessions`);
      await page.getByRole('heading', { name: 'Sessions', exact: true }).waitFor();
      await page.waitForFunction(() => document.documentElement.dataset.controlsEdge === 'right');
      const update = async (value: HostLayout | null) =>
        page.evaluate((layout) => {
          window.floatViewHostLayout = layout;
          window.dispatchEvent(new Event('floatview:layout'));
        }, value);
      const nav = page.getByRole('navigation', { name: 'Primary' });
      for (const link of await nav.getByRole('link').all()) await insideSide(link);
      const content = await page.locator('.app-content').boundingBox();
      assert.ok(content && content.x + content.width <= 383, JSON.stringify(content));
      assert.ok(
        (await page.locator('.app-content').evaluate((el) => parseFloat(getComputedStyle(el).paddingBottom))) < 50,
        'bottom tab reservation is released'
      );
      await mkdir(output, { recursive: true });
      await page.screenshot({ path: `${output}/${name}-sessions.png` });

      const left = {
        ...structuredClone(closedSideLayout),
        edge: 'left' as const,
        safeArea: { top: 0, right: 0, bottom: 34, left: 84 },
        occlusions: [{ x: 0, y: 0, width: 84, height: 174 }],
      };
      await update(left);
      await page.waitForFunction(() => document.documentElement.dataset.controlsEdge === 'left');
      for (const link of await nav.getByRole('link').all()) await insideSide(link, 4);
      await update(closedSideLayout);
      await page.locator('a[href^="#/sessions/boards/"]').first().click();
      await insideSide(page.locator('.app-back-control'));
      const boardMenu = page.getByRole('button', { name: 'Board actions', exact: true });
      await insideSide(boardMenu);
      await boardMenu.click();
      const menuPanel = page.locator('[data-header-menu]');
      await menuPanel.waitFor();
      const menuBounds = await menuPanel.boundingBox();
      assert.ok(menuBounds && menuBounds.y + menuBounds.height <= 637, JSON.stringify(menuBounds));
      assert.equal(
        await page.getByRole('button', { name: 'Configure board', exact: true }).evaluate((el) => {
          const r = el.getBoundingClientRect();
          return el.contains(document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2));
        }),
        true,
        'side header does not clip its menu to the narrow control column'
      );
      await page.keyboard.press('Escape');
      await menuPanel.waitFor({ state: 'hidden' });
      await page.locator('.app-back-control').focus();
      await page.keyboard.press('Enter');
      await page.getByRole('heading', { name: 'Sessions', exact: true }).waitFor();

      const settings = nav.getByRole('link', { name: 'settings', exact: true });
      await settings.click({ button: 'right' });
      assert.equal(new URL(page.url()).hash, '#/sessions', 'non-primary activation does not navigate');
      await page.keyboard.press('Escape');
      await settings.tap();
      await page.getByRole('heading', { name: 'Settings', exact: true }).waitFor();
      await page.getByRole('link', { name: 'View licences and notices' }).click();
      const back = page.locator('.app-back-control');
      await back.waitFor();
      await insideSide(back);
      assert.equal(await back.count(), 1, 'one existing back link');
      await page.screenshot({ path: `${output}/${name}-back.png` });
      await back.focus();
      await update(null);
      await page.waitForFunction(() => !document.documentElement.dataset.controlsEdge);
      assert.equal(
        await back.evaluate((el) => el === document.activeElement),
        true,
        'geometry fallback preserves focus'
      );
      assert.ok((await back.boundingBox())!.x < 100, 'fallback back link returns to content header');
      await update(closedSideLayout);
      await page.waitForFunction(() => document.documentElement.dataset.controlsEdge === 'right');
      await page.keyboard.press('Enter');
      await page.getByRole('heading', { name: 'Settings', exact: true }).waitFor();

      await page.getByRole('navigation', { name: 'Primary' }).getByRole('link', { name: 'board', exact: true }).click();
      await page.getByRole('region', { name: 'Live board widgets', exact: true }).waitFor();
      assert.equal(await page.getByRole('navigation', { name: 'Primary' }).count(), 0);
      const actions = page.locator('.app-page-header > .app-header-actions > button');
      assert.ok((await actions.count()) >= 1);
      for (const action of await actions.all()) await insideSide(action);
      await page.screenshot({ path: `${output}/${name}-live.png` });
      const menu = page.getByRole('button', { name: 'Board actions', exact: true });
      await menu.click({ button: 'right' });
      assert.equal(await menu.getAttribute('aria-expanded'), 'false');
      await page.keyboard.press('Escape');
      await menu.focus();
      await page.keyboard.press('Enter');
      const dialog = page.getByRole('dialog', { name: 'Board actions', exact: true });
      await dialog.waitFor();
      assert.equal(await menu.getAttribute('aria-expanded'), 'true');
      await page.keyboard.press('Escape');
      await dialog.waitFor({ state: 'hidden' });
      assert.equal(
        await menu.evaluate((el) => el === document.activeElement),
        true,
        'modal dismissal restores side trigger focus'
      );
      await menu.tap();
      await dialog.waitFor();
      await dialog.getByRole('button', { name: 'Done', exact: true }).click();
      await dialog.waitFor({ state: 'hidden' });

      const expanded = structuredClone(closedSideLayout);
      expanded.occlusions[1]!.height = 400;
      await update(expanded);
      await page.waitForFunction(() => !document.documentElement.dataset.controlsEdge);
      assert.ok((await menu.boundingBox())!.y < 100, 'insufficient side space falls back to the ordinary header');
      await update(closedSideLayout);
      await page.waitForFunction(() => document.documentElement.dataset.controlsEdge === 'right');
      await page.evaluate(() => (document.documentElement.style.fontSize = '32px'));
      await page.waitForFunction(() => !document.documentElement.dataset.controlsEdge);
      await page.evaluate(() => document.documentElement.style.removeProperty('font-size'));
      await page.waitForFunction(() => document.documentElement.dataset.controlsEdge === 'right');
      await page.setViewportSize({ width: 390, height: 844 });
      await page.waitForFunction(() => !document.documentElement.dataset.controlsEdge);
      await update(null);
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
      assert.equal(sockets, 1, 'route and geometry changes retain one backend connection');
      assert.deepEqual(errors, []);
    } finally {
      await browser.close();
      client.close();
      await stopMockServer();
    }
  });
}
