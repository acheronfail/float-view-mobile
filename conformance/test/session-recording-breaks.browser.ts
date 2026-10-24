import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  for (const reason of ['pause', 'connectionLoss', 'unknown'])
    for (const input of ['mouse', 'touch'])
      test(
        `${name}: ${reason} recording breaks support ${input} inspection, zoom and pan`,
        { timeout: 60_000 },
        async () => {
          const endpoint = getMockBootstrap(await startMockServer(11002, { safetyDisclaimerAccepted: true }));
          const browser = await engine.launch();
          try {
            const page = await browser.newPage({
              viewport: { width: input === 'touch' ? 390 : 1100, height: input === 'touch' ? 844 : 760 },
              hasTouch: input === 'touch',
            });
            page.setDefaultTimeout(7000);
            const errors: string[] = [];
            page.on('pageerror', (error) => errors.push(error.message));
            if (name === 'chromium')
              await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
            await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
            await page.route('https://tile.openstreetmap.org/**', (route) => route.abort());
            await page.routeWebSocket('**/api/ws?*', (socket) => {
              const server = socket.connectToServer();
              server.onMessage((raw) => {
                const message = JSON.parse(String(raw));
                if (message.result?.summary && message.result?.fields)
                  message.result.recordingBreaks = [[20000, 40000, reason]];
                socket.send(JSON.stringify(message));
              });
            });
            await page.route('**/api/session-data', async (route) => {
              const response = await route.fetch();
              const value = await response.json();
              if (value.kind === 'series') {
                // The small 10–12s timing interruption has no confirmed boundary.
                const source = [
                  [0, 0],
                  [10000, 0],
                  [12000, 1],
                  [20000, 1],
                  [40000, 2],
                  [60000, 2],
                ].filter(([time]) => time! >= value.range.startElapsedMs && time! <= value.range.endElapsedMs);
                for (const column of Object.values(value.columns))
                  Object.assign(column as object, {
                    elapsedMs: source.map(([time]) => time),
                    sequence: source.map((_, i) => i + 1),
                    generations: source.map(([, generation]) => generation),
                    values: source.map((_, i) => i * 5),
                  });
              }
              await route.fulfill({ response, json: value });
            });
            await page.goto(`${endpoint.origin}/#/sessions/session-1`);
            const chart = page.getByRole('img', { name: 'Speed over elapsed time', exact: true });
            await chart.waitFor();
            await page.getByRole('button', { name: 'Show/hide the map', exact: true }).click();
            await chart.scrollIntoViewIfNeeded();
            const separator = chart.locator('[data-recording-break]');
            if (reason !== 'pause') {
              assert.equal(await separator.count(), 0, 'only deliberate pauses have markers');
              const paths = chart.locator('.telemetry-chart-trace');
              assert.equal(await paths.count(), 3, 'interruption traces stay disconnected');
              assert.match((await paths.nth(1).getAttribute('d')) ?? '', /^M 64\.00 /);
              assert.match(
                (await paths.nth(2).getAttribute('d')) ?? '',
                /^M 213\.33 /,
                'non-pause gaps retain their original width'
              );
              assert.deepEqual(errors, []);
              return;
            }
            assert.equal(await separator.count(), 1);
            assert.match((await separator.locator('title').textContent()) ?? '', /0:20 missing.*0:20–0:40/);
            assert.equal(Number(await separator.locator('line').last().getAttribute('x1')), 160);
            const paths = chart.locator('.telemetry-chart-trace');
            assert.equal(await paths.count(), 3, 'ordinary timing gap remains a disconnected segment');
            assert.match(
              (await paths.nth(1).getAttribute('d')) ?? '',
              /^M 96\.00 /,
              'unconfirmed 2s delay retains width'
            );

            let bounds = (await chart.boundingBox())!;
            if (input === 'touch') {
              await page.mouse.move(bounds.x + bounds.width * 0.75, bounds.y + bounds.height / 2);
              await chart.dispatchEvent('pointerdown', {
                pointerType: 'touch',
                pointerId: 1,
                isPrimary: true,
                button: 0,
                clientX: bounds.x + bounds.width * 0.75,
                clientY: bounds.y + bounds.height / 2,
              });
              await page.getByText('50.0s into ride', { exact: true }).first().waitFor();
              await chart.dispatchEvent('pointermove', {
                pointerType: 'touch',
                pointerId: 1,
                isPrimary: true,
                buttons: 1,
                clientX: bounds.x + bounds.width * 0.5,
                clientY: bounds.y + bounds.height / 2,
              });
              await page.getByRole('status').filter({ hasText: 'Recording break · 0:20 missing' }).first().waitFor();
              await chart.dispatchEvent('pointerup', {
                pointerType: 'touch',
                pointerId: 1,
                isPrimary: true,
                button: 0,
              });
              assert.deepEqual(errors, []);
              return;
            }
            const move = (ratio: number) =>
              page.mouse.move(bounds.x + bounds.width * ratio, bounds.y + bounds.height / 2);
            await move(0.75);
            await page.getByText('50.0s into ride', { exact: true }).first().waitFor();
            await move(0.5);
            await page
              .getByRole('status')
              .filter({ hasText: 'Recording break · 0:20 missing · 0:20–0:40' })
              .first()
              .waitFor();
            const zoomed = page.waitForRequest(
              (request) =>
                request.url().endsWith('/api/session-data') &&
                request.postDataJSON().kind === 'series' &&
                request.postDataJSON().range.startElapsedMs > 0
            );
            await move(0.25);
            await page.mouse.down();
            await move(0.75);
            await page.mouse.up();
            const zoom = (await zoomed).postDataJSON().range;
            assert.ok(Math.abs(zoom.startElapsedMs - 10000) < 200);
            assert.ok(Math.abs(zoom.endElapsedMs - 50000) < 200);
            await chart.locator('.telemetry-chart-trace').first().waitFor({ state: 'attached' });
            bounds = (await chart.boundingBox())!;
            const panned = page.waitForRequest(
              (request) =>
                request.url().endsWith('/api/session-data') &&
                request.postDataJSON().kind === 'series' &&
                request.postDataJSON().range.startElapsedMs < 9000
            );
            await move(0.5);
            await page.mouse.down({ button: 'right' });
            await move(0.75);
            await page.mouse.up({ button: 'right' });
            const pan = (await panned).postDataJSON().range;
            assert.ok(Math.abs(pan.startElapsedMs - 5000) < 200);
            assert.ok(Math.abs(pan.endElapsedMs - 45000) < 200);
            await page.getByRole('button', { name: 'Reset chart zoom', exact: true }).click();
            await separator.waitFor({ state: 'attached' });
            await page.getByRole('button', { name: 'Show/hide the map', exact: true }).click();
            const filter = page.getByRole('switch', { name: 'Filter charts by map', exact: true });
            await filter.waitFor();
            await filter.click();
            await page.getByText('Map controls the chart range.', { exact: false }).first().waitFor();
            await chart.scrollIntoViewIfNeeded();
            await separator.waitFor({ state: 'attached' });
            assert.equal(await separator.count(), 1, 'map selection composes with the recording break');
            assert.deepEqual(errors, []);
          } finally {
            await browser.close();
            await stopMockServer();
          }
        }
      );
}
