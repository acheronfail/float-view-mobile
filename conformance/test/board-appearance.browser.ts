import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';
import { defaultBoardAppearance } from '../../frontend/src/lib/backend/contract.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: appearance saves per board and renders on cards`, { timeout: 90_000 }, async () => {
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
      const id = ready.activeConnection!.savedDeviceId!;
      const page = await browser.newPage({
        viewport: { width: 320, height: 740 },
        hasTouch: true,
        isMobile: true,
        reducedMotion: 'reduce',
      });
      page.setDefaultTimeout(5000);
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      const errors: string[] = [];
      page.on('pageerror', (e) => errors.push(e.message));
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.goto(`${endpoint.origin}/#/live`);
      const open = async () => {
        await page.getByRole('button', { name: 'Board actions', exact: true }).tap();
        await page.getByRole('button', { name: 'Configure board', exact: true }).tap();
        const preview = page.getByRole('button', { name: 'Board appearance Tap to customise', exact: true });
        await preview.waitFor();
        assert.equal(await preview.getAttribute('aria-haspopup'), 'dialog');
        assert.equal(await page.getByRole('region', { name: 'Board appearance', exact: true }).count(), 0);
        await preview.click({ button: 'right' });
        assert.equal(await preview.getAttribute('aria-haspopup'), 'dialog');
        await preview.tap();
        await page.getByRole('region', { name: 'Board appearance', exact: true }).waitFor();
      };
      const saved = async (rail: number) => {
        for (let i = 0; i < 100; i++) {
          const p = await client.request('boardProfile.get', { savedDeviceId: id });
          if (p.values.appearance?.rail === rail) return p.values.appearance;
          await page.waitForTimeout(30);
        }
        assert.fail('Appearance did not save');
      };
      await open();
      const editor = page.getByRole('region', { name: 'Board appearance', exact: true });
      const next = editor.getByRole('button', { name: 'Next rails option', exact: true });
      assert.equal(await editor.getByText('1 / 6', { exact: true }).count(), 1);
      await next.click({ button: 'right' });
      assert.equal(await editor.getByText('1 / 6', { exact: true }).count(), 1);
      await next.tap();
      assert.equal(await editor.getByText('2 / 6', { exact: true }).count(), 1);
      await next.focus();
      await page.keyboard.press('Enter');
      assert.equal(await editor.getByText('3 / 6', { exact: true }).count(), 1);
      await editor.getByRole('button', { name: 'Red', exact: true }).tap();
      let appearance = await saved(3);
      for (let i = 0; i < 100 && appearance.leftTop !== '#ed5555'; i++) {
        await page.waitForTimeout(30);
        appearance = await saved(3);
      }
      assert.equal(appearance.leftTop, '#ed5555');
      assert.equal(appearance.rightTop, defaultBoardAppearance.rightTop);
      assert.equal(await editor.evaluate((el) => el.scrollWidth <= el.clientWidth), true);
      const svg = await editor.locator('[data-board-icon]').innerHTML();
      assert.ok(svg.includes('#ed5555'));
      const appearanceDialog = page.getByRole('dialog', { name: 'Board appearance', exact: true });
      await editor.getByRole('button', { name: 'Sand', exact: true }).scrollIntoViewIfNeeded();
      const collapse = appearanceDialog.getByRole('button', { name: 'Done', exact: true });
      const doneBounds = await collapse.boundingBox();
      assert.ok(doneBounds && doneBounds.y >= 0 && doneBounds.y + doneBounds.height <= 740);
      await collapse.focus();
      await page.keyboard.press('Enter');
      const preview = page.getByRole('button', { name: 'Board appearance Tap to customise', exact: true });
      assert.equal(await preview.getAttribute('aria-haspopup'), 'dialog');
      await appearanceDialog.waitFor({ state: 'detached' });
      assert.equal(await editor.count(), 0);
      assert.equal(await preview.evaluate((el) => el === document.activeElement), true);
      assert.ok((await preview.locator('[data-board-icon]').innerHTML()).includes('#ed5555'));
      await page
        .getByRole('dialog', { name: 'Board configuration', exact: true })
        .getByRole('button', { name: 'Done', exact: true })
        .tap();
      await open();
      assert.equal(await editor.getByText('3 / 6', { exact: true }).count(), 1);
      assert.equal(await editor.getByRole('button', { name: 'Red', exact: true }).getAttribute('aria-pressed'), 'true');
      await page.reload();
      await page.getByRole('button', { name: 'Board appearance Tap to customise', exact: true }).tap();
      await editor.waitFor();
      assert.equal(await editor.getByText('3 / 6', { exact: true }).count(), 1);
      await page.setViewportSize({ width: 1024, height: 768 });
      await page.keyboard.press('Escape');
      await appearanceDialog.waitFor({ state: 'detached' });
      await page.keyboard.press('Enter');
      await appearanceDialog.waitFor();
      await page.evaluate(() => window.dispatchEvent(new Event('floatview:back', { cancelable: true })));
      await appearanceDialog.waitFor({ state: 'detached' });
      assert.equal(await preview.evaluate((el) => el === document.activeElement), true);
      await page
        .getByRole('dialog', { name: 'Board configuration', exact: true })
        .getByRole('button', { name: 'Done', exact: true })
        .tap();
      await client.request('device.disconnect', {});
      await page.goto(`${endpoint.origin}/#/sessions`);
      const boardCard = page.locator(`a[href="#/sessions/boards/${id}"]`);
      await boardCard.waitFor();
      assert.ok((await boardCard.locator('[data-board-icon]').innerHTML()).includes('#ed5555'));
      await page.goto(`${endpoint.origin}/#/sessions/boards/${id}`);
      const sessions = page.getByRole('region', { name: 'Sessions', exact: true });
      await sessions.locator('[data-board-icon]').first().waitFor();
      assert.ok((await sessions.locator('[data-board-icon]').first().innerHTML()).includes('#ed5555'));
      await page.getByRole('link', { name: 'Sessions', exact: true }).tap();
      await page.getByRole('navigation', { name: 'Primary' }).getByRole('link', { name: 'connect', exact: true }).tap();
      const discovered = page.getByRole('region', { name: 'VESC devices' });
      await discovered.locator('[data-board-icon]').first().waitFor();
      assert.ok((await discovered.locator('[data-board-icon]').first().innerHTML()).includes('#ed5555'));
      assert.deepEqual(errors, []);
    } finally {
      await browser.close();
      await client.close();
      await stopMockServer();
    }
  });
}
