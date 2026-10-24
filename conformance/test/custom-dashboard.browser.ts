import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';
import type { CustomDashboard } from '../../frontend/src/lib/backend/contract.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(
    `${name}: custom dashboard composition, responsive editing and shared warnings`,
    { timeout: 120_000 },
    async () => {
      const browser = await engine.launch();
      const port = await startMockServer(0, { safetyDisclaimerAccepted: true });
      const endpoint = getMockBootstrap(port);
      const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
      try {
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
        const id = ready.activeConnection!.savedDeviceId!;
        let profile = await client.request('boardProfile.get', { savedDeviceId: id });
        const dashboard: CustomDashboard = {
          version: 1,
          widgets: [
            { id: 'speed', kind: 'reading', field: 'speedKph', presentation: 'number', size: 'compact' },
            { id: 'lights', kind: 'control', control: 'headlights', size: 'compact' },
            { id: 'trend', kind: 'reading', field: 'motorCurrentA', presentation: 'trend', size: 'large' },
            { id: 'bms', kind: 'reading', field: 'bmsCellVoltageSpreadV', presentation: 'number', size: 'compact' },
            { id: 'battery', kind: 'visual', visual: 'battery', size: 'medium' },
          ],
        };
        profile = await client.request('boardProfile.update', {
          savedDeviceId: id,
          expectedRevision: profile.revision,
          patch: { customDashboard: dashboard, lastLiveView: 'custom' },
        });
        let signedCurrent: number | undefined;
        let loadSource: 'duty' | 'motor' | 'battery' = 'duty';
        const output = new URL(`../generated/custom-dashboard/${name}/`, import.meta.url);
        await mkdir(output, { recursive: true });
        const methods: string[] = [];
        let controlRequests = 0;
        const errors: string[] = [];
        let failSave = false,
          failLoad = false,
          reconnecting = false,
          disabled = false,
          guest = false,
          compactBms = false;
        const page = await browser.newPage({
          viewport: { width: 390, height: 844 },
          hasTouch: true,
          reducedMotion: 'reduce',
        });
        page.setDefaultTimeout(7000);
        if (name === 'chromium')
          await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
        page.on('pageerror', (e) => errors.push(e.message));
        await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
        await page.route(`${endpoint.origin}/api/bootstrap`, (route) =>
          route.fulfill({ json: { webSocketUrl: endpoint.webSocketUrl, platform: 'web' } })
        );
        await page.routeWebSocket('**/api/ws?*', (socket) => {
          const server = socket.connectToServer();
          socket.onMessage((raw) => {
            const message = JSON.parse(String(raw));
            methods.push(message.method);
            if (failLoad && message.method === 'boardProfile.get') {
              failLoad = false;
              socket.send(
                JSON.stringify({
                  id: message.id,
                  error: { code: 'unavailable', message: 'Profile temporarily unavailable' },
                })
              );
            } else if (failSave && message.method === 'boardProfile.update') {
              failSave = false;
              socket.send(JSON.stringify({ id: message.id, error: { code: 'conflict', message: 'Profile changed' } }));
            } else server.send(raw);
          });
          server.onMessage((raw) => {
            const message = JSON.parse(String(raw));
            const state =
              message.event === 'state.changed'
                ? message.data
                : message.result?.activeConnection
                  ? message.result
                  : null;
            if (state?.activeConnection) {
              if (signedCurrent !== undefined && state.activeConnection.latestSample)
                state.activeConnection.latestSample.motorCurrentA = signedCurrent;
              if (state.activeConnection.latestSample)
                state.activeConnection.latestSample.load = {
                  duty: 0.42,
                  motor: 0.61,
                  battery: 0.73,
                  winner: { source: loadSource, fraction: { duty: 0.42, motor: 0.61, battery: 0.73 }[loadSource] },
                };
              if (compactBms)
                state.activeConnection.bms = {
                  ...state.activeConnection.bms,
                  status: 'attached',
                  connection: { kind: 'can', canId: 42 },
                  candidates: [],
                  latestSnapshot: null,
                };
              if (reconnecting) state.activeConnection.state = 'reconnecting';
              state.activeConnection.refloatControls.packageEnabled = !disabled;
              state.activeConnection.refloatControls.flywheelEnabled = false;
              if (guest) {
                state.activeConnection.trackingMode = 'guest';
                state.activeConnection.savedDeviceId = null;
              }
            }
            socket.send(JSON.stringify(message));
          });
        });
        await page.goto(`${endpoint.origin}/#/live`);
        await page.getByRole('button', { name: 'Board actions', exact: true }).click();
        await page.getByRole('button', { name: 'Custom View', exact: true }).click();
        const custom = page.getByRole('region', { name: 'Custom dashboard', exact: true });
        const enterEditing = async () => {
          const card = custom.locator('[data-dashboard-widget]').first();
          await card.scrollIntoViewIfNeeded();
          const box = (await card.boundingBox())!;
          await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
          await page.mouse.down();
          await custom.getByRole('toolbar', { name: 'Edit dashboard' }).waitFor();
          const after = (await card.boundingBox())!;
          for (const dimension of ['x', 'y', 'width', 'height'] as const)
            assert.ok(Math.abs(after[dimension] - box[dimension]) < 1, `editing must not shift widget ${dimension}`);
          await page.mouse.up();
        };
        const chooseSize = async (size: 'compact' | 'medium' | 'large') => {
          const shape = { compact: [1, 1], medium: [2, 2], large: [4, 2] }[size];
          await page
            .getByRole('button', {
              name: `${shape[0]} ${shape[0] === 1 ? 'column' : 'columns'} by ${shape[1]} ${shape[1] === 1 ? 'row' : 'rows'}`,
              exact: true,
            })
            .click();
        };
        const assertSize = async (columns: number, rows: number) => {
          assert.equal(
            await page
              .getByRole('button', {
                name: `${columns} ${columns === 1 ? 'column' : 'columns'} by ${rows} ${rows === 1 ? 'row' : 'rows'}`,
                exact: true,
              })
              .getAttribute('aria-pressed'),
            'true'
          );
        };
        const choosePresentation = async (presentation: string) => {
          const label = { number: 'Number', bar: 'Bar', gauge: 'Gauge', trend: 'Chart' }[presentation]!;
          await page
            .getByRole('group', { name: 'Display as', exact: true })
            .getByRole('button', { name: label, exact: true })
            .click();
        };
        const viewing = () =>
          custom.getByText('Press and hold any widget to arrange your dashboard.', { exact: true }).waitFor();

        await custom.locator('[data-dashboard-widget]').first().waitFor();
        assert.equal(await custom.locator('[data-dashboard-widget]').count(), 5);
        assert.equal(methods.filter((m) => m === 'boardProfile.update').length, 0, 'opening must not write defaults');
        assert.match(await custom.locator('[data-dashboard-widget="bms"]').innerText(), /Unavailable/);

        for (const { columns, ...viewport } of [
          { width: 280, height: 653, columns: 2 },
          { width: 390, height: 844, columns: 4 },
          { width: 844, height: 390, columns: 8 },
          { width: 768, height: 900, columns: 8 },
          { width: 1280, height: 900, columns: 12 },
          { width: 1600, height: 1000, columns: 12 },
          { width: 2560, height: 1200, columns: 12 },
        ]) {
          await page.setViewportSize(viewport);
          const result = await custom.evaluate((el) => {
            const cards = [...el.querySelectorAll<HTMLElement>('[data-dashboard-widget]')];
            return {
              columns: getComputedStyle(el.querySelector('.dashboard-grid')!).gridTemplateColumns.split(/\s+/).length,
              shellWidth: el.closest('.app-content')!.getBoundingClientRect().width,
              ids: cards.map((c) => c.dataset.dashboardWidget),
              boxes: cards.map((c) => {
                const r = c.getBoundingClientRect();
                return { x: r.x, y: r.y, width: r.width, height: r.height };
              }),
              overflow: document.documentElement.scrollWidth > innerWidth,
            };
          });
          assert.deepEqual(
            result.ids,
            dashboard.widgets.map((w) => w.id)
          );
          assert.equal(result.overflow, false, `${name} overflow at ${viewport.width}`);
          assert.equal(result.columns, columns, `usable grid columns at ${viewport.width}`);
          assert.ok(result.shellWidth <= 1600, 'ultrawide dashboards keep a bounded content width');
          assert.ok(result.boxes.every((b) => b.width >= 64));
          for (let index = 1; index < result.boxes.length; index++) {
            const previous = result.boxes[index - 1]!,
              next = result.boxes[index]!;
            assert.ok(
              next.y >= previous.y - 1 && (next.y > previous.y + 1 || next.x >= previous.x),
              'visual reading order follows saved order at each width'
            );
          }
          for (let i = 0; i < result.boxes.length; i++)
            for (let j = i + 1; j < result.boxes.length; j++) {
              const a = result.boxes[i]!,
                b = result.boxes[j]!;
              assert.ok(
                a.x + a.width <= b.x + 1 ||
                  b.x + b.width <= a.x + 1 ||
                  a.y + a.height <= b.y + 1 ||
                  b.y + b.height <= a.y + 1,
                'packed widgets must not overlap'
              );
            }
        }
        assert.equal(
          methods.filter((m) => m === 'boardProfile.update').length,
          0,
          'responsive repacking never writes the layout'
        );
        await page.setViewportSize({ width: 390, height: 844 });
        assert.equal(await page.getByRole('button', { name: 'Edit dashboard', exact: true }).count(), 0);
        await custom.getByRole('switch', { name: 'Headlights', exact: true }).tap();
        await page.waitForFunction(() => !document.querySelector('[role="switch"][aria-busy="true"]'));
        assert.equal(methods.filter((m) => m === 'refloat.control.set').length, 1, 'a short tap still toggles once');
        controlRequests = 1;
        if (name === 'chromium') {
          const cdp = await page.context().newCDPSession(page);
          const box = (await custom.locator('[data-dashboard-widget="lights"]').boundingBox())!;
          const x = box.x + box.width / 2,
            y = box.y + box.height / 2;
          await cdp.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [{ x, y }] });
          await cdp.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: [{ x, y: y - 40 }] });
          await cdp.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
          assert.equal(await custom.getByText('Editing dashboard', { exact: true }).count(), 0);
          assert.equal(methods.filter((m) => m === 'refloat.control.set').length, controlRequests);
          assert.equal(await custom.locator('[data-pressing]').count(), 0, 'scrolling cancels press feedback');
          await custom.locator('[data-dashboard-widget="lights"]').scrollIntoViewIfNeeded();
          const held = (await custom.locator('[data-dashboard-widget="lights"]').boundingBox())!;
          await cdp.send('Input.dispatchTouchEvent', {
            type: 'touchStart',
            touchPoints: [{ x: held.x + held.width / 2, y: held.y + held.height / 2 }],
          });
          await custom.getByText('Editing dashboard', { exact: true }).waitFor();
          await cdp.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
          assert.equal(methods.filter((m) => m === 'refloat.control.set').length, controlRequests);
          await custom.getByRole('button', { name: 'Cancel', exact: true }).click();
          await viewing();
          await cdp.detach();
        }

        // A hold on the actual lighting control enters editing and continues dragging
        // without releasing, sending a command or opening configuration.
        const heldLight = custom.locator('[data-dashboard-widget="lights"]');
        const heldBox = (await heldLight.boundingBox())!;
        await page.mouse.move(heldBox.x + heldBox.width / 2, heldBox.y + heldBox.height / 2);
        await page.mouse.down();
        await custom.getByText('Editing dashboard', { exact: true }).waitFor();
        const speedBox = (await custom.locator('[data-dashboard-widget="speed"]').boundingBox())!;
        await page.mouse.move(speedBox.x + speedBox.width / 2, speedBox.y + speedBox.height / 2, { steps: 5 });
        assert.equal(await page.locator('body > [aria-hidden="true"][inert]').count(), 1);
        assert.equal(
          await heldLight.evaluate((el) => (el as HTMLElement).style.order),
          '0',
          'preview makes room before drop'
        );
        const placeholder = (await heldLight.boundingBox())!;
        await page.mouse.move(placeholder.x + placeholder.width + 3, placeholder.y + placeholder.height / 2);
        assert.equal(await heldLight.getAttribute('data-drop'), 'true', 'visible placeholder stays active over a gap');
        assert.equal(await custom.locator('[data-drop]').count(), 1);
        await page.mouse.up();
        assert.equal(methods.filter((m) => m === 'refloat.control.set').length, controlRequests);
        assert.deepEqual(
          await custom
            .locator('[data-dashboard-widget]')
            .evaluateAll((els) => els.map((el) => el.getAttribute('data-dashboard-widget'))),
          ['lights', 'speed', 'trend', 'bms', 'battery']
        );
        await custom.getByRole('button', { name: 'Cancel', exact: true }).click();
        await page.getByRole('button', { name: 'Discard changes', exact: true }).click();
        await viewing();
        await enterEditing();
        const removeBadge = await custom
          .locator('[data-dashboard-remove] > span')
          .first()
          .evaluate((el) => ({
            width: el.getBoundingClientRect().width,
            height: el.getBoundingClientRect().height,
            color: getComputedStyle(el).color,
            ordinary: getComputedStyle(el.closest('article')!).color,
            background: getComputedStyle(el).backgroundColor,
            target: el.parentElement!.getBoundingClientRect().width,
            borderLayer: Number(getComputedStyle(el.closest('article')!, '::before').zIndex),
            buttonLayer: Number(getComputedStyle(el.parentElement!).zIndex),
            parentOutline: getComputedStyle(el.closest('article')!).outlineStyle,
            cornerX: Math.abs(
              el.getBoundingClientRect().x +
                el.getBoundingClientRect().width / 2 -
                el.closest('article')!.getBoundingClientRect().right
            ),
            cornerY: Math.abs(
              el.getBoundingClientRect().y +
                el.getBoundingClientRect().height / 2 -
                el.closest('article')!.getBoundingClientRect().top
            ),
            overlap: [...el.closest('article')!.querySelectorAll('.live-widget-title')].some((title) => {
              const badge = el.getBoundingClientRect(),
                text = title.getBoundingClientRect();
              const x = Math.max(text.left, Math.min(badge.x + badge.width / 2, text.right));
              const y = Math.max(text.top, Math.min(badge.y + badge.height / 2, text.bottom));
              return Math.hypot(x - (badge.x + badge.width / 2), y - (badge.y + badge.height / 2)) < badge.width / 2;
            }),
          }));
        assert.ok(
          removeBadge.width === 28 && removeBadge.height === 28 && removeBadge.color !== removeBadge.ordinary,
          'delete badge is compact and visibly danger styled'
        );
        assert.equal(removeBadge.target, 44);
        assert.ok(
          removeBadge.borderLayer < removeBadge.buttonLayer && removeBadge.parentOutline === 'none',
          'edit border is painted below the opaque delete badge'
        );
        assert.ok(
          removeBadge.cornerX <= 6 &&
            removeBadge.cornerY <= 6 &&
            Math.abs(removeBadge.cornerX - removeBadge.cornerY) < 1,
          'badge straddles the tile corner with equal offsets'
        );
        assert.equal(removeBadge.overlap, false, 'badge clears the title');
        assert.ok(!removeBadge.background.endsWith(', 0)'), 'badge background stays opaque');
        await page.emulateMedia({ reducedMotion: 'no-preference' });
        const wiggle = await custom
          .locator('[data-dashboard-widget]')
          .first()
          .evaluate((card) => {
            const border = getComputedStyle(card, '::before'),
              content = getComputedStyle(card.querySelector(':scope > div')!);
            return {
              border: border.animationName,
              content: content.animationName,
              difference: Math.abs(parseFloat(border.rotate) - parseFloat(content.rotate)),
            };
          });
        assert.ok(
          wiggle.border.includes('wiggle') && wiggle.border === wiggle.content && wiggle.difference < 0.001,
          'editing border and content wiggle together'
        );
        const toolbarShadow = await custom
          .getByRole('toolbar', { name: 'Edit dashboard' })
          .evaluate((el) => getComputedStyle(el).boxShadow);
        assert.ok(
          (toolbarShadow.match(/rgba?\(|color\(/g)?.length ?? 0) >= 3,
          'floating toolbar has layered accent and depth shadows'
        );
        const badgesAboveNeighbours = await custom.locator('[data-dashboard-remove] > span').evaluateAll((badges) =>
          badges.every((badge) => {
            const r = badge.getBoundingClientRect();
            const button = badge.closest('[data-dashboard-remove]');
            return [
              [0.5, 0.5],
              [0.75, 0.75],
            ].every(
              ([x, y]) =>
                document
                  .elementFromPoint(r.x + r.width * x!, r.y + r.height * y!)
                  ?.closest('[data-dashboard-remove]') === button
            );
          })
        );
        assert.equal(badgesAboveNeighbours, true, 'badges remain above neighbouring tiles while wiggling');
        await page.emulateMedia({ reducedMotion: 'reduce' });
        assert.deepEqual(
          await custom
            .locator('[data-dashboard-widget]')
            .first()
            .evaluate((card) => [
              getComputedStyle(card, '::before').animationName,
              getComputedStyle(card.querySelector(':scope > div')!).animationName,
            ]),
          ['none', 'none'],
          'reduced motion disables both animations'
        );
        await page.screenshot({ path: new URL('delete-badges.png', output).pathname });
        for (const width of [390, 768, 1280, 2560]) {
          await page.setViewportSize({ width, height: 844 });
          for (const button of await custom.locator('[data-dashboard-remove]').all()) {
            await button.focus();
            const boundary = await button.evaluate((el) => {
              const scroller = el.closest('.dashboard-scroll') as HTMLElement;
              const clip = scroller.getBoundingClientRect(),
                target = el.getBoundingClientRect();
              return {
                left: scroller.scrollLeft,
                extraWidth: scroller.scrollWidth - scroller.clientWidth,
                contained: target.left >= clip.left - 1 && target.right <= clip.right + 1,
              };
            });
            assert.equal(boundary.left, 0, 'focusing delete never scrolls the grid sideways');
            assert.ok(
              boundary.extraWidth <= 1 && boundary.contained,
              `delete target stays inside the scroll frame: ${JSON.stringify(boundary)}`
            );
          }
        }
        await page.setViewportSize({ width: 390, height: 844 });

        const removeLight = custom.getByRole('button', { name: 'Remove Headlights', exact: true });
        await removeLight.click({ button: 'right' });
        assert.equal(await custom.locator('[data-dashboard-widget]').count(), 5);
        const removeBox = (await removeLight.boundingBox())!;
        await page.mouse.move(removeBox.x + removeBox.width / 2, removeBox.y + removeBox.height / 2);
        await page.mouse.down();
        await page.mouse.move(removeBox.x - 25, removeBox.y + 30);
        await page.mouse.up();
        assert.equal(await custom.locator('[data-dashboard-widget]').count(), 5, 'moving off remove must cancel');
        await removeLight.tap();
        assert.equal(await custom.locator('[data-dashboard-widget]').count(), 4);
        assert.equal(await page.getByRole('heading', { name: 'Configure widget', exact: true }).count(), 0);
        assert.equal(methods.filter((m) => m === 'refloat.control.set').length, controlRequests);
        assert.equal(methods.filter((m) => m === 'boardProfile.update').length, 0, 'removal stays in the draft');
        await custom.getByRole('button', { name: 'Cancel', exact: true }).click();
        await page.getByRole('button', { name: 'Discard changes', exact: true }).click();
        await viewing();
        await enterEditing();
        assert.equal(
          await custom
            .locator('.editing')
            .first()
            .evaluate((el) => getComputedStyle(el).animationName),
          'none'
        );
        const order = () =>
          custom
            .locator('[data-dashboard-widget]')
            .evaluateAll((els) => els.map((el) => el.getAttribute('data-dashboard-widget')));
        const drag = async (from: string, to: string) => {
          const a = (await custom.locator(`[data-dashboard-widget="${from}"]`).boundingBox())!;
          const b = (await custom.locator(`[data-dashboard-widget="${to}"]`).boundingBox())!;
          await page.mouse.move(a.x + a.width / 2, a.y + a.height / 2);
          await page.mouse.down();
          await page.mouse.move(b.x + b.width / 2, b.y + b.height / 2, { steps: 5 });
          await page.mouse.up();
        };
        await page.emulateMedia({ reducedMotion: 'no-preference' });
        await drag('speed', 'lights');
        assert.deepEqual((await order()).slice(0, 2), ['lights', 'speed']);
        assert.equal(
          await page.getByRole('heading', { name: 'Configure widget', exact: true }).count(),
          0,
          'drag must suppress tap'
        );
        await drag('lights', 'speed');
        await page.emulateMedia({ reducedMotion: 'reduce' });
        assert.deepEqual((await order()).slice(0, 2), ['speed', 'lights']);
        // Reflow can put the same neighbour back beneath a stationary pointer.
        for (const width of [390, 1280]) {
          await page.setViewportSize({ width, height: 844 });
          for (const reducedMotion of ['reduce', 'no-preference'] as const) {
            await page.emulateMedia({ reducedMotion });
            // Exercise both two-row charts and the four-row gauge/chart footprint.
            await custom.locator('[data-dashboard-widget="trend"]').evaluate(
              (card, rows) => {
                (card as HTMLElement).style.gridRow = `span ${rows}`;
              },
              reducedMotion === 'reduce' ? 2 : 4
            );
            const source = custom.locator('[data-dashboard-widget="speed"]');
            await source.scrollIntoViewIfNeeded();
            const a = (await source.boundingBox())!;
            const b = (await custom.locator('[data-dashboard-widget="trend"]').boundingBox())!;
            const point = { x: b.x + b.width / 2, y: b.y + b.height / 2 };
            const previewOrder = () =>
              custom
                .locator('[data-dashboard-widget]')
                .evaluateAll((cards) => cards.map((card) => (card as HTMLElement).style.order));
            const touch = name === 'chromium' && width === 390 ? await page.context().newCDPSession(page) : null;
            const move = async (x: number, y: number) => {
              if (touch) {
                await touch.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: [{ x, y }] });
                // Chromium can coalesce touch moves until the next rendering frame.
                await page.evaluate(() => new Promise<void>((resolve) => requestAnimationFrame(() => resolve())));
              } else await page.mouse.move(x, y);
            };
            if (touch) {
              await touch.send('Input.dispatchTouchEvent', {
                type: 'touchStart',
                touchPoints: [{ x: a.x + a.width / 2, y: a.y + a.height / 2 }],
              });
              await page.waitForFunction(() => !!document.querySelector('[data-dragging]'));
            } else {
              await page.mouse.move(a.x + a.width / 2, a.y + a.height / 2);
              await page.mouse.down();
            }
            await move(point.x, point.y);
            const placed = await previewOrder();
            assert.ok(
              placed.some((value) => value !== ''),
              'drag creates a proposed placement'
            );
            for (let step = 0; step < 12; step++) {
              await move(point.x + (step % 2 ? 2 : -2), point.y);
              assert.deepEqual(await previewOrder(), placed, `preview stays stable at ${width}px (${reducedMotion})`);
            }
            // Continued forward travel beyond the hysteresis must not toggle either.
            for (let y = point.y + 24; y < b.y + b.height - 8; y += 24) {
              await move(point.x, y);
              assert.deepEqual(
                await previewOrder(),
                placed,
                `forward travel stays stable at ${width}px (${reducedMotion})`
              );
            }
            if (width === 390) {
              await move(point.x, b.y + b.height / 4);
              assert.notDeepEqual(await previewOrder(), placed, 'crossing the original midpoint reverses placement');
              await move(point.x, b.y + (b.height * 3) / 4);
              assert.deepEqual(await previewOrder(), placed, 'crossing back restores placement without oscillation');
            }
            // Target its final slot rather than a painted position midway through FLIP.
            await custom
              .locator('[data-dashboard-widget]')
              .evaluateAll((cards) =>
                cards.forEach((card) => card.getAnimations().forEach((animation) => animation.finish()))
              );
            const retreat = (await custom.locator('[data-dashboard-widget="lights"]').boundingBox())!;
            await move(retreat.x + retreat.width / 2, retreat.y + retreat.height / 2);
            assert.notDeepEqual(await previewOrder(), placed, `deliberate reversal at ${width}px (${reducedMotion})`);
            await page.evaluate(() => window.dispatchEvent(new Event('blur')));
            if (touch) {
              await touch.send('Input.dispatchTouchEvent', { type: 'touchCancel', touchPoints: [] });
              await touch.detach();
            } else await page.mouse.up();
            assert.ok(
              (await previewOrder()).every((value) => value === ''),
              'cancellation clears preview order'
            );
          }
        }
        await custom.locator('[data-dashboard-widget="trend"]').evaluate((card) => {
          (card as HTMLElement).style.removeProperty('grid-row');
        });
        await page.setViewportSize({ width: 390, height: 844 });
        await page.emulateMedia({ reducedMotion: 'reduce' });
        const beforeCancel = await order();
        const speedEdit = custom.getByRole('button', { name: /^Configure Speed,/ });
        await speedEdit.focus();
        await page.keyboard.press('Enter');
        await page.getByRole('heading', { name: 'Configure widget', exact: true }).waitFor();
        await page.keyboard.press('Escape');
        await speedEdit.waitFor();
        assert.equal(await speedEdit.evaluate((el) => el === document.activeElement), true);
        reconnecting = true;
        await page.waitForFunction(() => !!document.querySelector('.custom-widget-content.opacity-45'));
        assert.deepEqual(await order(), beforeCancel);
        reconnecting = false;
        await page.waitForFunction(() => !document.querySelector('.custom-widget-content.opacity-45'));
        assert.match(await custom.innerText(), /Editing dashboard/);
        await speedEdit.dispatchEvent('pointerdown', {
          pointerId: 1,
          isPrimary: false,
          pointerType: 'mouse',
          button: 2,
        });
        await speedEdit.dispatchEvent('pointerup', { pointerId: 1, isPrimary: false, pointerType: 'mouse', button: 2 });
        assert.deepEqual(await order(), beforeCancel);
        assert.equal(await page.getByRole('heading', { name: 'Configure widget', exact: true }).count(), 0);
        // Real touch streams exercise browser capture and scroll/hold arbitration in Chromium.
        if (name === 'chromium') {
          const cdp = await page.context().newCDPSession(page);
          const box = (await speedEdit.boundingBox())!;
          const x = box.x + box.width / 2,
            y = box.y + box.height / 2;
          await cdp.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [{ x, y }] });
          await cdp.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: [{ x, y: y - 70 }] });
          await cdp.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
          assert.deepEqual(await order(), beforeCancel);
          assert.equal(await page.getByRole('heading', { name: 'Configure widget', exact: true }).count(), 0);
          await custom.locator('[data-dashboard-widget="speed"]').scrollIntoViewIfNeeded();
          const a = (await speedEdit.boundingBox())!;
          const b = (await custom.locator('[data-dashboard-widget="lights"]').boundingBox())!;
          await cdp.send('Input.dispatchTouchEvent', {
            type: 'touchStart',
            touchPoints: [{ x: a.x + a.width / 2, y: a.y + a.height / 2 }],
          });
          await page.waitForFunction(() => !!document.querySelector('[data-dragging]'));
          await cdp.send('Input.dispatchTouchEvent', {
            type: 'touchMove',
            touchPoints: [{ x: b.x + b.width / 2, y: b.y + b.height / 2 }],
          });
          await cdp.send('Input.dispatchTouchEvent', { type: 'touchCancel', touchPoints: [] });
          assert.deepEqual(await order(), beforeCancel, 'cancelled touch drag must not reorder');
          assert.equal(await custom.locator('[data-pressing]').count(), 0);
          assert.equal(await custom.locator('[data-dashboard-widget][style*="order"]').count(), 0);
          await cdp.send('Input.dispatchTouchEvent', {
            type: 'touchStart',
            touchPoints: [{ x: a.x + a.width / 2, y: a.y + a.height / 2 }],
          });
          await page.waitForFunction(() => !!document.querySelector('[data-dragging]'));
          await cdp.send('Input.dispatchTouchEvent', {
            type: 'touchMove',
            touchPoints: [{ x: b.x + b.width / 2, y: b.y + b.height / 2 }],
          });
          const destination = (await custom.locator('[data-dashboard-widget="speed"]').boundingBox())!;
          await cdp.send('Input.dispatchTouchEvent', {
            type: 'touchMove',
            touchPoints: [{ x: destination.x - 3, y: destination.y + destination.height / 2 }],
          });
          assert.equal(await custom.locator('[data-dashboard-widget="speed"]').getAttribute('data-drop'), 'true');
          await cdp.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
          assert.deepEqual(
            (await order()).slice(0, 2),
            ['lights', 'speed'],
            'touch release over a gap commits the visible placeholder'
          );
          await speedEdit.focus();
          await page.keyboard.press('Alt+ArrowLeft');
          assert.deepEqual(await order(), beforeCancel);
          await cdp.detach();
        }
        const light = custom.getByRole('button', { name: /^Configure Headlights/ });
        const lightFootprint = (await custom.locator('[data-dashboard-widget="lights"]').boundingBox())!;
        await light.tap();
        await page.getByRole('heading', { name: 'Configure widget', exact: true }).waitFor();
        const previewFootprint = (await page.locator('[aria-label="Widget preview"]').boundingBox())!;
        assert.ok(
          Math.abs(previewFootprint.width - lightFootprint.width) < 1 &&
            Math.abs(previewFootprint.height - lightFootprint.height) < 1,
          `compact preview uses the actual dashboard footprint: ${JSON.stringify({ previewFootprint, lightFootprint })}`
        );
        await assertSize(1, 1);
        await chooseSize('medium');
        const mediumPreview = (await page.locator('[aria-label="Widget preview"]').boundingBox())!;
        assert.ok(
          Math.abs(mediumPreview.height - (lightFootprint.height * 2 + 6)) < 1 &&
            Math.abs(mediumPreview.width - (lightFootprint.width * 2 + 6)) < 1,
          'preview updates to a real 2×2 footprint'
        );
        await chooseSize('compact');
        assert.equal(methods.filter((m) => m === 'refloat.control.set').length, controlRequests);
        assert.equal(await page.getByRole('button', { name: /^Move (earlier|later)$/ }).count(), 0);
        assert.equal(await page.getByRole('button', { name: 'Remove widget', exact: true }).count(), 0);
        assert.equal(await page.getByRole('button', { name: 'Apply to dashboard', exact: true }).count(), 0);
        await page.getByRole('button', { name: 'Save', exact: true }).click();
        await light.focus();
        await page.keyboard.press('Alt+ArrowLeft');
        assert.equal(await light.evaluate((el) => el === document.activeElement), true);
        assert.deepEqual(
          await custom
            .locator('[data-dashboard-widget]')
            .evaluateAll((els) => els.map((e) => e.getAttribute('data-dashboard-widget'))),
          ['lights', 'speed', 'trend', 'bms', 'battery']
        );
        await custom.getByRole('button', { name: /^Configure Speed,/ }).click();
        await choosePresentation('bar');
        await chooseSize('large');
        await page.getByLabel('Custom display range', { exact: false }).check();
        await page.getByLabel('Minimum', { exact: true }).fill('80');
        await page.getByLabel('Maximum', { exact: true }).fill('60');
        assert.equal(await page.getByRole('button', { name: 'Save', exact: true }).isDisabled(), true);
        await page.getByLabel('Minimum', { exact: true }).fill('0');
        await page.getByLabel('Maximum', { exact: true }).fill('80');
        await page.getByRole('button', { name: 'Save', exact: true }).click();
        await page.setViewportSize({ width: 1000, height: 800 });
        assert.match(await custom.innerText(), /Editing dashboard/);
        await custom.getByRole('button', { name: 'Save dashboard', exact: true }).click();
        await viewing();
        const saved = await client.request('boardProfile.get', { savedDeviceId: id });
        assert.deepEqual(
          saved.values.customDashboard!.widgets.map((w) => w.id),
          ['lights', 'speed', 'trend', 'bms', 'battery']
        );
        assert.deepEqual(saved.values.liveWidgetSlots, profile.values.liveWidgetSlots);
        assert.deepEqual(saved.values.customDashboard!.widgets[1]!.size, { columns: 4, rows: 2 });
        assert.equal(methods.filter((m) => m === 'boardProfile.update').length, 1);

        // A stale revision keeps the draft and requires deliberate reload, without auto-resubmission.
        await enterEditing();
        await custom.getByRole('button', { name: 'Remove Headlights', exact: true }).click();
        failSave = true;
        await custom.getByRole('button', { name: 'Save dashboard', exact: true }).click();
        await page.getByText('This board profile changed.', { exact: false }).waitFor();
        assert.equal(await custom.locator('[data-dashboard-widget]').count(), 4);
        assert.equal(methods.filter((m) => m === 'boardProfile.update').length, 2);
        await custom.getByRole('button', { name: 'Cancel', exact: true }).click();
        await page.getByRole('button', { name: 'Discard changes', exact: true }).click();
        assert.equal(await custom.locator('[data-dashboard-widget]').count(), 5);

        // Warnings stay on the dashboard and disappear behind all ordinary panels.
        disabled = true;
        await page.getByText('Refloat is disabled', { exact: true }).waitFor();
        assert.equal(await page.getByText('Refloat is disabled', { exact: true }).count(), 1);
        await enterEditing();
        await custom.getByRole('button', { name: /^Configure Speed,/ }).click();
        assert.equal(await page.getByText('Refloat is disabled', { exact: true }).isVisible(), false);
        assert.equal(await page.getByRole('dialog').locator('.live-warning-layer').count(), 0);
        assert.equal(await custom.getByRole('toolbar', { name: 'Edit dashboard' }).count(), 0);
        await page.getByRole('button', { name: 'Cancel', exact: true }).last().click();
        await custom.getByRole('button', { name: 'Cancel', exact: true }).click();
        await page.getByRole('button', { name: 'Board actions', exact: true }).click();
        const actionsPanel = page.getByRole('dialog', { name: 'Board actions', exact: true });
        assert.equal(await actionsPanel.locator('.live-warning-layer').count(), 0);
        assert.equal(await page.getByText('Refloat is disabled', { exact: true }).isVisible(), false);
        await actionsPanel.getByRole('button', { name: 'Done', exact: true }).click();
        await page.getByText('Refloat is disabled', { exact: true }).waitFor();
        const faultAction = page.getByRole('button', { name: /^Open fault history:/ });
        await faultAction.click();
        const faults = page.getByRole('dialog', { name: 'Fault history', exact: true });
        await faults.waitFor();
        assert.equal(await faults.locator('.live-warning-layer').count(), 0);
        await faults.getByRole('button', { name: 'Done', exact: true }).click();
        disabled = false;

        // Same-process frontend replacement restores Custom but never editing state or a mutation.
        await page.reload();
        await page.getByRole('region', { name: 'Custom dashboard', exact: true }).waitFor();
        await viewing();
        assert.equal(methods.filter((m) => m === 'boardProfile.update').length, 2);
        assert.equal(methods.filter((m) => m === 'refloat.control.set').length, controlRequests);
        assert.equal(methods.filter((m) => ['session.series', 'session.samples'].includes(m)).length, 0);
        await page.screenshot({ path: new URL('dashboard.png', output).pathname, fullPage: true });
        await page.setViewportSize({ width: 390, height: 844 });
        await page.screenshot({ path: new URL('phone.png', output).pathname, fullPage: true });
        await page.setViewportSize({ width: 1000, height: 800 });
        // A failed read must not become an empty/default dashboard or write one.
        failLoad = true;
        await page.reload();
        await page.getByText('Profile temporarily unavailable', { exact: true }).waitFor();
        assert.equal(await custom.locator('[data-dashboard-widget]').count(), 0);
        assert.equal(methods.filter((m) => m === 'boardProfile.update').length, 2);
        await custom.getByRole('button', { name: 'Retry loading', exact: true }).click();
        await custom.locator('[data-dashboard-widget]').first().waitFor();

        // A 2×2 widget leaves room for four dense 1×1 widgets beside it.
        const beforeCompact = await client.request('boardProfile.get', { savedDeviceId: id });
        await client.request('boardProfile.update', {
          savedDeviceId: id,
          expectedRevision: beforeCompact.revision,
          patch: {
            customDashboard: {
              version: 1,
              widgets: [
                {
                  id: 'medium-current',
                  kind: 'reading',
                  field: 'motorCurrentA',
                  presentation: 'number',
                  size: 'medium',
                },
                { id: 'compact-speed', kind: 'reading', field: 'speedKph', presentation: 'number', size: 'compact' },
                { id: 'compact-lights', kind: 'control', control: 'headlights', size: 'compact' },
                { id: 'compact-battery', kind: 'visual', visual: 'battery', size: 'compact' },
                { id: 'compact-temps', kind: 'visual', visual: 'temperatures', size: 'compact' },
              ],
            },
          },
        });
        compactBms = true;
        await page.setViewportSize({ width: 390, height: 844 });
        await page.reload();
        await custom.locator('[data-dashboard-widget="compact-temps"]').waitFor();
        const compactBoxes = await custom.locator('[data-dashboard-widget][data-size="compact"]').evaluateAll((els) =>
          els.map((el) => {
            const bounds = el.getBoundingClientRect();
            return {
              x: bounds.x,
              y: bounds.y,
              width: bounds.width,
              height: bounds.height,
              overflow:
                el.scrollWidth > el.clientWidth ||
                el.querySelector<HTMLElement>('.custom-widget-content')!.scrollHeight >
                  el.querySelector<HTMLElement>('.custom-widget-content')!.clientHeight + 1,
            };
          })
        );
        const mediumBox = (await custom.locator('[data-dashboard-widget="medium-current"]').boundingBox())!;
        assert.ok(
          Math.abs(mediumBox.height - (compactBoxes[0]!.height * 2 + 6)) < 1,
          'medium occupies two compact rows'
        );
        assert.ok(
          compactBoxes.every((box) => box.x >= mediumBox.x + mediumBox.width + 5),
          'all four compact widgets fit beside the medium widget'
        );
        assert.ok(Math.abs(compactBoxes[0]!.y - mediumBox.y) < 1 && Math.abs(compactBoxes[1]!.y - mediumBox.y) < 1);
        assert.ok(
          Math.abs(compactBoxes[2]!.y - (mediumBox.y + compactBoxes[0]!.height + 6)) < 1 &&
            Math.abs(compactBoxes[3]!.y - compactBoxes[2]!.y) < 1
        );
        assert.ok(
          compactBoxes.every((box) => box.width < 100 && box.height >= 56 && box.height < box.width && !box.overflow),
          JSON.stringify(compactBoxes)
        );
        await page.screenshot({
          path: new URL('mixed-cells.png', output).pathname,
          clip: { x: 0, y: 0, width: 390, height: 370 },
        });
        await enterEditing();
        await custom.getByRole('button', { name: 'Remove Actual motor current', exact: true }).focus();
        await page.keyboard.press('Enter');
        const compactRow = await custom
          .locator('[data-dashboard-widget]')
          .evaluateAll((els) => els.map((el) => el.getBoundingClientRect().y));
        assert.ok(
          compactRow.every((y) => Math.abs(y - compactRow[0]!) < 1),
          'without the medium widget, four compacts share one row'
        );
        await custom.getByRole('button', { name: 'Remove Temperatures', exact: true }).focus();
        await page.keyboard.press('Enter');
        assert.equal(await custom.locator('[data-dashboard-widget]').count(), 3);
        assert.equal(
          await custom
            .getByRole('button', { name: /^Configure Battery,/ })
            .evaluate((el) => el === document.activeElement),
          true
        );
        await custom.getByRole('button', { name: 'Cancel', exact: true }).click();
        await page.getByRole('button', { name: 'Discard changes', exact: true }).click();
        await viewing();

        await enterEditing();
        for (const presentation of ['trend', 'bar', 'gauge']) {
          await custom.getByRole('button', { name: /^Configure Actual motor current,/ }).click();
          await chooseSize('compact');
          await choosePresentation(presentation);
          await page.getByRole('button', { name: 'Save', exact: true }).click();
          const bounds = await custom.locator('[data-dashboard-widget="medium-current"]').evaluate((el) => {
            const content = el.querySelector<HTMLElement>('.custom-widget-content')!;
            const rect = el.getBoundingClientRect();
            return {
              width: rect.width,
              height: rect.height,
              contentHeight: content.clientHeight,
              scrollHeight: content.scrollHeight,
            };
          });
          assert.ok(
            bounds.height < bounds.width && bounds.scrollHeight <= bounds.contentHeight + 1,
            `${presentation}: ${JSON.stringify(bounds)}`
          );
          if (presentation === 'gauge') {
            const geometry = await custom.locator('[data-dashboard-widget="medium-current"]').evaluate((el) => {
              const svg = el.querySelector('.reading-gauge')!;
              const dial = svg.getBoundingClientRect();
              const text = svg.querySelector('.font-mono')!.getBoundingClientRect();
              const content = el.querySelector('.custom-widget-content')!.getBoundingClientRect();
              return {
                centered: Math.abs(text.x + text.width / 2 - (dial.x + dial.width / 2)) < 1,
                fillsCell: dial.height >= content.height - 1,
                numberInsideGauge: svg.contains(el.querySelector('.font-mono')),
              };
            });
            assert.deepEqual(geometry, { centered: true, fillsCell: true, numberInsideGauge: true });
            await custom
              .locator('[data-dashboard-widget="medium-current"]')
              .screenshot({ path: new URL('gauge-compact.png', output).pathname });
          }
        }
        await custom.getByRole('button', { name: 'Cancel', exact: true }).click();
        await page.getByRole('button', { name: 'Discard changes', exact: true }).click();
        await viewing();

        await enterEditing();
        await custom.getByRole('button', { name: /^Configure Actual motor current,/ }).click();
        await page.getByLabel('Content', { exact: true }).selectOption('reading:driveLoad');
        await chooseSize('medium');
        await page.getByRole('button', { name: 'Save', exact: true }).click();
        for (const presentation of ['bar', 'gauge', 'trend']) {
          await custom.getByRole('button', { name: /^Configure Load,/ }).click();
          await choosePresentation(presentation);
          await page.getByRole('button', { name: 'Save', exact: true }).click();
          for (const source of ['duty', 'motor', 'battery'] as const) {
            loadSource = source;
            const token = { duty: 'text-duty', motor: 'text-motor', battery: 'text-battery-current' }[source];
            await page.waitForFunction(
              (name) =>
                document
                  .querySelector('[data-dashboard-widget="medium-current"] .custom-widget-content')
                  ?.classList.contains(name),
              token
            );
            const colors = await custom
              .locator('[data-dashboard-widget="medium-current"] .custom-widget-content')
              .evaluate((el) => {
                const root = getComputedStyle(el).color;
                const visual = el.querySelector('.bg-current') ?? el.querySelector('svg path:last-child');
                const visualStyle = visual ? getComputedStyle(visual) : null;
                return {
                  root,
                  number: getComputedStyle(el.querySelector('.font-mono')!).color,
                  visual: visual?.matches('.bg-current') ? visualStyle!.backgroundColor : visualStyle?.stroke,
                };
              });
            assert.equal(colors.number, colors.root, `${presentation}/${source} text`);
            assert.equal(colors.visual, colors.root, `${presentation}/${source} visual`);
            if (presentation === 'gauge' && source === 'duty')
              await custom
                .locator('[data-dashboard-widget="medium-current"]')
                .screenshot({ path: new URL('gauge-widget.png', output).pathname });
          }
        }
        await custom.getByRole('button', { name: /^Configure Load,/ }).click();
        await page.getByLabel('Content', { exact: true }).selectOption('reading:motorCurrentA');
        await choosePresentation('bar');
        await page.getByRole('button', { name: 'Save', exact: true }).click();
        const signedBar = custom.locator('[data-dashboard-widget="medium-current"] .reading-bar');
        await signedBar.locator('[data-bar-zero]').waitFor();
        for (const [value, left, width] of [
          [-50, 25, 25],
          [0, 50, 0],
          [50, 50, 25],
        ]) {
          signedCurrent = value;
          await page.waitForFunction(
            ({ left, width }) => {
              const fill = document.querySelector<HTMLElement>(
                '[data-dashboard-widget="medium-current"] .reading-bar .bg-current'
              );
              return fill?.style.marginLeft === `${left}%` && fill.style.width === `${width}%`;
            },
            { left, width }
          );
          assert.equal(
            await signedBar.locator('[data-bar-zero]').evaluate((el) => (el as HTMLElement).style.left),
            '50%'
          );
        }
        signedCurrent = undefined;
        await custom.getByRole('button', { name: 'Cancel', exact: true }).click();
        await page.getByRole('button', { name: 'Discard changes', exact: true }).click();
        await viewing();

        // Search is value-first, selection is staged, and Cancel never adds a tile.
        await enterEditing();
        const beforeNew = await custom.locator('[data-dashboard-widget]').count();
        const writesBeforeNew = methods.filter((method) => method === 'boardProfile.update').length;
        await custom.getByRole('button', { name: 'Add widget', exact: true }).click();
        const search = page.getByRole('searchbox', { name: 'Search widgets', exact: true });
        await search.fill('cell voltage spread');
        const spreadChoice = page.getByRole('button', { name: 'Cell voltage spread', exact: true });
        await spreadChoice.click({ button: 'right' });
        assert.equal(await search.count(), 1, 'secondary input never selects a result');
        await spreadChoice.focus();
        await page.keyboard.press('Enter');
        assert.equal(await custom.locator('[data-dashboard-widget]').count(), beforeNew);
        assert.equal(
          await page.getByRole('button', { name: 'Number', exact: true }).getAttribute('aria-pressed'),
          'true'
        );
        assert.equal(
          await page
            .getByText(/Columns × rows|Dashboard preview ·|Uses this dashboard’s cell size|Changes stay in your draft/)
            .count(),
          0,
          'configuration omits redundant helper copy'
        );
        await assertSize(1, 1);
        await choosePresentation('trend');
        await assertSize(1, 1);
        for (const [columns, rows] of [
          [3, 3],
          [3, 4],
          [4, 4],
        ] as const) {
          const preset = page.getByRole('button', { name: `${columns} columns by ${rows} rows`, exact: true });
          if (rows === 3) {
            await preset.focus();
            await page.keyboard.press('Enter');
          } else await preset.click();
          await assertSize(columns, rows);
        }
        await page.getByRole('button', { name: 'Cancel', exact: true }).click();
        assert.equal(await custom.locator('[data-dashboard-widget]').count(), beforeNew);
        assert.equal(methods.filter((method) => method === 'boardProfile.update').length, writesBeforeNew);
        await custom.getByRole('button', { name: 'Add widget', exact: true }).click();
        await search.fill('pack voltage');
        await page.getByRole('button', { name: 'Pack voltage', exact: true }).click();
        assert.equal(await page.getByRole('button', { name: '3 columns by 3 rows', exact: true }).count(), 0);
        await choosePresentation('gauge');
        await assertSize(3, 3);
        assert.equal(await page.getByRole('combobox', { name: 'Width', exact: true }).count(), 0);
        assert.equal(await page.getByRole('combobox', { name: 'Height', exact: true }).count(), 0);
        await page.getByRole('button', { name: '3 columns by 4 rows', exact: true }).click();
        await assertSize(3, 4);
        const largestGauge = page.getByRole('button', { name: '4 columns by 4 rows', exact: true });
        await largestGauge.focus();
        await page.keyboard.press('Enter');
        await assertSize(4, 4);
        await choosePresentation('gauge');
        await assertSize(4, 4);
        await page.getByRole('button', { name: '3 columns by 3 rows', exact: true }).click();
        await assertSize(3, 3);
        await page.getByRole('button', { name: 'Cancel', exact: true }).click();
        await custom.getByRole('button', { name: 'Add widget', exact: true }).click();
        await search.fill('speed and load');
        await page.getByRole('button', { name: 'Speed and load', exact: true }).click();
        await assertSize(3, 3);
        assert.equal(await largestGauge.count(), 1);
        await page.getByRole('button', { name: 'Cancel', exact: true }).click();
        assert.equal(await custom.locator('[data-dashboard-widget]').count(), beforeNew);
        await custom.getByRole('button', { name: 'Add widget', exact: true }).click();
        await page.getByRole('button', { name: 'Browse all values →', exact: true }).click();
        assert.equal(
          await page.getByRole('button', { name: 'Controller-reported battery level', exact: true }).count(),
          1
        );
        assert.equal(await page.getByRole('button', { name: 'BMS battery level', exact: true }).count(), 1);
        await search.fill('no such widget');
        await page.getByText('No matching widgets.', { exact: false }).waitFor();
        await search.fill('battery panel');
        const panelChoice = page.getByRole('button', { name: 'Battery panel', exact: true });
        const choiceBox = (await panelChoice.boundingBox())!;
        await page.touchscreen.tap(choiceBox.x + choiceBox.width - 25, choiceBox.y + choiceBox.height / 2);
        await assertSize(2, 1);
        await page.getByRole('button', { name: '4 columns by 1 row', exact: true }).click();
        await page.getByRole('button', { name: 'Add widget', exact: true }).click();
        assert.equal(await custom.locator('[data-dashboard-widget]').count(), beforeNew + 1);
        const wideTile = await custom.locator('[data-dashboard-widget]').last().boundingBox();
        assert.ok(wideTile && wideTile.width > wideTile.height * 3.9, 'battery strip uses four columns and one row');
        await custom.getByRole('button', { name: 'Add widget', exact: true }).click();
        await search.fill('pack voltage');
        await page.getByRole('button', { name: 'Pack voltage', exact: true }).click();
        await page.getByRole('button', { name: '1 column by 2 rows', exact: true }).click();
        await page.getByRole('button', { name: 'Add widget', exact: true }).click();
        const tallId = await custom.locator('[data-dashboard-widget]').last().getAttribute('data-dashboard-widget');
        await page.setViewportSize({ width: 280, height: 653 });
        const tallTile = (await custom.locator(`[data-dashboard-widget="${tallId}"]`).boundingBox())!;
        const rowHeight = await custom
          .locator('.dashboard-grid')
          .evaluate((el) => parseFloat(getComputedStyle(el).gridAutoRows));
        assert.ok(
          Math.abs(tallTile.height - (rowHeight * 2 + 6)) < 1,
          'explicit row intent stays two on a two-column dashboard'
        );
        await page.setViewportSize({ width: 390, height: 844 });
        await custom.getByRole('button', { name: 'Save dashboard', exact: true }).click();
        await viewing();
        const flexible = (await client.request('boardProfile.get', { savedDeviceId: id })).values.customDashboard!;
        assert.deepEqual(flexible.widgets.at(-2)!.size, { columns: 4, rows: 1 });
        assert.deepEqual(flexible.widgets.at(-1)!.size, { columns: 1, rows: 2 });
        assert.equal(flexible.widgets[1]!.size, 'compact', 'untouched named footprints survive saves');
        await page.reload();
        await custom.locator(`[data-dashboard-widget="${tallId}"]`).waitFor();

        const current = await client.request('boardProfile.get', { savedDeviceId: id });
        const maximum = await client.request('boardProfile.update', {
          savedDeviceId: id,
          expectedRevision: current.revision,
          patch: {
            customDashboard: {
              version: 1,
              widgets: Array.from({ length: 24 }, (_, i) => ({
                id: `chart-${i}`,
                kind: 'reading',
                field: 'motorCurrentA',
                presentation: 'trend',
                size: 'medium',
              })),
            },
          },
        });
        const requestBoundary = methods.length;
        await page.reload();
        await custom.locator('[data-dashboard-widget="chart-23"]').waitFor();
        assert.equal(await custom.locator('svg').count(), 24);
        assert.ok(
          methods
            .slice(requestBoundary)
            .every((method) =>
              ['state.get', 'uiState.get', 'uiState.set', 'boardProfile.get', 'diagnostics.report'].includes(method)
            ),
          'charts consume existing samples without new collection operations'
        );
        await enterEditing();
        await custom.getByRole('button', { name: 'Add widget', exact: true }).click();
        await page.getByText('Dashboard limit reached', { exact: false }).waitFor();
        assert.equal(await page.getByRole('button', { name: 'Battery panel', exact: true }).isDisabled(), true);
        await page.getByRole('button', { name: 'Cancel', exact: true }).last().click();
        await custom.getByRole('button', { name: 'Cancel', exact: true }).click();
        await client.request('boardProfile.update', {
          savedDeviceId: id,
          expectedRevision: maximum.revision,
          patch: { customDashboard: { version: 1, widgets: [] } },
        });
        await page.reload();
        await page.getByText('No widgets yet.', { exact: false }).waitFor();
        await custom.getByRole('button', { name: 'Add widget', exact: true }).click();
        await page.getByRole('button', { name: 'Battery panel', exact: true }).click();
        await page.getByRole('button', { name: 'Add widget', exact: true }).click();
        await custom.getByRole('button', { name: 'Save dashboard', exact: true }).click();
        await viewing();
        assert.equal(await custom.locator('[data-dashboard-widget]').count(), 1);
        const savedCount = methods.filter((m) => m === 'boardProfile.update').length;
        await enterEditing();
        await custom.getByRole('button', { name: /^Configure Battery,/ }).click();
        await chooseSize('large');
        await page.getByRole('button', { name: 'Save', exact: true }).click();
        guest = true;
        await custom.waitFor({ state: 'detached' });
        assert.equal(
          methods.filter((m) => m === 'boardProfile.update').length,
          savedCount,
          'changing board identity never writes the old draft'
        );
        await page.reload();
        await page.getByRole('button', { name: 'Board actions', exact: true }).click();
        assert.equal(await page.getByRole('button', { name: 'Custom View', exact: true }).count(), 0);
        await page
          .getByRole('dialog', { name: 'Board actions', exact: true })
          .getByRole('button', { name: 'Done', exact: true })
          .click();
        await page.getByRole('dialog', { name: 'Board actions', exact: true }).waitFor({ state: 'hidden' });
        guest = false;
        await page.reload();
        await page.getByRole('button', { name: 'Board actions', exact: true }).click();
        await page.getByRole('button', { name: 'Custom View', exact: true }).click();
        const timedCard = custom.locator('[data-dashboard-widget]').first();
        await timedCard.waitFor();
        await timedCard.scrollIntoViewIfNeeded();
        const timedBounds = (await timedCard.boundingBox())!;
        const holdPoint = { x: timedBounds.x + timedBounds.width / 2, y: timedBounds.y + timedBounds.height / 2 };
        await page.clock.install({ time: 0 });
        await page.clock.pauseAt(1000);
        await page.mouse.move(holdPoint.x, holdPoint.y);
        await page.mouse.down();
        assert.equal(await timedCard.getAttribute('data-pressing'), 'true', 'press is acknowledged immediately');
        await page.clock.runFor(599);
        assert.equal(await timedCard.getAttribute('data-dragging'), null, 'entering edit mode still waits 600ms');
        await page.clock.runFor(1);
        assert.equal(await timedCard.getAttribute('data-dragging'), 'true');
        const liftedCue = await page.locator('[data-dashboard-lifted]').evaluate((el) => ({
          outline: getComputedStyle(el).outlineWidth,
          shadow: getComputedStyle(el).boxShadow,
          scale: getComputedStyle(el).scale,
          badges: el.querySelectorAll('[data-dashboard-remove]').length,
        }));
        assert.ok(
          liftedCue.outline === '2px' &&
            liftedCue.shadow !== 'none' &&
            liftedCue.scale === '1' &&
            liftedCue.badges === 0,
          'lift remains clear without motion or duplicate delete buttons'
        );
        assert.equal(await timedCard.getAttribute('data-pressing'), null);
        await page.mouse.up();
        await page.mouse.down();
        await page.clock.runFor(299);
        assert.equal(await timedCard.getAttribute('data-dragging'), null, 'editing hold is not armed before 300ms');
        await page.clock.runFor(1);
        assert.equal(await timedCard.getAttribute('data-dragging'), 'true', 'editing hold arms at 300ms');
        await page.mouse.up();
        await page.mouse.down();
        await page.evaluate(() => window.dispatchEvent(new Event('blur')));
        await page.clock.runFor(600);
        assert.equal(await timedCard.getAttribute('data-pressing'), null);
        assert.equal(await timedCard.getAttribute('data-dragging'), null, 'cancelled hold cannot lift later');
        await page.mouse.up();
        if (name === 'chromium') {
          const timingTouch = await page.context().newCDPSession(page);
          await timingTouch.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [holdPoint] });
          await page.clock.runFor(299);
          assert.equal(await timedCard.getAttribute('data-dragging'), null);
          await page.clock.runFor(1);
          assert.equal(await timedCard.getAttribute('data-dragging'), 'true', 'touch editing hold also arms at 300ms');
          await timingTouch.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
          await timingTouch.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [holdPoint] });
          await page.clock.runFor(100);
          await timingTouch.send('Input.dispatchTouchEvent', {
            type: 'touchMove',
            touchPoints: [{ x: holdPoint.x, y: holdPoint.y + 40 }],
          });
          assert.equal(await timedCard.getAttribute('data-pressing'), null);
          await page.clock.runFor(600);
          assert.equal(
            await timedCard.getAttribute('data-dragging'),
            null,
            'early touch scrolling cancels the short hold'
          );
          await timingTouch.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
          await timingTouch.detach();
        }
        assert.deepEqual(errors, []);
        await page.close();
      } finally {
        client.close();
        await browser.close();
        await stopMockServer();
      }
    }
  );
}
