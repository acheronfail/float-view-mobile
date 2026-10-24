import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
const screenshots = new URL('../generated/session-menu/', import.meta.url);
await mkdir(screenshots, { recursive: true });

for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(
    `${name}: session menu explains active, paused and busy actions and confirms ending`,
    { timeout: 60_000 },
    async () => {
      const browser = await engine.launch();
      const port = await startMockServer(0, { safetyDisclaimerAccepted: true });
      const endpoint = getMockBootstrap(port);
      const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
      try {
        await client.request('bluetooth.requestPermission', {});
        await client.request('bluetooth.scan', { timeoutMs: 100 });
        const scanned = await client.waitForState((state) => state.discoveredDevices.length > 0);
        await client.request('device.connect', {
          deviceId: scanned.discoveredDevices[0]!.deviceId,
          trackingMode: 'saved',
        });
        const ready = await client.waitForState((state) => state.activeConnection?.state === 'ready');
        const sessionId = ready.activeConnection!.activeSession!.sessionId;
        const page = await browser.newPage({
          viewport: { width: 390, height: 844 },
          hasTouch: true,
          colorScheme: 'light',
          reducedMotion: 'reduce',
        });
        page.setDefaultTimeout(10_000);
        if (name === 'chromium')
          await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
        const errors: string[] = [];
        page.on('pageerror', (error) => errors.push(error.message));
        let releaseSettings: (() => void) | undefined;
        let endRequests = 0;
        let settingsRequests = 0;
        let failEnd = false;
        await page.routeWebSocket('**/api/ws?*', (socket) => {
          const server = socket.connectToServer();
          socket.onMessage((raw) => {
            const message = JSON.parse(String(raw));
            if (message.method === 'settings.update') {
              settingsRequests++;
              releaseSettings = () => server.send(raw);
              return;
            }
            if (message.method === 'session.end') {
              endRequests++;
              if (failEnd) {
                failEnd = false;
                socket.send(
                  JSON.stringify({ id: message.id, error: { code: 'unavailable', message: 'Try ending again.' } })
                );
                return;
              }
            }
            server.send(raw);
          });
        });
        await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
        // Keep the authenticated development origin, while using an independent
        // ephemeral backend port so an existing development server can stay open.
        await page.route(`${endpoint.origin}/api/**`, async (route) => {
          const url = new URL(route.request().url());
          const response = await route.fetch({ url: `http://127.0.0.1:${port}${url.pathname}${url.search}` });
          if (url.pathname === '/api/bootstrap') {
            await route.fulfill({
              response,
              json: { ...(await response.json()), platform: 'ios', webSocketUrl: endpoint.webSocketUrl },
            });
          } else await route.fulfill({ response });
        });
        await page.route('https://tile.openstreetmap.org/**', (route) => route.abort());
        await page.goto(`${endpoint.origin}/#/sessions/${sessionId}`);
        const menu = page.getByRole('button', { name: 'Session actions', exact: true });
        async function assertDisabledActionsIgnoreActivation(labels: string[]) {
          for (const label of labels) {
            const button = page.getByRole('button', { name: label, exact: true });
            assert.equal(await button.isDisabled(), true);
            await button.scrollIntoViewIfNeeded();
            const bounds = (await button.boundingBox())!;
            await page.touchscreen.tap(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2);
            // Disabled buttons can still receive pointer events from their
            // descendants on WebKit. Exercise that route and assistive clicks.
            const child = button.locator('span').first();
            const pointer = { pointerId: 42, pointerType: 'touch', isPrimary: true, button: 0 };
            await child.dispatchEvent('pointerdown', pointer);
            await child.dispatchEvent('pointerup', pointer);
            await child.dispatchEvent('click', { detail: 0 });
            await page.evaluate(() => new Promise(requestAnimationFrame));
            assert.equal(await menu.getAttribute('aria-expanded'), 'true', `${label} keeps the menu open`);
            assert.equal(await page.getByRole('alertdialog').count(), 0, `${label} opens no confirmation`);
          }
        }
        await menu.tap();
        await page.getByText('End the ride for more options.', { exact: true }).waitFor();
        await page.getByRole('heading', { name: 'Chart display', exact: true }).waitFor();
        await page.getByRole('heading', { name: 'Session', exact: true }).waitFor();
        for (const label of ['Share Archive', 'Share CSV', 'Prune', 'Delete session'])
          assert.equal(await page.getByRole('button', { name: label, exact: true }).isDisabled(), true);
        const end = page.getByRole('button', { name: 'End ride…', exact: true });
        const explanation = page.getByText('End the ride for more options.', { exact: true });
        assert.ok((await end.boundingBox())!.y < (await explanation.boundingBox())!.y);
        const unavailableGroup = (await page.locator('[data-unavailable-actions]').boundingBox())!;
        const overlayBounds = (await explanation.boundingBox())!;
        assert.ok(
          Math.abs(overlayBounds.y + overlayBounds.height / 2 - unavailableGroup.y - unavailableGroup.height / 2) < 1
        );
        for (const label of ['Share Archive', 'Share CSV', 'Prune', 'Delete session']) {
          const reasonId = await page
            .getByRole('button', { name: label, exact: true })
            .getAttribute('aria-describedby');
          assert.equal(reasonId, await explanation.getAttribute('id'), 'disabled actions reference their explanation');
        }
        const disabledColors = await Promise.all(
          ['Share Archive', 'Share CSV', 'Prune', 'Delete session'].map((label) =>
            page.getByRole('button', { name: label, exact: true }).evaluate((button) => getComputedStyle(button).color)
          )
        );
        assert.equal(new Set(disabledColors).size, 1, 'disabled actions use a consistent muted colour');
        await page.screenshot({ path: fileURLToPath(new URL(`${name}-active.png`, screenshots)) });
        await assertDisabledActionsIgnoreActivation(['Delete session', 'Prune', 'Share Archive', 'Share CSV']);
        await end.click({ button: 'right' });
        assert.equal(await page.getByRole('alertdialog', { name: 'End ride?' }).count(), 0);
        await end.tap();
        const confirmation = page.getByRole('alertdialog', { name: 'End ride?' });
        await confirmation.waitFor();
        assert.match(await confirmation.innerText(), /disconnect the board/);
        await confirmation.getByRole('button', { name: 'Cancel', exact: true }).tap();
        assert.equal(endRequests, 0);
        assert.equal((await client.request('state.get', {})).activeConnection?.activeSession?.sessionId, sessionId);
        await menu.tap();
        await page.getByRole('switch', { name: 'Both footpads off at speed', exact: true }).tap();
        await page.getByText('Another operation is in progress. Please wait.', { exact: true }).waitFor();
        assert.equal(await end.isDisabled(), true);
        assert.ok(releaseSettings);
        releaseSettings();
        await page.getByText('End the ride for more options.', { exact: true }).waitFor();

        await client.request('device.disconnect', {});
        const pausedCopy = 'End the ride for more options.';
        await page.getByText(pausedCopy, { exact: true }).waitFor();
        await assertDisabledActionsIgnoreActivation(['Delete session', 'Prune', 'Share Archive', 'Share CSV']);
        await page.screenshot({ path: fileURLToPath(new URL(`${name}-paused.png`, screenshots)) });
        await page.emulateMedia({ colorScheme: 'dark' });
        await page.screenshot({ path: fileURLToPath(new URL(`${name}-paused-dark.png`, screenshots)) });
        await page.emulateMedia({ colorScheme: 'light' });
        const scroll = page.locator('[data-header-menu]');
        assert.equal(await page.getByRole('button', { name: /^Chart mode:/ }).isVisible(), false);
        for (const [label, width, height, textScale] of [
          ['narrow-phone', 320, 568, 1],
          ['landscape-phone', 844, 390, 1],
          ['large-text', 320, 568, 2],
          ['tablet', 768, 1024, 1],
          ['desktop', 1440, 900, 1],
        ] as const) {
          await page.setViewportSize({ width, height });
          await page.evaluate((scale) => {
            document.documentElement.style.fontSize = `${16 * scale}px`;
          }, textScale);
          await scroll.evaluate((node) => {
            node.scrollTop = 0;
          });
          const panel = await scroll.boundingBox();
          assert.ok(panel && panel.x >= 0 && panel.x + panel.width <= width, `${label}: fits width`);
          assert.ok(panel.y >= 0 && panel.y + panel.height <= height, `${label}: fits height`);
          assert.equal(await scroll.evaluate((node) => node.scrollWidth <= node.clientWidth), true);
          await page.screenshot({ path: fileURLToPath(new URL(`${name}-${label}.png`, screenshots)) });
          if (label === 'landscape-phone' && name === 'chromium') {
            const requestsBefore = settingsRequests;
            const smooth = page.getByRole('switch', { name: 'Smooth charts', exact: true });
            const checkedBefore = await smooth.getAttribute('aria-checked');
            const cdp = await page.context().newCDPSession(page);
            const x = panel.x + panel.width / 2;
            const startY = panel.y + 170;
            await cdp.send('Input.dispatchTouchEvent', {
              type: 'touchStart',
              touchPoints: [{ x, y: startY }],
            });
            for (let step = 1; step <= 12; step++) {
              await cdp.send('Input.dispatchTouchEvent', {
                type: 'touchMove',
                touchPoints: [{ x, y: startY - step * 10 }],
              });
              await page.waitForTimeout(16);
            }
            await cdp.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
            await page.waitForFunction(() => document.querySelector('[data-header-menu]')!.scrollTop > 20);
            const reason = (await page.locator('[data-unavailable-overlay]').boundingBox())!;
            const scrollBefore = await scroll.evaluate((node) => node.scrollTop);
            const overlayX = reason.x + reason.width / 2;
            const overlayY = Math.max(panel.y + 50, reason.y) + 10;
            await cdp.send('Input.dispatchTouchEvent', {
              type: 'touchStart',
              touchPoints: [{ x: overlayX, y: overlayY }],
            });
            for (let step = 1; step <= 10; step++) {
              await cdp.send('Input.dispatchTouchEvent', {
                type: 'touchMove',
                touchPoints: [{ x: overlayX, y: overlayY + step * 5 }],
              });
              await page.waitForTimeout(16);
            }
            await cdp.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
            await page.waitForFunction(
              (before) => document.querySelector('[data-header-menu]')!.scrollTop < before - 5,
              scrollBefore
            );
            assert.equal(settingsRequests, requestsBefore, 'swiping does not submit a setting');
            assert.equal(await smooth.getAttribute('aria-checked'), checkedBefore, 'swiping does not toggle charts');
            assert.equal(await confirmation.isVisible(), false, 'swiping does not end a ride');
            await cdp.detach();
            // Let touch momentum settle before positioning the menu for screenshots.
            await scroll.evaluate(async (node) => {
              let previous = node.scrollTop;
              let stableFrames = 0;
              for (let frame = 0; frame < 120 && stableFrames < 10; frame++) {
                await new Promise(requestAnimationFrame);
                const current = node.scrollTop;
                stableFrames = Math.abs(current - previous) < 0.5 ? stableFrames + 1 : 0;
                previous = current;
              }
            });
          }
          if (label === 'narrow-phone' || label === 'landscape-phone' || label === 'large-text') {
            await scroll.evaluate((node) => {
              node.scrollTop = node.scrollHeight;
            });
            const sessionHeading = await page.getByRole('heading', { name: 'Session', exact: true }).boundingBox();
            assert.ok(
              sessionHeading &&
                sessionHeading.y >= panel.y &&
                sessionHeading.y + sessionHeading.height <= panel.y + panel.height,
              'section heading stays visible'
            );
            const last = await page.getByRole('button', { name: 'Delete session', exact: true }).boundingBox();
            assert.ok(last && last.y >= panel.y && last.y + last.height <= panel.y + panel.height + 1);
            const reason = (await page.locator('[data-unavailable-overlay]').boundingBox())!;
            assert.ok(
              reason.y >= panel.y && reason.y + reason.height <= panel.y + panel.height + 1,
              'explanation stays visible'
            );
            await page.screenshot({ path: fileURLToPath(new URL(`${name}-${label}-scrolled.png`, screenshots)) });
          }
        }
        await page.evaluate(() => {
          document.documentElement.style.fontSize = '';
        });
        await page.setViewportSize({ width: 390, height: 568 });
        const deletion = page.getByRole('button', { name: 'Delete session', exact: true });
        await deletion.scrollIntoViewIfNeeded();
        const bounds = (await deletion.boundingBox())!;
        assert.ok(bounds.y >= 0 && bounds.y + bounds.height <= 568, 'menu scrolls on shorter screens');
        await page.setViewportSize({ width: 390, height: 844 });
        await end.focus();
        await page.keyboard.press('Enter');
        await confirmation.waitFor();
        assert.match(await confirmation.innerText(), /Reconnecting will start a new ride/);
        assert.doesNotMatch(await confirmation.innerText(), /disconnect the board/);
        await page.screenshot({ path: fileURLToPath(new URL(`${name}-confirmation.png`, screenshots)) });
        failEnd = true;
        await confirmation.getByRole('button', { name: 'End ride', exact: true }).tap();
        await confirmation.getByRole('alert').waitFor();
        assert.equal(endRequests, 1);
        await confirmation.getByRole('button', { name: 'End ride', exact: true }).focus();
        await page.keyboard.press('Enter');
        await confirmation.waitFor({ state: 'hidden' });
        assert.equal(endRequests, 2, 'each activation submits once');
        assert.equal(await page.getByRole('button', { name: /^Chart mode:/ }).isVisible(), true);
        await menu.tap();
        const share = page.getByRole('button', { name: 'Share Archive', exact: true });
        await page.waitForFunction(() => {
          const button = document.querySelector<HTMLButtonElement>('button[aria-label="Share Archive"]');
          return button && !button.disabled;
        });
        assert.equal(await share.isDisabled(), false);
        for (const label of ['Share CSV', 'Prune', 'Delete session'])
          assert.equal(await page.getByRole('button', { name: label, exact: true }).isDisabled(), false);
        assert.equal(await end.count(), 0);
        assert.equal(await page.getByText(pausedCopy, { exact: true }).count(), 0);
        await page.screenshot({ path: fileURLToPath(new URL(`${name}-completed.png`, screenshots)) });
        const deleteBounds = (await deletion.boundingBox())!;
        await page.mouse.move(deleteBounds.x + deleteBounds.width / 2, deleteBounds.y + deleteBounds.height / 2);
        await page.mouse.down();
        await page
          .getByRole('switch', { name: 'Both footpads off at speed', exact: true })
          .dispatchEvent('click', { detail: 0 });
        await page.getByText('Another operation is in progress. Please wait.', { exact: true }).waitFor();
        await page.mouse.up();
        await page.evaluate(() => new Promise(requestAnimationFrame));
        assert.equal(await menu.getAttribute('aria-expanded'), 'true', 'disabling during a press cancels activation');
        assert.equal(await page.getByRole('alertdialog').count(), 0);
        assert.equal(await share.isDisabled(), true);
        await assertDisabledActionsIgnoreActivation(['Delete session', 'Prune', 'Share Archive', 'Share CSV']);
        assert.equal(await page.getByText('End the ride for more options.', { exact: true }).count(), 0);
        releaseSettings();
        await page
          .getByText('Another operation is in progress. Please wait.', { exact: true })
          .waitFor({ state: 'hidden' });
        assert.equal(await share.isDisabled(), false);
        await deletion.tap();
        const deleteConfirmation = page.getByRole('alertdialog', { name: 'Delete session?' });
        await deleteConfirmation.waitFor();
        await deleteConfirmation.getByRole('button', { name: 'Cancel', exact: true }).tap();
        assert.deepEqual(errors, []);
      } finally {
        await browser.close();
        client.close();
        await stopMockServer();
      }
    }
  );
}
