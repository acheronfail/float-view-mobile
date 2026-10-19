import assert from 'node:assert/strict';
import { readFile, mkdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(
    `${name}: Tune presets save reliably and switch immediately by touch or keyboard`,
    { timeout: 60000 },
    async () => {
      const port = await startMockServer(11002, { safetyDisclaimerAccepted: true });
      const endpoint = getMockBootstrap(port);
      const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
      const browser = await engine.launch();
      try {
        await client.request('bluetooth.requestPermission', {});
        await client.request('bluetooth.scan', { timeoutMs: 100 });
        const scan = await client.waitForState((s) => s.discoveredDevices.length > 0);
        await client.request('device.connect', {
          deviceId: scan.discoveredDevices[0]!.deviceId,
          trackingMode: 'saved',
        });
        await client.waitForState((s) => s.activeConnection?.state === 'ready');
        const context = await browser.newContext({ viewport: { width: 390, height: 700 }, hasTouch: true });
        if (name === 'chromium') await context.grantPermissions(['local-network-access'], { origin: endpoint.origin });
        await context.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
        let failMethod: string | null = null;
        let creates = 0;
        let applies = 0;
        let updates = 0;
        const reads: Array<{ source: string; refresh?: boolean }> = [];
        await context.routeWebSocket('**/api/ws?*', (socket) => {
          const server = socket.connectToServer();
          server.onMessage((raw) => socket.send(raw));
          socket.onMessage((raw) => {
            const request = JSON.parse(String(raw));
            if (request.method === 'refloat.configuration.read') reads.push(request.params);
            if (request.method === 'refloat.tune.create') creates++;
            if (request.method === 'refloat.tune.apply') applies++;
            if (request.method === 'refloat.tune.update') updates++;
            if (request.method === failMethod) {
              socket.send(
                JSON.stringify({
                  id: request.id,
                  error: { code: 'invalid_state', message: 'Preset test failure' },
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
        await page.getByRole('button', { name: 'Tunes…', exact: true }).tap();
        await page.getByText('No saved tunes.', { exact: true }).waitFor();
        const screenshots = new URL('../generated/tune-library/', import.meta.url);
        await mkdir(screenshots, { recursive: true });
        await page.screenshot({
          animations: 'disabled',
          path: fileURLToPath(new URL(`${name}-empty-list.png`, screenshots)),
        });
        await page.getByRole('button', { name: 'Save tune', exact: true }).tap();
        const dialog = page.getByRole('alertdialog', { name: 'Save current tune' });
        await dialog.getByRole('textbox', { name: 'Name', exact: true }).fill('Base');
        const save = dialog.getByRole('button', { name: 'Save tune', exact: true });
        await save.dispatchEvent('pointerdown', { button: 0, isPrimary: false, pointerId: 2 });
        await save.dispatchEvent('pointerup', { button: 0, isPrimary: false, pointerId: 2 });
        assert.equal(creates, 0);
        for (const method of ['refloat.configuration.read', 'refloat.tune.create']) {
          failMethod = method;
          await save.tap();
          await dialog.getByRole('alert').waitFor();
          assert.equal(await dialog.getByRole('alert').textContent(), 'Preset test failure');
          await page.waitForFunction(() =>
            Array.from(document.querySelectorAll('[role="alertdialog"] button')).some(
              (button) => button.textContent === 'Save tune' && !button.hasAttribute('disabled')
            )
          );
          assert.equal(await dialog.getByRole('textbox', { name: 'Name', exact: true }).inputValue(), 'Base');
        }
        failMethod = null;
        await save.focus();
        await page.keyboard.press('Enter');
        await dialog.waitFor({ state: 'hidden' });
        await page.getByRole('button', { name: /^Base.*Active/ }).waitFor();
        assert.equal(creates, 2, 'keyboard/touch activation duplicated a save');
        assert.ok(reads.every((read) => read.source === 'current' && read.refresh === true));
        await page.reload();
        await page.getByRole('button', { name: /^Base/ }).waitFor();

        const current = await client.request('refloat.configuration.read', { source: 'current', refresh: true });
        const listed = await client.request('refloat.tune.list', { identity: current.identity, limit: 50 });
        const base = await client.request('refloat.tune.get', { tuneId: listed.items[0]!.tuneId });
        await client.request('refloat.tune.create', {
          name: 'Cruise',
          description: null,
          identity: current.identity,
          sourceRevision: current.revision,
          values: { ...base.values, 'leds.on': { type: 'boolean', value: false } },
        });
        await client.waitForState((state) => state.activeConnection?.latestSample?.rideState === 'riding');
        await page.reload();
        const cruise = page.getByRole('button', { name: /^Cruise/ });
        await cruise.waitFor();
        await cruise.dispatchEvent('pointerdown', { button: 0, isPrimary: false, pointerId: 2 });
        await cruise.dispatchEvent('pointerup', { button: 0, isPrimary: false, pointerId: 2 });
        assert.equal(applies, 0);
        await cruise.scrollIntoViewIfNeeded();
        const bounds = await cruise.boundingBox();
        assert.ok(bounds);
        const x = bounds.x + bounds.width / 2;
        const y = bounds.y + bounds.height / 2;
        await cruise.evaluate((element) => {
          element.addEventListener(
            'pointerdown',
            (event) => {
              element.setAttribute('data-test-pointer-id', String((event as PointerEvent).pointerId));
            },
            { once: true }
          );
        });
        await page.mouse.move(x, y);
        await page.mouse.down();
        await cruise.dispatchEvent('pointercancel', {
          pointerId: Number(await cruise.getAttribute('data-test-pointer-id')),
        });
        await page.mouse.up();
        assert.equal(applies, 0);
        await page.mouse.move(x, y);
        await page.mouse.down();
        await page.mouse.move(x, y + 25);
        await page.mouse.up();
        assert.equal(applies, 0, 'scrolling must not apply a Tune');
        await cruise.focus();
        await page.keyboard.press('Enter');
        await page.getByRole('status').filter({ hasText: 'Cruise applied.' }).waitFor();
        assert.equal(applies, 1, 'keyboard activation must apply a Tune once while riding');
        assert.equal(await page.getByRole('alertdialog').count(), 0);
        assert.equal(await page.getByRole('button', { name: 'Apply preset', exact: true }).count(), 0);
        const applied = await client.request('refloat.configuration.read', { source: 'current', refresh: true });
        assert.deepEqual(applied.values['leds.on'], { type: 'boolean', value: false });
        await cruise.tap();
        await page.getByRole('button', { name: /^Cruise.*Active/ }).waitFor();
        assert.equal(applies, 1, 'tapping the active Tune must not rewrite it');
        await page.getByRole('button', { name: /^Base/ }).tap();
        await page.getByRole('status').filter({ hasText: 'Base applied.' }).waitFor();
        assert.equal(applies, 2, 'one touch must switch back to the saved Tune');
        failMethod = 'refloat.tune.apply';
        await cruise.tap();
        await page.getByRole('dialog', { name: 'Tunes', exact: true }).getByRole('alert').waitFor();
        assert.equal(applies, 3);
        assert.equal(await page.getByRole('button', { name: /^Cruise.*Active/ }).count(), 0);
        failMethod = null;
        await cruise.tap();
        await page.getByRole('status').filter({ hasText: 'Cruise applied.' }).waitFor();
        assert.equal(applies, 4, 'a failed switch must be retryable with one tap');
        await page.getByRole('button', { name: 'Options for Base', exact: true }).tap();
        await page.getByRole('button', { name: 'Overwrite', exact: true }).tap();
        const overwrite = page.getByRole('alertdialog', { name: 'Overwrite Base?', exact: true });
        await overwrite.getByRole('button', { name: 'Cancel', exact: true }).tap();
        await overwrite.waitFor({ state: 'hidden' });
        assert.equal(updates, 0);
        await page.getByRole('button', { name: 'Options for Base', exact: true }).tap();
        await page.getByRole('button', { name: 'Overwrite', exact: true }).tap();
        const confirmOverwrite = overwrite.getByRole('button', { name: 'Overwrite', exact: true });
        await confirmOverwrite.dispatchEvent('pointerdown', { button: 0, isPrimary: false, pointerId: 2 });
        await confirmOverwrite.dispatchEvent('pointerup', { button: 0, isPrimary: false, pointerId: 2 });
        assert.equal(updates, 0);
        for (const method of ['refloat.configuration.read', 'refloat.tune.update']) {
          failMethod = method;
          await confirmOverwrite.tap();
          await overwrite.getByRole('alert').waitFor();
          assert.equal(await overwrite.getByRole('alert').textContent(), 'Preset test failure');
          assert.deepEqual(await client.request('refloat.tune.get', { tuneId: base.tuneId }), base);
        }
        failMethod = null;
        await confirmOverwrite.focus();
        await page.keyboard.press('Enter');
        await overwrite.waitFor({ state: 'hidden' });
        await page.getByRole('button', { name: /^Base.*Active/ }).waitFor();
        const overwritten = await client.request('refloat.tune.get', { tuneId: base.tuneId });
        assert.equal(overwritten.revision, base.revision + 1);
        assert.equal(overwritten.name, base.name);
        assert.equal(overwritten.description, base.description);
        assert.deepEqual(overwritten.identity, base.identity);
        assert.deepEqual(overwritten.values['leds.on'], { type: 'boolean', value: false });
        assert.equal(creates, 2, 'overwrite must not create a new tune');
        assert.equal(updates, 2, 'one failed and one successful overwrite should be sent');
        assert.equal(applies, 4, 'overwriting a preset must not write to the board');
        const savedMessage = page.getByRole('status').filter({ hasText: 'Base saved.' });
        await savedMessage.waitFor();
        const messageBounds = await savedMessage.boundingBox();
        const saveButtonBounds = await page.getByRole('button', { name: 'Save tune', exact: true }).boundingBox();
        assert.ok(messageBounds && saveButtonBounds);
        assert.ok(
          saveButtonBounds.y - messageBounds.y - messageBounds.height >= 11,
          'save feedback must have space below it'
        );
        await page.locator('[role="alertdialog"]').waitFor({ state: 'detached' });
        await page.screenshot({
          animations: 'disabled',
          path: fileURLToPath(new URL(`${name}-overwrite-saved.png`, screenshots)),
        });
        await page.getByRole('button', { name: 'Options for Cruise', exact: true }).tap();
        await page.getByRole('button', { name: 'Rename', exact: true }).tap();
        const renameDialog = page.getByRole('alertdialog', { name: 'Rename tune', exact: true });
        await renameDialog.getByRole('textbox', { name: 'Name', exact: true }).fill('Renamed Cruise');
        failMethod = 'refloat.tune.update';
        await renameDialog.getByRole('button', { name: 'Save', exact: true }).tap();
        await renameDialog.getByRole('alert').waitFor();
        assert.equal(await renameDialog.getByRole('alert').textContent(), 'Preset test failure');
        assert.equal(
          await renameDialog.getByRole('textbox', { name: 'Name', exact: true }).inputValue(),
          'Renamed Cruise'
        );
        failMethod = null;
        await renameDialog.getByRole('button', { name: 'Save', exact: true }).focus();
        await page.keyboard.press('Enter');
        await renameDialog.waitFor({ state: 'hidden' });
        await page.getByRole('button', { name: 'Options for Renamed Cruise', exact: true }).tap();
        await page.getByRole('button', { name: 'Delete', exact: true }).tap();
        const deleteDialog = page.getByRole('alertdialog', { name: 'Delete Renamed Cruise?', exact: true });
        failMethod = 'refloat.tune.delete';
        await deleteDialog.getByRole('button', { name: 'Delete tune', exact: true }).tap();
        await deleteDialog.getByRole('alert').waitFor();
        assert.equal(await deleteDialog.getByRole('alert').textContent(), 'Preset test failure');
        await deleteDialog.getByRole('button', { name: 'Cancel', exact: true }).tap();
        await deleteDialog.waitFor({ state: 'hidden' });
        await page.getByRole('button', { name: 'Options for Renamed Cruise', exact: true }).tap();
        await page.getByRole('button', { name: 'Delete', exact: true }).tap();
        assert.equal(await deleteDialog.getByRole('alert').count(), 0);
        failMethod = null;
        await deleteDialog.getByRole('button', { name: 'Delete tune', exact: true }).tap();
        await deleteDialog.waitFor({ state: 'hidden' });
        await page.getByRole('button', { name: /^Renamed Cruise/ }).waitFor({ state: 'hidden' });
        assert.equal(applies, 4, 'card menu actions must never apply a tune');
        assert.deepEqual(errors, []);
      } finally {
        await browser.close();
        client.close();
        await stopMockServer();
      }
    }
  );
}

for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: one tune list separates applying, compatibility, and card actions`, { timeout: 60000 }, async () => {
    const older = {
      tuneId: 'older-trail',
      revision: 1,
      name: 'Trail · 1.2',
      description: 'My favourite trail tune',
      identity: {
        packageName: 'refloat' as const,
        packageVersion: '1.2.3',
        schemaFingerprint: 'a'.repeat(64),
        wireSignature: 123,
      },
      values: { kp: { type: 'double' as const, value: 25 } },
      createdAt: '2026-10-01T00:00:00.000Z',
      updatedAt: '2026-10-01T00:00:00.000Z',
    };
    const port = await startMockServer(11002, {
      safetyDisclaimerAccepted: true,
      initialRefloatTunes: [
        older,
        {
          ...older,
          tuneId: 'alternate',
          name: 'Alternate schema',
          identity: { ...older.identity, packageVersion: '1.3.0' },
        },
      ],
    });
    const endpoint = getMockBootstrap(port);
    const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
    const browser = await engine.launch();
    try {
      await client.request('bluetooth.requestPermission', {});
      await client.request('bluetooth.scan', { timeoutMs: 100 });
      const scan = await client.waitForState((s) => s.discoveredDevices.length > 0);
      await client.request('device.connect', { deviceId: scan.discoveredDevices[0]!.deviceId, trackingMode: 'saved' });
      await client.waitForState((s) => s.activeConnection?.state === 'ready');
      const context = await browser.newContext({
        viewport: { width: 390, height: 844 },
        hasTouch: true,
        reducedMotion: 'reduce',
      });
      if (name === 'chromium') await context.grantPermissions(['local-network-access'], { origin: endpoint.origin });
      await context.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      let applies = 0;
      await context.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        server.onMessage((raw) => socket.send(raw));
        socket.onMessage((raw) => {
          if (JSON.parse(String(raw)).method === 'refloat.tune.apply') applies++;
          server.send(raw);
        });
      });
      const page = await context.newPage();
      page.setDefaultTimeout(5000);
      const errors: string[] = [];
      page.on('pageerror', (error) => errors.push(error.message));
      await page.goto(`${endpoint.origin}/`);
      await page.getByRole('button', { name: 'Board actions', exact: true }).tap();
      await page.getByRole('button', { name: 'Tunes…', exact: true }).tap();
      const oldCard = page.locator('[data-tune-id="older-trail"]');
      const oldButton = oldCard.locator(':scope > button');
      await oldButton.waitFor();
      assert.equal(await page.getByRole('button', { name: 'View all saved tunes' }).count(), 0);
      assert.equal(await oldCard.getAttribute('data-compatible'), 'false');
      assert.match((await oldButton.textContent()) ?? '', /Refloat 1.2.3.*Unavailable/s);
      const screenshots = new URL('../generated/tune-library/', import.meta.url);
      await mkdir(screenshots, { recursive: true });
      const capture = async (state: string) => {
        await page.locator('[role="alertdialog"]').waitFor({ state: 'detached' });
        await page.screenshot({
          animations: 'disabled',
          path: fileURLToPath(new URL(`${name}-${state}.png`, screenshots)),
        });
      };
      await capture('unavailable-list-mobile');
      const alternate = page.locator('[data-tune-id="alternate"] > button');
      await alternate.tap();
      const info = page.getByRole('dialog', { name: 'Tune unavailable', exact: true });
      await info
        .getByText('This tune uses a different configuration and cannot be applied to this board.', { exact: true })
        .waitFor();
      await info.getByRole('button', { name: 'Done', exact: true }).tap();
      await info.waitFor({ state: 'hidden' });
      await oldButton.dispatchEvent('pointerdown', { button: 0, isPrimary: false, pointerId: 2 });
      await oldButton.dispatchEvent('pointerup', { button: 0, isPrimary: false, pointerId: 2 });
      assert.equal(await info.count(), 0);
      await oldButton.focus();
      await page.keyboard.press('Enter');
      await info.getByText('Requires Refloat 1.2.3. This board uses 1.3.0.', { exact: true }).waitFor();
      await capture('incompatible-dialog-mobile');
      assert.equal(applies, 0);
      await info.getByRole('button', { name: 'Done', exact: true }).tap();
      await info.waitFor({ state: 'hidden' });
      await page.getByRole('button', { name: 'Options for Alternate schema', exact: true }).tap();
      await page.getByRole('button', { name: 'Delete', exact: true }).tap();
      const deleteAlternate = page.getByRole('alertdialog', { name: 'Delete Alternate schema?', exact: true });
      await deleteAlternate.getByRole('button', { name: 'Delete tune', exact: true }).tap();
      await deleteAlternate.waitFor({ state: 'hidden' });
      await page.getByRole('button', { name: 'Save tune', exact: true }).tap();
      const save = page.getByRole('alertdialog', { name: 'Save current tune', exact: true });
      await save.getByRole('textbox', { name: 'Name', exact: true }).fill('Everyday · 1.3');
      await save.getByRole('button', { name: 'Save tune', exact: true }).tap();
      await save.waitFor({ state: 'hidden' });
      await page.getByRole('button', { name: /^Everyday · 1.3.*Active/ }).waitFor();
      const rows = page.locator('[data-tune-id]');
      assert.equal(await rows.first().getAttribute('data-compatible'), 'true');
      assert.equal(await rows.last().getAttribute('data-compatible'), 'false');
      await capture('cards-mobile');
      const options = page.getByRole('button', { name: 'Options for Trail · 1.2', exact: true });
      const cardBounds = await oldCard.boundingBox();
      const optionsBounds = await options.boundingBox();
      assert.ok(cardBounds && optionsBounds);
      assert.ok(optionsBounds.width >= 64, 'card menu needs a generous tap target');
      assert.ok(Math.abs(optionsBounds.height - cardBounds.height) <= 2, 'options must span the full card height');
      assert.ok(Math.abs(optionsBounds.x + optionsBounds.width - cardBounds.x - cardBounds.width) <= 2);
      await options.dispatchEvent('pointerdown', { button: 0, isPrimary: false, pointerId: 2 });
      await options.dispatchEvent('pointerup', { button: 0, isPrimary: false, pointerId: 2 });
      assert.equal(await options.getAttribute('aria-expanded'), 'false');
      await options.focus();
      await page.keyboard.press('Enter');
      await page.getByRole('button', { name: 'Rename', exact: true }).waitFor();
      assert.equal(await page.getByRole('button', { name: 'Overwrite', exact: true }).isDisabled(), true);
      await capture('card-menu-mobile');
      assert.equal(applies, 0);
      await page.keyboard.press('Escape');
      assert.equal(await options.getAttribute('aria-expanded'), 'false');
      assert.equal(await options.evaluate((element) => element === document.activeElement), true);
      await options.tap({ position: { x: 4, y: optionsBounds.height / 2 } });
      assert.equal(await options.getAttribute('aria-expanded'), 'true');
      assert.equal(applies, 0, 'a tap near the divider must open the menu without applying the tune');
      await page.keyboard.press('Escape');
      await page.setViewportSize({ width: 1280, height: 900 });
      await capture('cards-desktop');
      await options.click();
      await page.getByRole('button', { name: 'Rename', exact: true }).click();
      const rename = page.getByRole('alertdialog', { name: 'Rename tune', exact: true });
      await rename.getByRole('textbox', { name: 'Name', exact: true }).fill('Trail archive');
      await rename.getByRole('button', { name: 'Save', exact: true }).click();
      await rename.waitFor({ state: 'hidden' });
      const saved = await client.request('refloat.tune.get', { tuneId: older.tuneId });
      assert.equal(saved.name, 'Trail archive');
      assert.deepEqual(saved.identity, older.identity);
      assert.deepEqual(saved.values, older.values);
      assert.equal(applies, 0);
      await page.getByRole('button', { name: /^Everyday · 1.3.*Active/ }).waitFor();
      // The matching page must be loaded before newer incompatible library pages.
      const config = await client.request('refloat.configuration.read', { source: 'current', refresh: true });
      const library = await client.request('refloat.tune.list', { identity: null, limit: 50 });
      const everyday = await client.request('refloat.tune.get', {
        tuneId: library.items.find((tune) => tune.name === 'Everyday · 1.3')!.tuneId,
      });
      for (let index = 0; index < 51; index++)
        await client.request('refloat.tune.create', {
          name: `Pagination ${index}`,
          description: null,
          identity: config.identity,
          sourceRevision: config.revision,
          values: everyday.values,
        });
      // A metadata update makes the incompatible tune newest in the unfiltered library.
      await client.request('refloat.tune.update', {
        tuneId: saved.tuneId,
        expectedRevision: saved.revision,
        name: saved.name,
        description: saved.description,
        values: saved.values,
      });
      await page.reload();
      await page.locator('[data-tune-id][data-compatible="true"]').first().waitFor();
      assert.equal(await rows.first().getAttribute('data-compatible'), 'true');
      await rows.last().scrollIntoViewIfNeeded();
      await page.waitForFunction(() => document.querySelectorAll('[data-tune-id]').length === 53);
      const ids = await rows.evaluateAll((elements) => elements.map((element) => element.getAttribute('data-tune-id')));
      assert.equal(new Set(ids).size, 53);
      assert.equal(await rows.last().getAttribute('data-tune-id'), older.tuneId);
      await rows.last().scrollIntoViewIfNeeded();
      await page.getByRole('button', { name: 'Options for Trail archive', exact: true }).click();
      const renameBounds = await page.getByRole('button', { name: 'Rename', exact: true }).boundingBox();
      const deleteBounds = await page.getByRole('button', { name: 'Delete', exact: true }).boundingBox();
      assert.ok(renameBounds && deleteBounds);
      assert.ok(
        renameBounds.y >= 0 && deleteBounds.y + deleteBounds.height <= 900,
        'last-card menu must fit on screen'
      );
      assert.equal(applies, 0);
      assert.deepEqual(errors, []);
    } finally {
      await browser.close();
      client.close();
      await stopMockServer();
    }
  });
}
