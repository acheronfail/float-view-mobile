import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: inspection stays full height across sparse readings and gaps`, { timeout: 60_000 }, async () => {
    const port = await startMockServer(11002, { safetyDisclaimerAccepted: true });
    const endpoint = getMockBootstrap(port);
    const browser = await engine.launch();
    try {
      const page = await browser.newPage({ viewport: { width: 1100, height: 760 } });
      page.setDefaultTimeout(5000);
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.route('https://tile.openstreetmap.org/**', (route) => route.abort());
      await page.route('**/api/session-data', async (route) => {
        const response = await route.fetch();
        const value = await response.json();
        if (value.kind === 'series') {
          for (const column of Object.values(value.columns)) {
            Object.assign(column as object, {
              elapsedMs: [15000, 20000, 25000, 40000, 45000],
              sequence: [1, 2, 3, 4, 5],
              values: [1, 2, null, 2, 1],
              generations: [0, 0, 0, 1, 1],
            });
          }
        }
        await route.fulfill({ response, json: value });
      });
      await page.goto(`${endpoint.origin}/#/sessions/session-1`);
      const chart = page.getByRole('img', { name: 'Current over elapsed time', exact: true });
      await chart.waitFor();
      await page.getByRole('button', { name: 'Show/hide the map', exact: true }).click();
      await chart.scrollIntoViewIfNeeded();
      const plot = chart.locator('..');
      await page.waitForFunction(() =>
        document.querySelector('svg[aria-label="Current over elapsed time"] path[stroke]')
      );
      // Enter/leave both ends of a short segment and cross an internal gap.
      for (const ratio of [0.3, 0.1, 0.3, 0.5, 0.7, 0.9, 0.7, 0.5]) {
        const box = (await chart.boundingBox())!;
        await page.mouse.move(box.x + box.width * ratio, box.y + box.height / 2);
        await page.waitForFunction(
          ({ ratio }) => {
            const chart = document.querySelector('svg[aria-label="Current over elapsed time"]')!;
            const cursor = chart.parentElement!.querySelector('[data-history-inspection]');
            if (!cursor) return false;
            const box = chart.getBoundingClientRect();
            return Math.abs(cursor.getBoundingClientRect().x - (box.x + box.width * ratio)) < 1;
          },
          { ratio }
        );
        const cursor = plot.locator('[data-history-inspection]');
        assert.equal(await cursor.count(), 1, 'one cursor spans the entire plot');
        const bounds = (await cursor.boundingBox())!;
        assert.ok(Math.abs(bounds.y - box.y) < 1 && Math.abs(bounds.height - box.height) < 1);
        const hasReading = ratio === 0.3 || ratio === 0.7;
        assert.equal(await plot.getByRole('status').count(), hasReading ? 1 : 0);
        assert.equal((await plot.locator('[data-history-point]').count()) > 0, hasReading);
      }
      await page.mouse.move(0, 0);
      await plot.locator('[data-history-inspection]').waitFor({ state: 'detached' });
    } finally {
      await browser.close();
      await stopMockServer();
    }
  });
  test(`${name}: tooltips fit mobile charts and cover markers`, { timeout: 60_000 }, async () => {
    const port = await startMockServer(11002, { safetyDisclaimerAccepted: true });
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
          for (const [field, column] of Object.entries(value.columns) as [
            string,
            {
              elapsedMs: number[];
              sequence: number[];
              values: number[];
              generations: number[];
            },
          ][]) {
            Object.assign(column, {
              elapsedMs: [0, 60000],
              sequence: [1, 2],
              values:
                field === 'dutyCycle'
                  ? [-0.4, -0.4]
                  : field === 'motorLoad'
                    ? [0.6, 0.6]
                    : field === 'batteryLoad'
                      ? [0.75, 0.75]
                      : [123.45, 234.56],
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
          const points = plot.locator('[data-history-point]');
          assert.ok((await points.count()) > 0, 'inspection highlights the chart lines');
          for (const point of await points.all()) {
            const dot = (await point.boundingBox())!;
            assert.ok(Math.abs(dot.width - dot.height) < 0.1, `highlight stays circular at ${width}px`);
            assert.equal(dot.width, 7, 'highlight size is independent of plot scaling');
          }
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
      const load = page.getByRole('img', { name: 'Load over elapsed time', exact: true });
      await load.scrollIntoViewIfNeeded();
      await page.waitForFunction(
        () =>
          document.querySelector('svg[aria-label="Load over elapsed time"]')?.querySelectorAll('path[stroke]')
            .length === 3
      );
      const loadBox = (await load.boundingBox())!;
      await page.mouse.move(loadBox.x + loadBox.width * 0.3, loadBox.y + 50);
      const loadTooltip = load.locator('..').getByRole('status');
      await loadTooltip.waitFor();
      assert.match(await loadTooltip.innerText(), /40%/);
      assert.match(await loadTooltip.innerText(), /60%/);
      assert.match(await loadTooltip.innerText(), /75%/);
      await chart.scrollIntoViewIfNeeded();
      // Real touch input exercises browser scrolling and pointer cancellation;
      // dispatchEvent alone cannot establish which recognizer owns the gesture.
      if (name === 'chromium') {
        const input = await page.context().newCDPSession(page);
        const box = (await chart.boundingBox())!;
        const x = box.x + box.width * 0.4;
        const y = box.y + box.height * 0.7;
        const scrollY = await page.evaluate(() => window.scrollY);
        const move = async (clientX: number, clientY: number, type: 'touchStart' | 'touchMove' = 'touchMove') => {
          await input.send('Input.dispatchTouchEvent', {
            type,
            touchPoints: [{ x: clientX, y: clientY }],
          });
          await page.waitForTimeout(30);
        };
        await move(x, y, 'touchStart');
        await move(x + 6, y);
        await move(x + 25, y);
        for (let drift = 10; drift <= 90; drift += 10) await move(x + 25, y - drift);
        assert.equal(await page.evaluate(() => window.scrollY), scrollY, 'horizontal inspection owns vertical drift');
        await plot.getByRole('status').waitFor();
        await input.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
        await move(x, y, 'touchStart');
        for (let drift = 10; drift <= 90; drift += 10) await move(x, y - drift);
        await input.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
        assert.ok((await page.evaluate(() => window.scrollY)) > scrollY, 'vertical starts still scroll');
        await input.detach();
      }
    } finally {
      await browser.close();
      await stopMockServer();
    }
  });
}
