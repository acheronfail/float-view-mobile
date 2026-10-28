import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import type { BmsState } from '../../frontend/src/lib/backend/contract.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(
    `${name}: live electrical readings preserve distance and animate signed power`,
    { timeout: 60_000 },
    async () => {
      const port = await startMockServer(11002, { safetyDisclaimerAccepted: true });
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
        const active = (await client.request('state.get', {})).activeConnection!;
        assert.deepEqual(active.liveWidgetSlots, ['empty', 'powerW', 'elapsedMs', 'odometerM', 'distanceM']);
        const profile = await client.request('boardProfile.get', { savedDeviceId: active.savedDeviceId! });
        await client.request('boardProfile.update', {
          savedDeviceId: profile.savedDeviceId,
          expectedRevision: profile.revision,
          patch: {
            liveWidgetSlots: ['fieldWeakeningCurrentA', 'motorCurrentA', 'elapsedMs', 'powerW', 'distanceM'],
          },
        });
        const page = await browser.newPage({ viewport: { width: 390, height: 844 }, hasTouch: true });
        page.setDefaultTimeout(5000);
        if (name === 'chromium')
          await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
        const errors: string[] = [];
        page.on('pageerror', (e) => errors.push(e.message));
        let fw: number | null = 0.7;
        let amps: number | null = 12;
        let padState = 'both';
        let rideSpeed = 24.8;
        let rejectSelection = false;
        let saves = 0;
        let chargingBms: BmsState | null = null;
        let refreshBmsTime = true;
        await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
        await page.routeWebSocket('**/api/ws?*', (socket) => {
          const server = socket.connectToServer();
          socket.onMessage((raw) => {
            const message = JSON.parse(String(raw));
            if (message.method === 'boardProfile.update' && message.params?.patch?.liveWidgetSlots) {
              saves++;
              if (rejectSelection) {
                socket.send(
                  JSON.stringify({ id: message.id, error: { code: 'invalid_request', message: 'Test save failure' } })
                );
                return;
              }
            }
            server.send(raw);
          });
          server.onMessage((raw) => {
            const message = JSON.parse(String(raw));
            const state =
              message.event === 'state.changed'
                ? message.data
                : message.result?.activeConnection
                  ? message.result
                  : null;
            if (state?.activeConnection?.latestSample)
              Object.assign(state.activeConnection.latestSample, {
                elapsedMs: 90_000,
                ridingElapsedMs: 12_000,
                fieldWeakeningCurrentA: fw,
                batteryCurrentA: amps,
                motorCurrentA: 32.4,
                packVoltageV: 75,
                speedKph: rideSpeed,
                dutyCycle: 0.42,
                distanceM: 12400,
                footpadState: padState,
                footpadAdc1V: 0.4,
                footpadAdc2V: 3.1,
                motorTempC: 65,
                batteryLevel: 0.15,
              });
            if (state?.activeConnection && chargingBms) {
              state.activeConnection.latestSample.charging = false;
              if (refreshBmsTime && chargingBms.latestSnapshot)
                chargingBms.latestSnapshot.observedAt = new Date().toISOString();
              state.activeConnection.bms = chargingBms;
            }
            socket.send(JSON.stringify(message));
          });
        });
        await page.goto(`${endpoint.origin}/#/live`);
        chargingBms = {
          status: 'attached',
          connection: { kind: 'can', canId: 10 },
          candidates: [],
          latestSnapshot: {
            observedAt: new Date().toISOString(),
            status: 'CHARGING',
            dataVersion: 1,
            totalVoltageV: 78,
            chargeVoltageV: 84,
            inputCurrentA: 0,
            inputIcCurrentA: -5,
            sessionAmpHoursAh: 0,
            sessionWattHoursWh: 0,
            cellVoltagesV: [3.9],
            balancing: [false],
            temperaturesC: [22],
            icTemperatureC: 22,
            humiditySensorTemperatureC: 22,
            humidityPercent: 50,
            highestCellTemperatureC: 22,
            highestCircuitryTemperatureC: 30,
            stateOfCharge: 0.6,
            stateOfHealth: 1,
            reportedCanId: 10,
            lifetimeChargeAmpHoursAh: 0,
            lifetimeChargeWattHoursWh: 0,
            lifetimeDischargeAmpHoursAh: 0,
            lifetimeDischargeWattHoursWh: 0,
            pressurePa: 0,
          },
        };
        const widgetBms = structuredClone(chargingBms);
        const chargingLabel = page.locator('#power-title').getByText('Charging', { exact: true });
        await chargingLabel.waitFor();
        assert.equal(await page.getByText('Charging', { exact: true }).count(), 1);
        await page.locator('#power-title').evaluate((element) => {
          (window as any).chargingDisappeared = false;
          new MutationObserver(() => {
            if (!element.textContent?.includes('Charging')) (window as any).chargingDisappeared = true;
          }).observe(element, { childList: true, subtree: true, characterData: true });
        });
        await page.waitForTimeout(2500);
        assert.equal(await page.evaluate(() => (window as any).chargingDisappeared), false);
        refreshBmsTime = false;
        // Expire unchanged BMS data even while controller snapshots keep arriving.
        await chargingLabel.waitFor({ state: 'hidden', timeout: 8000 });
        chargingBms.latestSnapshot!.observedAt = new Date().toISOString();
        await chargingLabel.waitFor();
        chargingBms.status = 'unavailable';
        await chargingLabel.waitFor({ state: 'hidden' });
        await page.getByRole('button', { name: 'Board actions', exact: true }).tap();
        await page.getByRole('button', { name: 'BMS', exact: true }).tap();
        const cellBar = page.getByRole('progressbar', { name: 'Cell 1 voltage', exact: true });
        await cellBar.waitFor();
        assert.equal(await cellBar.getAttribute('aria-valuemin'), '3');
        assert.equal(await cellBar.getAttribute('aria-valuemax'), '4.2');
        assert.equal(await cellBar.getAttribute('aria-valuenow'), '3.9');
        assert.equal(
          await cellBar
            .locator('div')
            .last()
            .evaluate((el) => getComputedStyle(el).backgroundColor),
          'rgb(22, 163, 74)'
        );
        assert.equal(
          await page
            .getByRole('progressbar', { name: 'Sensor 1 temperature', exact: true })
            .getAttribute('aria-valuenow'),
          '22'
        );
        for (const width of [390, 1024]) {
          await page.setViewportSize({ width, height: 844 });
          assert.equal(await cellBar.evaluate((el) => el.getBoundingClientRect().width > 0), true);
          assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
          if (process.env.SCREENSHOT_DIR) {
            await mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
            await page.screenshot({ path: `${process.env.SCREENSHOT_DIR}/bms-${name}-${width}.png` });
          }
        }
        await page.setViewportSize({ width: 390, height: 844 });
        await page.getByRole('button', { name: 'Done', exact: true }).tap();
        chargingBms = null;
        const field = page.getByRole('group', { name: 'Field weakening', exact: true });
        const power = page.getByRole('group', { name: 'Battery power', exact: true });
        await page.waitForFunction(() =>
          document.querySelector('[aria-label="Battery power"]')?.textContent?.includes('+900')
        );
        assert.match(await field.innerText(), /0\.7/);
        const distanceBox = page.getByRole('button', { name: 'Box 5: Ride distance', exact: true });
        await distanceBox.waitFor();
        const odometerBox = page.getByRole('button', { name: 'Box 5: Odometer', exact: true });
        const savesBeforeToggle = saves;
        await distanceBox.click({ button: 'right' });
        await distanceBox.dispatchEvent('pointerdown', { button: 0, isPrimary: false, pointerId: 99 });
        await distanceBox.dispatchEvent('pointerup', { button: 0, isPrimary: false, pointerId: 99 });
        assert.equal(saves, savesBeforeToggle);
        const distanceBounds = (await distanceBox.boundingBox())!;
        await page.mouse.move(distanceBounds.x + 20, distanceBounds.y + 20);
        await page.mouse.down();
        await page.mouse.move(distanceBounds.x + 40, distanceBounds.y + 20);
        await page.mouse.up();
        assert.equal(saves, savesBeforeToggle, 'movement cancels the tap');
        await distanceBox.tap();
        await odometerBox.waitFor();
        assert.equal(saves, savesBeforeToggle + 1, 'touch switches once despite its synthesized click');
        assert.equal((await client.request('state.get', {})).activeConnection!.liveWidgetSlots[4], 'odometerM');
        await page.reload();
        await odometerBox.waitFor();
        await odometerBox.click();
        await distanceBox.waitFor();
        assert.equal(saves, savesBeforeToggle + 2, 'mouse switches back once');
        await distanceBox.focus();
        await page.keyboard.press('Enter');
        await odometerBox.waitFor();
        await page.keyboard.press('Space');
        await distanceBox.waitFor();
        assert.equal(saves, savesBeforeToggle + 4, 'keyboard activation switches both ways');
        rejectSelection = true;
        await distanceBox.tap();
        await page.getByRole('alert').filter({ hasText: 'Test save failure' }).waitFor();
        assert.equal(await distanceBox.count(), 1, 'a failed save preserves the current reading');
        rejectSelection = false;
        const rideTimer = page.getByRole('button', { name: 'Choose value for box 3: Ride time', exact: true });
        await rideTimer.getByText('0:00:12', { exact: true }).waitFor();
        await page.waitForTimeout(1100);
        assert.match(await rideTimer.innerText(), /0:00:12/);
        const timerProfile = await client.request('boardProfile.get', { savedDeviceId: active.savedDeviceId! });
        await client.request('boardProfile.update', {
          savedDeviceId: timerProfile.savedDeviceId,
          expectedRevision: timerProfile.revision,
          patch: {
            liveWidgetSlots: ['fieldWeakeningCurrentA', 'motorCurrentA', 'batteryCurrentA', 'powerW', 'distanceM'],
          },
        });
        await page.getByRole('button', { name: 'Choose value for box 3: Battery current', exact: true }).waitFor();
        assert.equal(await page.getByRole('button', { name: /^Choose value for box/ }).count(), 4);
        assert.equal(await page.locator('[aria-label="Live number widgets"] button').count(), 4);
        assert.equal(
          await page.locator('svg + div').getByRole('button', { name: 'Box 5: Ride distance', exact: true }).count(),
          1
        );
        const checkCentreBox = async (target = distanceBox) => {
          const bounds = await target.evaluate((button) => {
            const box = button.getBoundingClientRect();
            const gauge = button.parentElement!.parentElement!.previousElementSibling!.getBoundingClientRect();
            const readings = [
              ...button.querySelectorAll('[role="group"], [aria-hidden="true"], [aria-hidden="true"] > span'),
            ].map((element) => element.getBoundingClientRect());
            return {
              centreOffset: Math.abs((box.left + box.right - gauge.left - gauge.right) / 2),
              insideGauge:
                box.left >= gauge.left - 1 &&
                box.right <= gauge.right + 1 &&
                box.top >= gauge.top - 1 &&
                box.bottom <= gauge.bottom + 1,
              touchHeight: box.height,
              contained: readings.every(
                (reading) =>
                  reading.left >= box.left &&
                  reading.right <= box.right &&
                  reading.top >= box.top &&
                  reading.bottom <= box.bottom
              ),
            };
          });
          assert.ok(bounds.centreOffset < 1, JSON.stringify(bounds));
          assert.ok(bounds.insideGauge, JSON.stringify(bounds));
          assert.ok(bounds.touchHeight >= 44, JSON.stringify(bounds));
          assert.ok(bounds.contained, JSON.stringify(bounds));
        };
        await checkCentreBox();
        await page.setViewportSize({ width: 844, height: 390 });
        await checkCentreBox();
        await page.setViewportSize({ width: 390, height: 844 });
        const savesBeforeHold = saves;
        await distanceBox.hover();
        await page.mouse.down();
        await page.waitForTimeout(650);
        await page.mouse.up();
        const distancePicker = page.getByRole('dialog', { name: 'Choose a value' });
        await distancePicker.waitFor();
        assert.equal(saves, savesBeforeHold, 'holding configures without switching');
        // A drag that starts on a choice must leave the scrollable picker open.
        const firstChoice = distancePicker.getByRole('button', { name: 'Empty', exact: true });
        const savesBeforeScroll = saves;
        if (name === 'chromium') {
          const touch = await page.context().newCDPSession(page);
          const start = (await firstChoice.boundingBox())!;
          const x = start.x + start.width / 2;
          const y = start.y + start.height - 20;
          await touch.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [{ x, y }] });
          for (const offset of [20, 40, 60, 80]) {
            await touch.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: [{ x, y: y - offset }] });
            await page.waitForTimeout(20);
          }
          await touch.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
          await touch.detach();
          assert.ok(await distancePicker.locator('.overflow-y-auto').evaluate((element) => element.scrollTop > 0));
          assert.equal(await distancePicker.isVisible(), true);
          assert.equal(saves, savesBeforeScroll);
        }
        await firstChoice.scrollIntoViewIfNeeded();
        const choiceBounds = (await firstChoice.boundingBox())!;
        await page.mouse.move(choiceBounds.x + 20, choiceBounds.y + 20);
        await page.mouse.down();
        await page.mouse.move(choiceBounds.x + 20, choiceBounds.y + 65, { steps: 5 });
        await page.mouse.wheel(0, 400);
        await page.mouse.up();
        assert.equal(await distancePicker.isVisible(), true);
        assert.equal(saves, savesBeforeScroll);
        await distancePicker.getByRole('button', { name: 'LEDs', exact: true }).tap();
        await distancePicker.waitFor({ state: 'hidden' });
        const ledBox = page.locator('svg + div').getByRole('switch', { name: 'LEDs', exact: true });
        await ledBox.waitFor();
        await checkCentreBox(ledBox);
        await page.setViewportSize({ width: 844, height: 390 });
        await checkCentreBox(ledBox);
        await page.setViewportSize({ width: 390, height: 844 });
        const checked = await ledBox.getAttribute('aria-checked');
        await ledBox.tap();
        await page.waitForFunction(
          (checked) => document.querySelector('svg + div [role="switch"]')?.getAttribute('aria-checked') !== checked,
          checked
        );
        await ledBox.focus();
        await page.keyboard.press('Shift+Enter');
        await distancePicker.waitFor();
        await distancePicker.getByRole('button', { name: 'Load', exact: true }).tap();
        await distancePicker.waitFor({ state: 'hidden' });
        await page.getByRole('button', { name: 'Choose value for box 5: Load', exact: true }).focus();
        await page.keyboard.press('Enter');
        await distancePicker.getByRole('button', { name: 'Ride distance', exact: true }).tap();
        await distancePicker.waitFor({ state: 'hidden' });
        await distanceBox.focus();
        await page.keyboard.press('Shift+Enter');
        await distancePicker.waitFor();
        const pickerBounds = await distancePicker.boundingBox();
        assert.ok(pickerBounds && Math.abs(pickerBounds.width - 390) < 0.01);
        assert.ok(pickerBounds && Math.abs(pickerBounds.height - 844) < 0.01);
        if (process.env.SCREENSHOT_DIR)
          await page.screenshot({ path: `${process.env.SCREENSHOT_DIR}/gauge-picker-${name}.png` });
        await page.keyboard.press('Escape');
        await distancePicker.waitFor({ state: 'hidden' });
        assert.equal(await distanceBox.evaluate((el) => el === document.activeElement), true);
        saves = 0;
        await page.emulateMedia({ colorScheme: 'dark' });
        await page.waitForFunction(() => document.documentElement.dataset.theme === 'dark');
        const drawColor = await power
          .locator('div')
          .last()
          .evaluate((el) => getComputedStyle(el).color);
        for (const theme of ['light', 'dark'] as const) {
          await page.emulateMedia({ colorScheme: theme });
          await page.waitForFunction((theme) => document.documentElement.dataset.theme === theme, theme);
          await page.waitForTimeout(200);
          if (process.env.SCREENSHOT_DIR) {
            await mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
            await page.screenshot({ path: `${process.env.SCREENSHOT_DIR}/live-${name}-${theme}-draw.png` });
          }
        }
        // Observe intermediate rendered values, not just arrival at the target.
        await field.evaluate((el) => {
          (window as any).fwFrames = [];
          new MutationObserver(() => (window as any).fwFrames.push(el.textContent)).observe(el, {
            subtree: true,
            characterData: true,
            childList: true,
          });
        });
        fw = 8;
        amps = -4;
        await page.waitForFunction(() =>
          document.querySelector('[aria-label="Battery power"]')?.textContent?.includes('-300')
        );
        assert.match(await power.innerText(), /Regen/i);
        assert(await page.evaluate(() => (window as any).fwFrames.some((text: string) => /[1-7]\.\d/.test(text))));
        assert.notEqual(
          await power
            .locator('div')
            .last()
            .evaluate((el) => getComputedStyle(el).color),
          drawColor
        );
        for (const theme of ['light', 'dark'] as const) {
          await page.emulateMedia({ colorScheme: theme });
          await page.waitForFunction((theme) => document.documentElement.dataset.theme === theme, theme);
          await page.waitForTimeout(200);
          if (process.env.SCREENSHOT_DIR)
            await page.screenshot({ path: `${process.env.SCREENSHOT_DIR}/live-${name}-${theme}-regen.png` });
        }
        await page.emulateMedia({ reducedMotion: 'reduce' });
        fw = 0;
        amps = 0;
        await page.waitForFunction(() =>
          document.querySelector('[aria-label="Field weakening"]')?.textContent?.includes('0.0')
        );
        assert.equal(await distanceBox.count(), 1);
        fw = null;
        amps = null;
        await page.waitForFunction(() =>
          document.querySelector('[aria-label="Battery power"]')?.textContent?.includes('—')
        );
        assert.match(await field.innerText(), /—/);
        for (const viewport of [
          { width: 320, height: 568 },
          { width: 1280, height: 900 },
        ]) {
          await page.setViewportSize(viewport);
          assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
          const gauge = await page.getByRole('group', { name: 'Live speed and drive load gauge' }).boundingBox();
          assert(gauge && gauge.height > 150);
        }
        await page.setViewportSize({ width: 390, height: 844 });
        fw = 0.7;
        amps = 12;
        const firstBox = page.getByRole('button', { name: 'Choose value for box 1: Field weakening', exact: true });
        await firstBox.click({ button: 'right' });
        assert.equal(await page.getByRole('dialog').count(), 0);
        await firstBox.dispatchEvent('pointerdown', { button: 0, isPrimary: false, pointerId: 99 });
        await firstBox.dispatchEvent('pointerup', { button: 0, isPrimary: false, pointerId: 99 });
        assert.equal(await page.getByRole('dialog').count(), 0);
        await firstBox.evaluate((el) =>
          el.addEventListener(
            'pointerdown',
            (event) => el.setAttribute('data-pointer-id', String((event as PointerEvent).pointerId)),
            { once: true }
          )
        );
        await firstBox.hover();
        await page.mouse.down();
        await firstBox.dispatchEvent('pointercancel', {
          pointerId: Number(await firstBox.getAttribute('data-pointer-id')),
        });
        await page.mouse.up();
        assert.equal(await page.getByRole('dialog').count(), 0);
        await firstBox.tap();
        assert.equal(await page.getByRole('dialog').count(), 0, 'short taps do not configure');
        await firstBox.hover();
        await page.mouse.down();
        await page.waitForTimeout(650);
        await page.mouse.up();
        const picker = page.getByRole('dialog', { name: 'Choose a value' });
        await picker.waitFor();
        assert.equal(
          await picker.getByRole('button', { name: 'Field weakening', exact: true }).getAttribute('aria-pressed'),
          'true'
        );
        assert.equal(await picker.getByRole('button', { name: 'Charging voltage', exact: true }).count(), 1);
        for (const theme of ['light', 'dark'] as const) {
          await page.emulateMedia({ colorScheme: theme });
          await page.waitForFunction((theme) => document.documentElement.dataset.theme === theme, theme);
          if (process.env.SCREENSHOT_DIR)
            await page.screenshot({ path: `${process.env.SCREENSHOT_DIR}/picker-${name}-${theme}.png` });
        }
        await page.keyboard.press('Escape');
        await picker.waitFor({ state: 'hidden' });
        assert.equal(await firstBox.evaluate((el) => el === document.activeElement), true);
        await page.keyboard.press('Enter');
        await picker.waitFor();
        rejectSelection = true;
        await picker.getByRole('button', { name: 'Pack voltage', exact: true }).tap();
        await picker.getByRole('alert').filter({ hasText: 'Test save failure' }).waitFor();
        assert.equal(
          (await client.request('state.get', {})).activeConnection!.liveWidgetSlots[0],
          'fieldWeakeningCurrentA'
        );
        rejectSelection = false;
        await picker.getByRole('button', { name: 'Pack voltage', exact: true }).tap();
        await picker.waitFor({ state: 'hidden' });
        assert.equal(saves, 2, 'one request per activation, including the rejected save');
        assert.match(
          await page.getByRole('button', { name: 'Choose value for box 1: Pack voltage', exact: true }).innerText(),
          /75.0/
        );
        assert.equal(
          await page
            .getByRole('button', { name: 'Choose value for box 1: Pack voltage', exact: true })
            .evaluate((el) => el === document.activeElement || el.matches(':focus-visible')),
          false
        );
        await page.reload();
        const voltageBox = page.getByRole('button', { name: 'Choose value for box 1: Pack voltage', exact: true });
        await voltageBox.waitFor();
        assert.match(await voltageBox.innerText(), /75.0/);
        const secondBox = page.getByRole('button', {
          name: 'Choose value for box 2: Actual motor current',
          exact: true,
        });
        await secondBox.focus();
        await page.keyboard.press('Space');
        await picker.waitFor();
        await picker.getByRole('button', { name: 'Charging voltage', exact: true }).scrollIntoViewIfNeeded();
        await picker.getByRole('button', { name: 'Charging voltage', exact: true }).focus();
        await page.keyboard.press('Enter');
        await picker.waitFor({ state: 'hidden' });
        assert.equal(saves, 3);
        assert.equal(
          await page
            .getByRole('button', { name: 'Choose value for box 2: Charging voltage', exact: true })
            .evaluate((el) => el === document.activeElement && el.matches(':focus-visible')),
          true
        );
        assert.equal((await client.request('state.get', {})).activeConnection!.liveWidgetSlots[1], 'chargingVoltageV');
        padState = 'left';
        const currentProfile = await client.request('boardProfile.get', { savedDeviceId: profile.savedDeviceId });
        await client.request('boardProfile.update', {
          savedDeviceId: profile.savedDeviceId,
          expectedRevision: currentProfile.revision,
          patch: {
            liveWidgetSlots: ['footpadAdc1V', 'footpadAdc2V', 'motorTempC', 'batteryLevel', 'distanceM'],
          },
        });
        const leftPad = page.getByRole('button', { name: 'Choose value for box 1: Footpad ADC 1', exact: true });
        const rightPad = page.getByRole('button', { name: 'Choose value for box 2: Footpad ADC 2', exact: true });
        await leftPad.locator('.text-success').waitFor();
        await rightPad.locator('.text-danger').waitFor();
        await page
          .getByRole('button', { name: 'Choose value for box 3: Motor temperature', exact: true })
          .locator('.text-danger')
          .waitFor();
        await page
          .getByRole('button', { name: 'Choose value for box 4: Controller-reported battery level', exact: true })
          .locator('.text-danger')
          .waitFor();
        await rightPad.focus();
        await page.keyboard.press('Enter');
        await picker.waitFor();
        await picker.getByRole('button', { name: 'Cancel', exact: true }).tap();
        await picker.waitFor({ state: 'hidden' });
        for (const theme of ['light', 'dark'] as const) {
          await page.emulateMedia({ colorScheme: theme });
          await page.waitForFunction((theme) => document.documentElement.dataset.theme === theme, theme);
          await page.waitForTimeout(200);
          if (process.env.SCREENSHOT_DIR)
            await page.screenshot({ path: `${process.env.SCREENSHOT_DIR}/colours-${name}-${theme}.png` });
        }
        await rightPad.focus();
        await page.keyboard.press('Enter');
        await picker.getByRole('button', { name: 'Footpad ADC 2', exact: true }).locator('.text-danger').waitFor();
        rideSpeed = 0;
        await picker
          .getByRole('button', { name: 'Footpad ADC 2', exact: true })
          .locator('.font-mono.text-muted')
          .waitFor();
        await picker.getByRole('button', { name: 'Cancel', exact: true }).tap();
        await picker.waitFor({ state: 'hidden' });
        await rightPad.locator('.font-mono.text-muted').waitFor();
        assert.equal(
          await rightPad.evaluate((el) => el === document.activeElement || el.matches(':focus-visible')),
          false
        );
        const bmsProfile = await client.request('boardProfile.get', { savedDeviceId: profile.savedDeviceId });
        await client.request('boardProfile.update', {
          savedDeviceId: profile.savedDeviceId,
          expectedRevision: bmsProfile.revision,
          patch: {
            liveWidgetSlots: [
              'bmsStateOfCharge',
              'highestTempC',
              'bmsCellVoltageSpreadV',
              'bmsHighestCellTempC',
              'distanceM',
            ],
          },
        });
        chargingBms = widgetBms;
        chargingBms.status = 'attached';
        refreshBmsTime = true;
        Object.assign(chargingBms!.latestSnapshot!, {
          stateOfCharge: 0.6,
          temperaturesC: [80],
          highestCellTemperatureC: 80,
          highestCircuitryTemperatureC: 30,
          cellVoltagesV: [4, 4.1],
        });
        const bmsLevel = page.getByRole('button', { name: 'Choose value for box 1: BMS battery level', exact: true });
        const hottest = page.getByRole('button', { name: 'Choose value for box 2: Highest temperature', exact: true });
        await page.waitForFunction(() =>
          document
            .querySelector('[aria-label="Choose value for box 1: BMS battery level"]')
            ?.textContent?.includes('60')
        );
        await hottest.getByText('Battery temp', { exact: true }).waitFor();
        await page.waitForFunction(() =>
          document
            .querySelector('[aria-label="Choose value for box 2: Highest temperature"]')
            ?.textContent?.includes('80.0')
        );
        await hottest.focus();
        await page.keyboard.press('Enter');
        await picker
          .getByRole('button', { name: 'Highest temperature', exact: true })
          .getByText('Battery temp', { exact: true })
          .waitFor();
        await page.keyboard.press('Escape');
        refreshBmsTime = false;
        await page.waitForFunction(
          () =>
            document
              .querySelector('[aria-label="Choose value for box 1: BMS battery level"]')
              ?.textContent?.includes('—'),
          undefined,
          { timeout: 8000 }
        );
        await hottest.getByText('Motor temp', { exact: true }).waitFor();
        chargingBms!.status = 'not_found';
        await hottest.focus();
        await page.keyboard.press('Enter');
        await picker
          .getByRole('button', { name: 'BMS battery level', exact: true })
          .getByText('Unavailable', { exact: true })
          .waitFor();
        await page.keyboard.press('Escape');
        await page.reload();
        await bmsLevel.waitFor();
        assert.deepEqual((await client.request('state.get', {})).activeConnection!.liveWidgetSlots, [
          'bmsStateOfCharge',
          'highestTempC',
          'bmsCellVoltageSpreadV',
          'bmsHighestCellTempC',
          'distanceM',
        ]);
        assert.deepEqual(errors, []);
      } finally {
        await browser.close();
        client.close();
        await stopMockServer();
      }
    }
  );
}
