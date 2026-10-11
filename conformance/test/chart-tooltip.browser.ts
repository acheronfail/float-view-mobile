import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: tooltips fit mobile charts and cover markers`, { timeout: 60_000 }, async () => {
    const port = await startMockServer(11002);
    const endpoint = getMockBootstrap(port);
    const browser = await engine.launch();
    try {
      const page = await browser.newPage({ viewport: { width: 390, height: 844 }, hasTouch: true });
      page.setDefaultTimeout(5000);
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.route('https://tile.openstreetmap.org/**', (route) => route.abort());
      await page.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        server.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          if (message.result?.summary?.sessionId === 'session-1') message.result.markerCount = 1;
          socket.send(JSON.stringify(message));
        });
      });
      await page.route('**/api/session-data', async (route) => {
        const response = await route.fetch();
        const value = await response.json();
        if (value.kind === 'series') {
          for (const column of Object.values(value.columns) as {
            elapsedMs: number[];
            sequence: number[];
            values: number[];
            generations: number[];
          }[]) {
            Object.assign(column, {
              elapsedMs: [0, 60000],
              sequence: [1, 2],
              values: [123.45, 234.56],
              generations: [0, 0],
            });
          }
        }
        if (value.kind === 'markers') {
          value.markers = [
            {
              sequence: 1,
              elapsedMs: 30000,
              observedAtMs: 30000,
              operationId: '00000000-0000-4000-8000-000000000001',
              location: null,
            },
          ];
          value.nextCursor = null;
        }
        await route.fulfill({ response, json: value });
      });
      await page.goto(`${endpoint.origin}/#/sessions/session-1`);
      const chart = page.getByRole('img', { name: 'Current over elapsed time', exact: true });
      await chart.waitFor();
      await page.getByRole('button', { name: 'Show/hide the map', exact: true }).click();
      const plot = chart.locator('..');
      for (const width of [320, 390, 844]) {
        await page.setViewportSize({ width, height: 844 });
        // ResizeObserver reloads the chart series on the next animation frame.
        await page.waitForTimeout(250);
        await chart.scrollIntoViewIfNeeded();
        for (const ratio of [0.01, 0.49, 0.51, 0.99]) {
          const box = (await chart.boundingBox())!;
          await chart.dispatchEvent('pointermove', {
            pointerType: 'mouse',
            buttons: 0,
            clientX: box.x + box.width * ratio,
            clientY: box.y + 90,
          });
          const tooltip = plot.getByRole('status');
          await tooltip.waitFor();
          await page.evaluate(
            () => new Promise<void>((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve())))
          );
          const bounds = (await tooltip.boundingBox())!;
          assert.ok(
            bounds.x >= box.x - 1 && bounds.x + bounds.width <= box.x + box.width + 1,
            `tooltip fits ${width}px at ${ratio}`
          );
          assert.match(await tooltip.innerText(), /Motor current/);
          assert.equal(await tooltip.evaluate((node) => node.scrollWidth <= node.clientWidth), true);
          for (const value of await tooltip.locator('.text-right').all()) {
            const valueBounds = (await value.boundingBox())!;
            assert.ok(valueBounds.x >= bounds.x && valueBounds.x + valueBounds.width <= bounds.x + bounds.width);
            assert.match(await value.innerText(), /\d/);
          }
          if (ratio === 0.49) {
            assert.equal(
              await tooltip.evaluate((node) => {
                const markerBox = node.parentElement!.querySelector('button')!.getBoundingClientRect();
                const tooltipBox = node.getBoundingClientRect();
                (node as HTMLElement).style.pointerEvents = 'auto';
                const hit = document.elementFromPoint(markerBox.x + markerBox.width / 2, tooltipBox.y + 8);
                (node as HTMLElement).style.pointerEvents = '';
                return node.contains(hit);
              }),
              true,
              'tooltip paints over the marker'
            );
          }
        }
      }
    } finally {
      await browser.close();
      await stopMockServer();
    }
  });
}
