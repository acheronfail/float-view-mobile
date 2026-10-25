import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: select and retain current-ride max speed in Live and Custom`, { timeout: 60_000 }, async () => {
    const port = await startMockServer(11002, { safetyDisclaimerAccepted: true });
    const endpoint = getMockBootstrap(port);
    const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
    let browser: Awaited<ReturnType<typeof engine.launch>> | undefined;
    try {
      browser = await engine.launch();
      await client.request('bluetooth.requestPermission', {});
      await client.request('bluetooth.scan', { timeoutMs: 100 });
      const scanned = await client.waitForState((s) => s.discoveredDevices.length > 0);
      await client.request('device.connect', {
        deviceId: scanned.discoveredDevices[0]!.deviceId,
        trackingMode: 'saved',
      });
      const ready = await client.waitForState(
        (s) => s.activeConnection?.state === 'ready' && !!s.activeConnection.latestSample
      );
      const savedDeviceId = ready.activeConnection!.savedDeviceId!;
      const profile = await client.request('boardProfile.get', { savedDeviceId });
      await client.request('boardProfile.update', {
        savedDeviceId,
        expectedRevision: profile.revision,
        patch: { customDashboard: { version: 1, widgets: [] } },
      });
      const page = await browser.newPage({
        viewport: { width: 390, height: 844 },
        hasTouch: true,
        reducedMotion: 'reduce',
      });
      page.setDefaultTimeout(7000);
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      const errors: string[] = [];
      page.on('pageerror', (error) => errors.push(error.message));
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.goto(`${endpoint.origin}/#/live`);
      const firstBox = page.getByRole('button', { name: 'Choose value for box 1: Empty', exact: true });
      await firstBox.focus();
      await page.keyboard.press('Enter');
      const picker = page.getByRole('dialog', { name: 'Choose a value' });
      const choice = picker.getByRole('button', { name: 'Max speed', exact: true });
      await choice.scrollIntoViewIfNeeded();
      await choice.tap();
      const peakBox = page.getByRole('button', { name: 'Choose value for box 1: Max speed', exact: true });
      await peakBox.waitFor();
      assert.match(await peakBox.innerText(), /24\.3/);
      await page.getByRole('button', { name: 'Board actions', exact: true }).tap();
      await page.getByRole('button', { name: 'Custom View', exact: true }).tap();
      const custom = page.getByRole('region', { name: 'Custom dashboard', exact: true });
      await custom.getByRole('button', { name: 'Add widget', exact: true }).tap();
      await page.getByRole('searchbox', { name: 'Search widgets' }).fill('max speed');
      await page.getByRole('button', { name: 'Max speed', exact: true }).tap();
      await page.getByRole('button', { name: 'Add widget', exact: true }).tap();
      await custom.getByRole('button', { name: 'Save dashboard', exact: true }).tap();
      await custom.getByText('Press and hold any widget to arrange your dashboard.', { exact: true }).waitFor();
      assert.match(await custom.innerText(), /24\.3/);
      const saved = await client.request('boardProfile.get', { savedDeviceId });
      assert.equal(saved.values.liveWidgetSlots?.[0], 'maxSpeedKph');
      assert.deepEqual(
        saved.values.customDashboard?.widgets.map((w) => w.kind === 'reading' && w.field),
        ['maxSpeedKph']
      );
      const settings = (await client.request('state.get', {})).settings;
      await client.request('settings.update', {
        expectedRevision: settings.revision,
        patch: { unitPreference: 'imperial' },
      });
      await page.reload();
      await custom.getByText('mph', { exact: true }).waitFor();
      assert.match(await custom.innerText(), /15\.1/);
      assert.deepEqual(errors, []);
    } finally {
      await browser?.close();
      client.close();
      await stopMockServer();
    }
  });
}
