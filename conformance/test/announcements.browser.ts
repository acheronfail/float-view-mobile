import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';
const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: announcement settings and guarded activation`, { timeout: 60_000 }, async () => {
    const port = await startMockServer(11002);
    const endpoint = getMockBootstrap(port);
    const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
    const browser = await engine.launch();
    try {
      const page = await browser.newPage({ hasTouch: true, reducedMotion: 'reduce' });
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      const requests: string[] = [];
      const errors: string[] = [];
      page.on('pageerror', (e) => errors.push(e.message));
      page.on('websocket', (socket) =>
        socket.on('framesent', (frame) => {
          try {
            requests.push(JSON.parse(String(frame.payload)).method);
          } catch {}
        })
      );
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.goto(`${endpoint.origin}/#/settings`);
      const speed = page.getByRole('group', { name: 'Speed announcements', exact: true });
      await speed.getByRole('button', { name: 'On', exact: true }).waitFor();
      await speed.getByRole('button', { name: 'On', exact: true }).click({ button: 'right' });
      assert.equal(await speed.getByRole('button', { name: 'Off', exact: true }).getAttribute('aria-pressed'), 'true');
      await speed.getByRole('button', { name: 'On', exact: true }).tap();
      await client.waitForState((s) => s.settings.announcements.speedEnabled);
      await page
        .getByRole('group', { name: 'Load announcements', exact: true })
        .getByRole('button', { name: 'On', exact: true })
        .focus();
      await page.keyboard.press('Enter');
      await client.waitForState((s) => s.settings.announcements.loadEnabled);
      await page.getByLabel('Repeat interval (seconds)', { exact: true }).fill('1');
      await page.getByRole('slider').focus();
      await page.keyboard.press('Home');
      await page.getByRole('slider').blur();
      await client.waitForState(
        (s) => s.settings.announcements.volumePercent === 0 && s.settings.announcements.repeatIntervalSeconds === 1
      );
      assert.equal(await page.getByRole('button', { name: 'Save announcements', exact: true }).count(), 0);
      const state = await client.request('state.get', {});
      assert.equal(state.settings.announcements.speedEnabled, true);
      assert.equal(state.settings.announcements.loadEnabled, true);
      assert.equal(state.settings.announcements.volumePercent, 0);
      assert.equal(requests.filter((x) => x === 'settings.update').length, 4);
      const control = page.getByRole('group', { name: 'Lower other audio while speaking', exact: true });
      assert.equal(await control.count(), 1);
      const button = page.getByRole('button', { name: 'Test combined', exact: true });
      // Cancellation and non-primary pointers must not activate speech.
      await button.evaluate((el) =>
        el.addEventListener('pointerdown', (event) =>
          el.setAttribute('data-pointer-id', String((event as PointerEvent).pointerId))
        )
      );
      await button.hover();
      await page.mouse.down();
      const pointerId = Number(await button.getAttribute('data-pointer-id'));
      await button.dispatchEvent('pointercancel', { pointerId, isPrimary: true });
      await page.mouse.up();
      await button.click({ button: 'right' });
      assert.equal(requests.filter((x) => x === 'announcements.test').length, 0);
      await button.tap();
      await page.getByText('Test finished silently: announcement volume is muted.', { exact: true }).waitFor();
      assert.equal(requests.filter((x) => x === 'announcements.test').length, 1);
      assert.equal(await page.getByRole('button', { name: 'Voice settings', exact: true }).count(), 0);
      let voiceRequests = 0;
      let failSaves = true;
      let automaticSaves = 0;
      let speechRequests = 0;
      let savedLoad = 65;
      let loadAtTest: number | undefined;
      const automatic = await browser.newPage({ hasTouch: true });
      if (name === 'chromium')
        await automatic.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      await automatic.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        socket.onMessage((raw) => {
          const request = JSON.parse(String(raw));
          if (request.method === 'settings.update') {
            automaticSaves++;
            if (failSaves)
              socket.send(
                JSON.stringify({ id: request.id, error: { code: 'unavailable', message: 'Test save failure' } })
              );
            else setTimeout(() => server.send(raw), 250);
          } else if (request.method === 'announcements.test') {
            speechRequests++;
            loadAtTest = savedLoad;
            server.send(raw);
          } else if (request.method === 'announcements.openVoiceSettings') {
            voiceRequests++;
            setTimeout(
              () =>
                socket.send(
                  JSON.stringify(
                    voiceRequests === 1
                      ? { id: request.id, error: { code: 'unavailable', message: 'Test launch failure' } }
                      : { id: request.id, result: { accepted: true } }
                  )
                ),
              100
            );
          } else server.send(raw);
        });
        server.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          if (message.result?.announcements) savedLoad = message.result.announcements.loadThresholdPercent;
          const state = message.event === 'state.changed' ? message.data : message.result;
          if (state?.announcementAudio) {
            state.announcementAudio.loweringControl = 'automatic';
            state.announcementAudio.voiceSettingsAvailable = true;
          }
          socket.send(JSON.stringify(message));
        });
      });
      await automatic.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await automatic.goto(`${endpoint.origin}/#/settings`);
      await automatic.getByRole('slider').waitFor();
      assert.equal(
        await automatic.getByRole('group', { name: 'Lower other audio while speaking', exact: true }).count(),
        0
      );
      await automatic.getByLabel('Load threshold (%)', { exact: true }).fill('77');
      await automatic.getByLabel('Load threshold (%)', { exact: true }).blur();
      await automatic
        .getByText('Could not save announcement settings. Your edits are kept; try again.', { exact: true })
        .waitFor();
      assert.equal(await automatic.getByLabel('Load threshold (%)', { exact: true }).inputValue(), '77');
      const voiceButton = automatic.getByRole('button', { name: 'Voice settings', exact: true });
      await voiceButton.click({ button: 'right' });
      assert.equal(voiceRequests, 0);
      await voiceButton.tap();
      await automatic
        .getByText('Could not open voice settings. Open text-to-speech settings on your device and try again.', {
          exact: true,
        })
        .waitFor();
      assert.equal(voiceRequests, 1);
      assert.equal(await automatic.getByLabel('Load threshold (%)', { exact: true }).inputValue(), '77');
      await voiceButton.focus();
      await automatic.keyboard.press('Enter');
      await automatic
        .getByText('Voice settings opened. Return here after choosing an offline English voice.', { exact: true })
        .waitFor();
      assert.equal(voiceRequests, 2);
      assert.equal(await automatic.getByLabel('Load threshold (%)', { exact: true }).inputValue(), '77');
      // Failed automatic saves retain edits and never allow a test with old settings.
      await automatic.getByRole('button', { name: 'Test combined', exact: true }).tap();
      await automatic
        .getByText('Could not save announcement settings. Your edits are kept; try again.', { exact: true })
        .waitFor();
      assert.equal(speechRequests, 0);
      failSaves = false;
      await automatic.getByRole('button', { name: 'Retry saving', exact: true }).tap();
      // A delayed save of 77 must not replace a newer, still-focused edit.
      const loadInput = automatic.getByLabel('Load threshold (%)', { exact: true });
      await loadInput.fill('81');
      await automatic.getByText('Announcement settings saved.', { exact: true }).waitFor();
      assert.equal(await loadInput.inputValue(), '81');
      assert.equal((await client.request('state.get', {})).settings.announcements.loadThresholdPercent, 77);
      await automatic.getByRole('button', { name: 'Test combined', exact: true }).tap();
      await automatic.getByText('Test finished silently: announcement volume is muted.', { exact: true }).waitFor();
      assert.equal(speechRequests, 1);
      assert.equal(loadAtTest, 81);
      assert.equal((await client.request('state.get', {})).settings.announcements.loadThresholdPercent, 81);
      const beforeInvalid = automaticSaves;
      const interval = automatic.getByLabel('Repeat interval (seconds)', { exact: true });
      await interval.fill('0');
      await interval.blur();
      assert.equal(automaticSaves, beforeInvalid);
      await interval.fill('2');
      await interval.blur();
      await client.waitForState((s) => s.settings.announcements.repeatIntervalSeconds === 2);
      assert.equal(automaticSaves, beforeInvalid + 1);
      // The adjustable lowering control saves immediately too.
      await control.getByRole('button', { name: 'Off', exact: true }).tap();
      await client.waitForState((s) => !s.settings.announcements.lowerOtherAudio);
      await automatic.close();
      assert.deepEqual(errors, []);
    } finally {
      await browser.close();
      client.close();
      await stopMockServer();
    }
  });
}
