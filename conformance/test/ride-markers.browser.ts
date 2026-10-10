import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: gauge capture and marker review in the production UI`, { timeout: 60_000 }, async () => {
    const port = await startMockServer(11002);
    const endpoint = getMockBootstrap(port);
    const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
    const browser = await engine.launch();
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
        colorScheme: name === 'webkit' ? 'light' : 'dark',
        reducedMotion: 'reduce',
        hasTouch: true,
      });
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      const errors: string[] = [];
      page.on('pageerror', (error) => errors.push(error.message));

      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.goto(`${endpoint.origin}/#/live`);
      const capture = page.getByRole('button', { name: 'Save ride marker', exact: true });
      await capture.waitFor({ timeout: 10_000 }).catch(async (error) => {
        console.error(page.url(), await page.locator('body').innerText(), errors);
        throw error;
      });
      await capture.click();
      assert.equal((await client.request('session.get', { sessionId })).markerCount, 0);
      // Separate the single tap from the deliberate pair.
      await page.waitForTimeout(400);
      if (name === 'webkit') {
        await capture.tap();
        await capture.tap();
      } else await capture.dblclick({ delay: 60 });
      await page.getByRole('status').filter({ hasText: 'Marker saved' }).waitFor();
      assert.equal((await client.request('session.get', { sessionId })).markerCount, 1);
      await page.waitForTimeout(550);
      await capture.focus();
      await page.keyboard.press('Enter');
      await page.waitForTimeout(150);
      assert.equal((await client.request('session.get', { sessionId })).markerCount, 2);
      // Immediate repeat is rejected by admission rate limiting and never announces success.
      await page.keyboard.press('Enter');
      await page.getByText('Marker could not be saved', { exact: true }).waitFor();
      assert.equal((await client.request('session.get', { sessionId })).markerCount, 2);
      await client.request('device.disconnect', {});
      // A marker-only map fixture: the base track has been removed, while the
      // second marker still owns its committed fix. Keep capture above unmodified.
      await page.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        const methods = new Map<number, string>();
        socket.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          methods.set(message.id, message.method);
          server.send(raw);
        });
        server.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          if (methods.get(message.id) === 'session.get' && message.result?.summary.sessionId === sessionId) {
            message.result.snapshot.committedLocationSequence = 1;
          }
          methods.delete(message.id);
          socket.send(JSON.stringify(message));
        });
      });
      await page.route('**/api/session-data', async (route) => {
        const response = await route.fetch();
        const value = await response.json();
        if (value.kind === 'series') {
          for (const column of Object.values(value.columns) as { values: (number | null)[] }[])
            column.values.fill(null);
        }
        if (value.kind === 'markers' && value.markers[1]) {
          const marker = value.markers[1];
          marker.location = {
            locationSequence: 1,
            elapsedMs: marker.elapsedMs,
            observedAtMs: marker.observedAtMs,
            latitude: -33.8,
            longitude: 151.2,
            accuracyM: 5,
          };
        }
        await route.fulfill({ response, json: value });
      });
      await page.route('https://tile.openstreetmap.org/**', (route) => route.abort());
      await page.reload();

      await page.goto(`${endpoint.origin}/#/sessions/${sessionId}`);
      const select = page.getByRole('group', { name: 'Select ride marker' });
      await select.waitFor();
      assert.equal(await select.getByRole('button').count(), 2);
      await page
        .getByRole('button', { name: /Select marker at/ })
        .first()
        .waitFor();
      await page.getByText('No finite samples in this range.', { exact: true }).first().waitFor();
      await page.getByRole('button', { name: 'Next marker', exact: true }).click();
      assert.match(await select.getByRole('button', { pressed: true }).innerText(), /Marker 1.*No location/);
      await page.getByRole('button', { name: 'Next marker', exact: true }).focus();
      await page.keyboard.press('Enter');
      assert.match(await select.getByRole('button', { pressed: true }).innerText(), /Marker 2/);
      assert.doesNotMatch(await select.getByRole('button', { pressed: true }).innerText(), /No location/);
      const mapMarker = page.getByRole('button', { name: /Ride marker at/ });
      await mapMarker.waitFor();
      assert.equal(await page.locator('.location-marker').count(), 0);
      await page.getByRole('button', { name: 'Previous marker', exact: true }).click();
      await mapMarker.focus();
      await page.keyboard.press('Enter');
      assert.match(await select.getByRole('button', { pressed: true }).innerText(), /Marker 2/);
      await page
        .getByRole('button', { name: /Select marker at/ })
        .first()
        .click();
      assert.match(await select.getByRole('button', { pressed: true }).innerText(), /Marker 1/);

      // Scroll chart markers underneath the sticky map and open header menu.
      // Hit testing catches markers escaping their chart's stacking context.
      const chartMarker = page.getByRole('button', { name: /Select marker at/ }).first();
      const map = page.getByRole('region', { name: 'Session location map' });
      const mapBox = await map.boundingBox();
      assert.ok(mapBox);
      await chartMarker.evaluate(
        (marker, y) => {
          const box = marker.getBoundingClientRect();
          window.scrollBy(0, box.y + box.height / 2 - y);
        },
        mapBox.y + mapBox.height / 2
      );
      await page.evaluate(() => new Promise(requestAnimationFrame));
      assert.equal(
        await chartMarker.evaluate((marker) => {
          const box = marker.getBoundingClientRect();
          const top = document.elementFromPoint(box.x + box.width / 2 - 3, box.y + box.height / 2);
          return !!top?.closest('[aria-label="Session location map"]');
        }),
        true,
        'sticky map must cover chart markers'
      );

      await page.getByRole('button', { name: 'Session actions', exact: true }).click();
      const share = page.getByRole('button', { name: /^(Share|Save|Download) Archive$/ });
      const shareLabel = (await share.innerText()).trim();
      const shareBox = await share.boundingBox();
      assert.ok(shareBox);
      const menuMarker = chartMarker.locator('..').getByRole('button').last();
      await menuMarker.evaluate(
        (marker, y) => {
          const box = marker.getBoundingClientRect();
          window.scrollBy(0, box.y + box.height / 2 - y);
        },
        shareBox.y + shareBox.height / 2
      );
      await page.evaluate(() => new Promise(requestAnimationFrame));
      assert.equal(
        await menuMarker.evaluate((marker) => {
          const box = marker.getBoundingClientRect();
          const top = document.elementFromPoint(box.x + box.width / 2 - 3, box.y + box.height / 2);
          return top?.closest('button')?.textContent?.trim();
        }),
        shareLabel,
        'header menu must cover chart markers'
      );
      assert.deepEqual(errors, []);
    } finally {
      await browser.close();
      client.close();
      await stopMockServer();
    }
  });
}
