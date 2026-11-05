import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit, type Page } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { dispatchEdgeBack } from './support/edge-back.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
async function settled(page: Page) {
  await page.waitForFunction(() => {
    const content = document.querySelector('[data-route-content]') as HTMLElement | null;
    return (
      content &&
      getComputedStyle(content).visibility === 'visible' &&
      !content.querySelector('[data-navigation-pending="true"]')
    );
  });
  await page.evaluate(() => new Promise(requestAnimationFrame));
}

for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(
    `${name}: deep session browsing restores context across navigation and catalog changes`,
    { timeout: 90_000 },
    async () => {
      const endpoint = getMockBootstrap(await startMockServer(11002, { safetyDisclaimerAccepted: true }));
      const browser = await engine.launch();
      try {
        const page = await browser.newPage({
          viewport: { width: 390, height: 844 },
          hasTouch: true,
          reducedMotion: 'reduce',
        });
        page.setDefaultTimeout(7000);
        await page.addInitScript(() => {
          const messages: boolean[] = [];
          const routeMotionStarts = { count: 0 };
          const animate = Element.prototype.animate;
          Element.prototype.animate = function (this: Element, ...args: Parameters<Element['animate']>) {
            if (this.hasAttribute('data-route-content')) routeMotionStarts.count++;
            return animate.apply(this, args);
          };
          Object.assign(window, {
            routeMotionStarts,
            edgeBackMessages: messages,
            webkit: {
              messageHandlers: { floatViewBackNavigation: { postMessage: (value: boolean) => messages.push(value) } },
            },
          });
        });
        if (name === 'chromium')
          await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
        const errors: string[] = [];
        page.on('pageerror', (error) => errors.push(error.message));
        await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
        let revision = 0;
        let changed = false;
        const removed = new Set([128]);
        let failLists = false;
        let publications = 0;
        let delayLists = false;
        let realSession = '';
        let realBoard = '';
        let publishChangedState: (() => void) | undefined;
        const cursors: (string | null)[] = [];
        await page.routeWebSocket('**/api/ws?*', (socket) => {
          const server = socket.connectToServer();
          const lists = new Map<number, number>();
          const details = new Map<number, string>();
          socket.onMessage((raw) => {
            const request = JSON.parse(String(raw));
            if (request.method === 'session.list' && request.params.limit === 50) {
              const cursor = request.params.cursor ?? null;
              cursors.push(cursor);
              if (failLists) {
                socket.send(
                  JSON.stringify({ id: request.id, error: { code: 'internal_error', message: 'List unavailable' } })
                );
                return;
              }
              if (cursor) assert.ok(cursor.startsWith(`r${revision}:`), 'never reuse an invalidated cursor');
              lists.set(request.id, cursor ? Number(cursor.split(':')[1]) : 0);
              request.params.cursor = null;
              request.params.savedDeviceId = realBoard;
            }
            if (request.method === 'session.get' && request.params.sessionId.startsWith('row-')) {
              details.set(request.id, request.params.sessionId);
              request.params.sessionId = realSession;
            }
            server.send(JSON.stringify(request));
          });
          server.onMessage((raw) => {
            const message = JSON.parse(String(raw));
            const state =
              message.event === 'state.changed' ? message.data : message.result?.bluetooth ? message.result : null;
            if (state) {
              revision = state.sessionCatalogRevision + (changed ? 1 : 0);
              state.sessionCatalogRevision = revision;
              // Enough saved boards to scroll the Sessions index as well.
              const board = state.savedDevices[0];
              realBoard = board.savedDeviceId;
              state.savedDevices = Array.from({ length: 20 }, (_, index) => ({
                ...board,
                savedDeviceId: index ? `board-${index}` : board.savedDeviceId,
                displayName: `Board ${index}`,
              }));
            }
            const detail = details.get(message.id);
            if (detail && message.result?.summary) {
              message.result.summary.sessionId = detail;
              details.delete(message.id);
            }
            const start = lists.get(message.id);
            if (start !== undefined) {
              lists.delete(message.id);
              const summary = message.result.items[0];
              realSession ||= summary.sessionId;
              const ids = Array.from({ length: 180 }, (_, index) => index).filter(
                (index) => !changed || !removed.has(index)
              );
              if (changed) ids.unshift(999);
              message.result = {
                ...message.result,
                catalogRevision: revision,
                items: ids.slice(start, start + 50).map((index) => ({ ...summary, sessionId: `row-${index}` })),
                nextCursor: start + 50 < ids.length ? `r${revision}:${start + 50}` : null,
              };
              if (delayLists) {
                setTimeout(() => socket.send(JSON.stringify(message)), 350);
                return;
              }
            }
            if (state)
              publishChangedState = () => {
                const fresh = structuredClone(state);
                fresh.sessionCatalogRevision = ++revision;
                fresh.revision += ++publications;
                socket.send(JSON.stringify({ event: 'state.changed', data: fresh }));
              };
            socket.send(JSON.stringify(message));
          });
        });
        await page.route('**/api/session-data', async (route) => {
          const request = route.request().postDataJSON();
          if (!request.sessionId.startsWith('row-')) return route.continue();
          const response = await route.fetch({ postData: JSON.stringify({ ...request, sessionId: realSession }) });
          const result = await response.json();
          if (result.sessionId) result.sessionId = request.sessionId;
          await route.fulfill({ response, json: result });
        });
        await page.goto(`${endpoint.origin}/#/sessions`);
        await page.waitForFunction(
          () => (window as Window & { edgeBackMessages?: boolean[] }).edgeBackMessages?.at(-1) === false
        );
        assert.equal(
          await page.evaluate(() =>
            window.dispatchEvent(new CustomEvent('floatview:back', { cancelable: true, detail: { source: 'edge' } }))
          ),
          true,
          'edge Back does nothing on primary destinations'
        );
        const boards = page.getByRole('region', { name: 'Saved boards', exact: true });
        const boardLink = boards.getByRole('link').nth(10);
        await boardLink.scrollIntoViewIfNeeded();
        const indexY = await page.evaluate(() => scrollY);
        const boardHref = await boardLink.getAttribute('href');
        await boardLink.tap();
        await settled(page);
        assert.equal(await page.evaluate(() => scrollY), 0, 'new board starts at top');
        await page.waitForFunction(
          () => (window as Window & { edgeBackMessages?: boolean[] }).edgeBackMessages?.at(-1) === true
        );
        const sessions = page.getByRole('region', { name: 'Sessions', exact: true });
        for (const row of [99, 128]) {
          await sessions.getByLabel('Load more sessions').scrollIntoViewIfNeeded();
          await sessions.locator(`a[href*="row-${row}?"]`).waitFor();
        }
        const target = sessions.locator('a[href*="row-128?"]');
        // Put an item well into the third page at a stable offset below the sticky header.
        await target.evaluate((node) => window.scrollBy(0, node.getBoundingClientRect().top - 230));
        const targetY = (await target.boundingBox())!.y;
        const deepY = await page.evaluate(() => scrollY);
        assert.ok(deepY > 5000);
        const headerBack = page.getByRole('link', { name: 'Board sessions', exact: true });
        const motionCount = () =>
          page.evaluate(() => (window as Window & { routeMotionStarts?: { count: number } }).routeMotionStarts!.count);
        for (const back of ['header', 'browser', 'native', 'edge', 'edge-fallback', 'keyboard']) {
          if (back.startsWith('edge')) await page.emulateMedia({ reducedMotion: 'no-preference' });
          const beforeOpenMotion = await motionCount();
          if (back === 'keyboard') {
            await page.keyboard.press('Tab');
            await target.focus();
            await page.keyboard.press('Enter');
          } else await target.tap();
          await page.waitForURL('**/#/sessions/row-128?*');
          await headerBack.waitFor();
          if (back.startsWith('edge'))
            assert.ok((await motionCount()) > beforeOpenMotion, 'ordinary route entry still animates');
          assert.equal(await page.evaluate(() => scrollY), 0, 'new session starts at top');
          const requestCount = cursors.length;
          if (back === 'browser') await page.goBack();
          else if (back === 'native')
            await page.evaluate(() => window.dispatchEvent(new Event('floatview:back', { cancelable: true })));
          else if (back.startsWith('edge')) {
            const beforeBackMotion = await motionCount();
            assert.deepEqual(
              await dispatchEdgeBack(page, back === 'edge'),
              { consumed: true, navigated: true },
              'interactive Back acknowledges restored presentation'
            );
            assert.equal(
              (await motionCount()) - beforeBackMotion,
              back === 'edge' ? 0 : 1,
              'native slide suppresses route fade; capture fallback keeps ordinary motion'
            );
            await page.emulateMedia({ reducedMotion: 'reduce' });
          } else if (back === 'keyboard') await headerBack.press('Enter');
          else await headerBack.tap();
          if (!back.startsWith('edge')) await settled(page);
          assert.ok(Math.abs((await target.boundingBox())!.y - targetY) < 3, `${back}: same row offset`);
          assert.equal(cursors.length, requestCount, 'retained pages need no refetch');
          if (back === 'keyboard') assert.equal(await target.evaluate((node) => node === document.activeElement), true);
        }
        // Parent return restores the scrolled board index; another board starts independently.
        await page.getByRole('link', { name: 'Sessions', exact: true }).tap();
        await settled(page);
        assert.ok(Math.abs((await page.evaluate(() => scrollY)) - indexY) < 3);
        await boards.getByRole('link').nth(11).tap();
        await settled(page);
        assert.equal(await page.evaluate(() => scrollY), 0);
        await page.getByRole('link', { name: 'Sessions', exact: true }).tap();
        await settled(page);
        await page
          .getByRole('navigation', { name: 'Primary' })
          .getByRole('link', { name: 'settings', exact: true })
          .tap();
        await settled(page);
        await page.evaluate(() => window.scrollTo(0, 400));
        const settingsY = await page.evaluate(() => scrollY);
        await page
          .getByRole('navigation', { name: 'Primary' })
          .getByRole('link', { name: 'sessions', exact: true })
          .tap();
        await settled(page);
        assert.ok(Math.abs((await page.evaluate(() => scrollY)) - indexY) < 3);
        await page
          .getByRole('navigation', { name: 'Primary' })
          .getByRole('link', { name: 'settings', exact: true })
          .tap();
        await settled(page);
        assert.ok(Math.abs((await page.evaluate(() => scrollY)) - settingsY) < 3);
        await page
          .getByRole('navigation', { name: 'Primary' })
          .getByRole('link', { name: 'sessions', exact: true })
          .tap();
        await settled(page);
        await page.locator(`a[href="${boardHref}"]`).tap();
        await settled(page);
        assert.ok(Math.abs((await page.evaluate(() => scrollY)) - deepY) < 3);
        const firstVisible = await sessions.locator('li a').evaluateAll((links) => {
          const top = document.querySelector('header')!.getBoundingClientRect().bottom;
          return links.find((link) => link.getBoundingClientRect().bottom > top)!.getAttribute('href')!;
        });
        removed.add(Number(firstVisible.match(/row-(\d+)/)![1]));
        const survivingAnchor = await sessions.locator('li a').evaluateAll(
          (links, removedIds) => {
            const top = document.querySelector('header')!.getBoundingClientRect().bottom;
            const node = links
              .filter((link) => link.getBoundingClientRect().bottom > top)
              .find((link) => !removedIds.includes(Number(link.getAttribute('href')!.match(/row-(\d+)/)![1])))!;
            return { href: node.getAttribute('href')!, y: node.getBoundingClientRect().top };
          },
          [...removed]
        );
        await target.tap();
        await headerBack.waitFor();
        changed = true;
        delayLists = true;
        assert.ok(publishChangedState);
        publishChangedState();
        await headerBack.tap();
        await page.waitForFunction(() => document.querySelector('[data-navigation-pending="true"]'));
        assert.equal(
          await page.locator('[data-route-content]').evaluate((node) => getComputedStyle(node).visibility),
          'hidden'
        );
        await settled(page);
        assert.equal(await target.count(), 0, 'deleted row is not retained');
        const nearby = sessions.locator('a[href*="row-129?"]');
        assert.ok(
          Math.abs((await page.locator(`a[href="${survivingAnchor.href}"]`).boundingBox())!.y - survivingAnchor.y) < 3,
          'surviving anchor keeps its offset'
        );
        assert.ok(
          (await nearby.boundingBox())!.y > 150 && (await nearby.boundingBox())!.y < 500,
          'restores near removed anchor'
        );
        assert.ok(cursors.filter((cursor) => cursor?.startsWith(`r${revision}:`)).length >= 2);

        // Failed restoration reveals an actionable list without automatic retry loops.
        await nearby.tap();
        await headerBack.waitFor();
        failLists = true;
        publishChangedState();
        await headerBack.tap();
        await settled(page);
        await sessions.getByRole('alert').filter({ hasText: 'List unavailable' }).waitFor();
        const failedRequests = cursors.length;
        await page.evaluate(() => new Promise((resolve) => setTimeout(resolve, 100)));
        assert.equal(cursors.length, failedRequests);
        failLists = false;
        await sessions.getByRole('button', { name: 'Try again' }).tap();
        await sessions.getByRole('link').first().waitFor();

        // Leave while fresh pages are delayed; late completions must not move the parent.
        await sessions.getByRole('link').first().tap();
        await headerBack.waitFor();
        publishChangedState();
        const beforeDelayedReturn = cursors.length;
        await headerBack.tap();
        await page.waitForFunction(() => location.hash.includes('/sessions/boards/'));
        while (cursors.length === beforeDelayedReturn) await page.evaluate(() => new Promise(requestAnimationFrame));
        await page.getByRole('link', { name: 'Sessions', exact: true }).tap();
        await settled(page);
        const parentY = await page.evaluate(() => scrollY);
        await page.evaluate(() => new Promise((resolve) => setTimeout(resolve, 800)));
        assert.ok(
          Math.abs((await page.evaluate(() => scrollY)) - parentY) < 3,
          'late reads cannot move a newer destination'
        );
        await page.locator(`a[href="${boardHref}"]`).tap();
        await settled(page);
        assert.ok((await sessions.getByRole('link').count()) >= 150, 'interrupted loading retains its intended extent');
        assert.deepEqual(errors, []);
      } finally {
        await browser.close();
        await stopMockServer();
      }
    }
  );
}
