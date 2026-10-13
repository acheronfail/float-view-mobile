import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: surface motion, immediate focus transfer and reduced motion`, { timeout: 60_000 }, async () => {
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
      const page = await browser.newPage({ viewport: { width: 390, height: 844 }, hasTouch: true });
      page.setDefaultTimeout(5000);
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      const errors: string[] = [];
      page.on('pageerror', (error) => errors.push(error.message));
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.goto(`${endpoint.origin}/#/live`);
      const trigger = page.getByRole('button', { name: 'Board actions', exact: true });
      const actions = page.getByRole('dialog', { name: 'Board actions', exact: true });
      await trigger.waitFor();
      await page.evaluate(
        () => new Promise<void>((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve())))
      );
      for (const reducedMotion of ['no-preference', 'reduce'] as const) {
        await page.emulateMedia({ reducedMotion });
        await trigger.focus();
        await page.keyboard.press('Enter');
        await actions.waitFor();
        assert.equal(await actions.evaluate((node) => node.contains(document.activeElement)), true);
        const animated = await actions.evaluate((node) =>
          node.getAnimations().some((animation) => Number(animation.effect?.getTiming().duration) === 220)
        );
        assert.equal(animated, reducedMotion === 'no-preference');
        // The menu closes before the feature surface opens; no second active modal.
        await actions.getByRole('button', { name: 'Configure board', exact: true }).focus();
        await page.keyboard.press('Enter');
        const panel = page.getByRole('dialog', { name: 'Board configuration', exact: true });
        await panel.waitFor();
        assert.equal(await actions.isVisible(), false);
        const done = panel.getByRole('button', { name: 'Done', exact: true });
        await page.waitForFunction(() => document.activeElement?.textContent?.trim() === 'Done');
        assert.equal(await done.evaluate((node) => node === document.activeElement), true);
        const panelAnimated = await panel.evaluate((node) => node.getAnimations().length > 0);
        assert.equal(panelAnimated, reducedMotion === 'no-preference', 'feature panels use the shared motion policy');
        await page.keyboard.press('Escape');
        // Accessibility and keyboard focus transfer before the old pixels finish fading.
        await panel.waitFor({ state: 'hidden' });
        assert.equal(await trigger.evaluate((node) => node === document.activeElement), true);
        await page.keyboard.press('Enter');
        await actions.waitFor();
        await page.keyboard.press('Escape');
        await actions.waitFor({ state: 'hidden' });
      }
      await page.emulateMedia({ reducedMotion: 'no-preference' });
      await trigger.focus();
      await page.keyboard.press('Enter');
      await actions.getByRole('button', { name: 'Configure Refloat', exact: true }).focus();
      await page.keyboard.press('Enter');
      const editor = page.getByRole('dialog', { name: 'Configure Refloat', exact: true });
      const search = editor.getByRole('searchbox');
      await search.fill('Duty');
      const disclosure = editor.getByRole('button', { name: 'Section and search controls', exact: true });
      await disclosure.focus();
      await page.keyboard.press('Enter');
      assert.equal(await disclosure.getAttribute('aria-expanded'), 'false');
      assert.equal(await disclosure.evaluate((node) => node === document.activeElement), true);
      // Reverse the disclosure while its exit is still running.
      await page.keyboard.press('Enter');
      await search.waitFor();
      assert.equal(await search.inputValue(), 'Duty');
      assert.equal(await search.evaluate((node) => !!node.closest('[inert]')), false);
      await editor.getByRole('spinbutton').first().fill('0.85');
      const write = editor.getByRole('button', { name: 'Write configuration', exact: true });
      await page.waitForFunction((node) => !(node as HTMLButtonElement).disabled, await write.elementHandle());
      await write.focus();
      await page.keyboard.press('Enter');
      const confirmation = page.getByRole('alertdialog', {
        name: 'Write persistent Refloat configuration?',
        exact: true,
      });
      await confirmation.waitFor();
      await page.keyboard.press('Escape');
      await confirmation.waitFor({ state: 'hidden' });
      // Reopen before the old confirmation's visual exit finishes.
      await page.waitForFunction((node) => !(node as HTMLButtonElement).disabled, await write.elementHandle());
      await write.focus();
      await page.keyboard.press('Enter');
      await confirmation.waitFor();
      assert.equal(await confirmation.count(), 1);
      await page.keyboard.press('Escape');
      await confirmation.waitFor({ state: 'hidden' });
      assert.equal(await editor.isVisible(), true);
      await editor.getByRole('button', { name: 'Done', exact: true }).focus();
      await page.keyboard.press('Enter');
      await editor.waitFor({ state: 'hidden' });
      // Exercise rapid tab changes without retaining inert content or recreating a stale modal.
      await page.goto(`${endpoint.origin}/#/settings`);
      await page.getByRole('group', { name: 'Theme', exact: true }).waitFor();
      await page.getByRole('link', { name: 'sessions', exact: true }).click();
      await page.getByRole('link', { name: 'settings', exact: true }).click();
      await page.getByRole('group', { name: 'Theme', exact: true }).waitFor();
      assert.equal(
        await page.getByRole('group', { name: 'Theme', exact: true }).evaluate((node) => !!node.closest('[inert]')),
        false
      );
      assert.deepEqual(errors, []);
    } finally {
      client.close();
      await browser.close();
      await stopMockServer();
    }
  });
}
