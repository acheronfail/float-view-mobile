import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(
    `${name}: board export modal supports progress, cancellation, touch, keyboard and failures`,
    { timeout: 60_000 },
    async () => {
      const endpoint = getMockBootstrap(await startMockServer(11002, { safetyDisclaimerAccepted: true }));
      const browser = await engine.launch();
      try {
        const page = await browser.newPage({
          viewport: { width: 390, height: 844 },
          hasTouch: true,
          reducedMotion: 'reduce',
        });
        page.setDefaultTimeout(10_000);
        if (name === 'chromium')
          await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
        await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
        let empty = false;
        let simulated: 'exporting' | 'cancelled' | null = null;
        let release: (() => void) | undefined;
        await page.routeWebSocket('**/api/ws?*', (socket) => {
          const server = socket.connectToServer();
          socket.onMessage((raw) => {
            const message = JSON.parse(String(raw));
            if (simulated && ['session.export.status', 'session.export.cancel'].includes(message.method)) {
              if (message.method === 'session.export.cancel') {
                simulated = 'cancelled';
                release?.();
              }
              socket.send(
                JSON.stringify({
                  id: message.id,
                  result: {
                    operationId: message.params.operationId,
                    phase: simulated,
                    completedSessions: 2,
                    totalSessions: 8,
                    error: null,
                  },
                })
              );
            } else server.send(raw);
          });
          server.onMessage((raw) => {
            const message = JSON.parse(String(raw));
            const state = message.result?.permissions
              ? message.result
              : message.event === 'state.changed'
                ? message.data
                : null;
            if (empty && state) for (const board of state.savedDevices) board.summary.sessionCount = 0;
            socket.send(JSON.stringify(message));
          });
        });
        const exports: Record<string, unknown>[] = [];
        let fail = false;
        await page.route('**/api/session-export', async (route) => {
          exports.push(route.request().postDataJSON());
          simulated = 'exporting';
          await new Promise<void>((resolve) => {
            release = resolve;
          });
          if ((simulated as string | null) === 'cancelled') await route.fulfill({ status: 400, body: 'Cancelled' });
          else if (fail) {
            simulated = null;
            await route.fulfill({ status: 500, body: 'Export failed' });
          } else {
            simulated = null;
            await route.continue();
          }
        });
        await page.goto(`${endpoint.origin}/#/sessions/boards/saved-board`);
        const menu = page.getByRole('button', { name: 'Board actions', exact: true });
        await page.getByText('Export all saves one ZIP. Active recordings are skipped.', { exact: true }).waitFor();
        for (const [index, format] of ['float-view-archive', 'csv'].entries()) {
          await menu.tap();
          const action = page.getByRole('button', {
            name: index === 0 ? 'Export all · Float View archives' : 'Export all · CSV ZIPs',
            exact: true,
          });
          await action.click({ button: 'right' });
          assert.equal(exports.length, index);
          const pointer = { pointerId: 15, pointerType: 'touch', isPrimary: false, button: 0 };
          await action.dispatchEvent('pointerdown', pointer);
          await action.dispatchEvent('pointerup', pointer);
          assert.equal(exports.length, index);
          if (index === 0) await action.tap();
          else {
            await action.focus();
            await page.keyboard.press('Enter');
          }
          const dialog = page.getByRole('dialog', { name: 'Exporting sessions' });
          await dialog.getByRole('status').filter({ hasText: 'Exported 2 of 8 sessions' }).waitFor();
          assert.equal(await dialog.isVisible(), true);
          await page.keyboard.press('Tab');
          assert.equal(await dialog.locator(':focus').count(), 1);
          const download = page.waitForEvent('download');
          release!();
          assert.match((await download).suggestedFilename(), /\.zip$/);
          await page.getByRole('status').filter({ hasText: 'downloaded.' }).waitFor();
          assert.equal(exports[index]!.savedDeviceId, 'saved-board');
          assert.equal(exports[index]!.format, format);
          assert.match(String(exports[index]!.operationId), /^[a-f0-9-]{36}$/);
          await dialog.waitFor({ state: 'detached' });
        }
        fail = true;
        await menu.tap();
        await page.getByRole('button', { name: 'Export all · CSV ZIPs', exact: true }).tap();
        await page.getByRole('dialog', { name: 'Exporting sessions' }).waitFor();
        release!();
        await page.getByRole('alert').filter({ hasText: 'HTTP 500' }).waitFor();
        fail = false;
        await menu.tap();
        await page.getByRole('button', { name: 'Export all · CSV ZIPs', exact: true }).tap();
        const dialog = page.getByRole('dialog', { name: 'Exporting sessions' });
        await dialog.getByRole('status').filter({ hasText: 'Exported 2 of 8 sessions' }).waitFor();
        const cancel = dialog.getByRole('button', { name: 'Cancel', exact: true });
        await cancel.click({ button: 'right' });
        assert.equal(simulated, 'exporting');
        await cancel.focus();
        await page.keyboard.press('Enter');
        await page.getByRole('status').filter({ hasText: 'Export cancelled.' }).waitFor();
        await dialog.waitFor({ state: 'detached' });
        for (const activation of ['touch', 'escape']) {
          simulated = null;
          await menu.tap();
          await page.getByRole('button', { name: 'Export all · CSV ZIPs', exact: true }).tap();
          await dialog.getByRole('status').filter({ hasText: 'Exported 2 of 8 sessions' }).waitFor();
          if (activation === 'touch') await dialog.getByRole('button', { name: 'Cancel', exact: true }).tap();
          else await page.keyboard.press('Escape');
          await page.getByRole('status').filter({ hasText: 'Export cancelled.' }).waitFor();
          await dialog.waitFor({ state: 'detached' });
        }
        simulated = null;
        empty = true;
        await page.reload();
        await menu.tap();
        for (const label of ['Export all · Float View archives', 'Export all · CSV ZIPs'])
          assert.equal(await page.getByRole('button', { name: label, exact: true }).isDisabled(), true);
        assert.equal(exports.length, 6);
      } finally {
        await browser.close();
        await stopMockServer();
      }
    }
  );
}
