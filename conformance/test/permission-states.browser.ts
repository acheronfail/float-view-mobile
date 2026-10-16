import assert from 'node:assert/strict';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { permissionInventory } from '../../frontend/backend-mock/permissions.ts';
import type { PermissionRow } from '../../frontend/src/lib/backend/contract.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
const base = permissionInventory('android36');
const row = (permission: PermissionRow['permission'], patch: Partial<PermissionRow> = {}): PermissionRow => ({
  ...base.find((value) => value.permission === permission)!,
  ...patch,
});
const cases: { name: string; rows: PermissionRow[] }[] = [
  { name: 'First setup', rows: base },
  {
    name: 'Allowed',
    rows: base.map((value) => ({
      ...value,
      status: 'granted',
      guidance: 'none',
      action: 'openSettings',
      accuracy: value.permission === 'location' ? 'precise' : 'unknown',
      scope: value.permission === 'location' ? 'always' : 'unknown',
    })),
  },
  {
    name: 'Limited access',
    rows: [
      row('location', {
        status: 'limited',
        accuracy: 'approximate',
        scope: 'whileInUse',
        action: 'openSettings',
        guidance: 'alwaysAccess',
      }),
    ],
  },
  {
    name: 'Not allowed',
    rows: [
      row('bluetooth', {
        status: 'denied',
        scan: 'denied',
        connect: 'granted',
        action: 'openSettings',
        guidance: 'appSettings',
      }),
      row('location', { status: 'denied', action: 'openSettings', guidance: 'iosSettings' }),
    ],
  },
  {
    name: 'Restricted',
    rows: [
      row('bluetooth', {
        status: 'restricted',
        scan: 'restricted',
        connect: 'restricted',
        action: null,
        guidance: 'restricted',
      }),
    ],
  },
  {
    name: 'Unavailable',
    rows: [
      row('bluetooth', {
        status: 'unavailable',
        scan: 'unknown',
        connect: 'unknown',
        action: null,
        guidance: 'unsupportedHardware',
      }),
      row('location', { status: 'unavailable', action: null, guidance: 'checking' }),
    ],
  },
  {
    name: 'Waiting for the system',
    rows: [row('location', { pending: true }), row('notifications'), row('backgroundLocation')],
  },
  {
    name: 'Services off',
    rows: [
      row('bluetooth', { servicesEnabled: false, guidance: 'servicesDisabled' }),
      row('location', {
        status: 'granted',
        servicesEnabled: false,
        action: 'openSettings',
        guidance: 'servicesDisabled',
        scope: 'whileInUse',
        accuracy: 'precise',
      }),
    ],
  },
  { name: 'Allow another permission first', rows: [row('backgroundLocation'), row('liveUpdates')] },
  { name: 'Request explanation', rows: [row('location', { guidance: 'rationale', status: 'denied' })] },
  {
    name: 'Android location settings',
    rows: [
      row('backgroundLocation', {
        action: 'openSettings',
        guidance: 'backgroundSettings',
        optionLabel: 'Allow all the time',
      }),
      row('location', { optional: false, guidance: 'fineLocationForScan' }),
    ],
  },
  {
    name: 'Mac settings',
    rows: [
      row('bluetooth', {
        status: 'denied',
        scan: null,
        connect: null,
        guidance: 'macBluetooth',
        action: 'openSettings',
      }),
      row('location', { status: 'denied', guidance: 'macLocation', action: 'openSettings' }),
    ],
  },
  {
    name: 'Notification settings',
    rows: [
      row('notifications', { status: 'granted', action: 'openSettings', guidance: 'notificationSettings' }),
      row('liveUpdates', { status: 'granted', action: 'openSettings', guidance: 'liveUpdateSettings' }),
    ],
  },
  {
    name: 'Partial Bluetooth access',
    rows: [
      row('bluetooth', {
        status: 'limited',
        scan: 'granted',
        connect: 'denied',
        guidance: 'appSettings',
        action: 'openSettings',
      }),
    ],
  },
];

// Keep every rendered inventory valid: location is required and inventories have at least two rows.
for (const scenario of cases) {
  if (!scenario.rows.some((value) => value.permission === 'location')) scenario.rows.push(row('location'));
  if (scenario.rows.length < 2) scenario.rows.push(row('bluetooth'));
}

for (const [engineName, engine] of Object.entries({ chromium, webkit })) {
  test(
    `${engineName}: permission states remain readable across themes and screen sizes`,
    { timeout: 120_000 },
    async () => {
      const port = await startMockServer(11002, { safetyDisclaimerAccepted: true });
      const endpoint = getMockBootstrap(port);
      const browser = await engine.launch();
      const output = engineName === 'chromium' ? process.env.PERMISSION_SCREENSHOT_DIR : undefined;
      const captures: { name: string; file: string }[] = [];
      const cards: { name: string; file: string; theme: string }[] = [];
      try {
        if (output) await mkdir(output, { recursive: true });
        const page = await browser.newPage({
          viewport: { width: 390, height: 844 },
          hasTouch: true,
          reducedMotion: 'reduce',
        });
        page.setDefaultTimeout(5000);
        if (engineName === 'chromium')
          await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
        const errors: string[] = [];
        page.on('pageerror', (error) => errors.push(error.message));
        let rows = base;
        let theme: 'light' | 'dark' = 'light';
        let closeBackend = () => {};
        let reject = false;
        await page.routeWebSocket('**/api/ws?*', (socket) => {
          const server = socket.connectToServer();
          closeBackend = () => server.close();
          socket.onMessage((raw) => {
            const message = JSON.parse(String(raw));
            if (reject && message.method === 'permissions.recover') {
              socket.send(
                JSON.stringify({
                  id: message.id,
                  error: { code: 'invalid_state', message: 'Couldn’t open settings. Try again.' },
                })
              );
            } else server.send(raw);
          });
          server.onMessage((raw) => {
            const message = JSON.parse(String(raw));
            const state = message.result?.permissions
              ? message.result
              : message.data?.permissions
                ? message.data
                : null;
            if (state) {
              state.permissions = rows;
              state.settings.themePreference = theme;
            }
            socket.send(JSON.stringify(message));
          });
        });
        await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
        const permissions = page.getByRole('region', { name: 'Permissions', exact: true });
        const capture = async (name: string, save = true) => {
          await permissions.scrollIntoViewIfNeeded();
          assert.equal(
            await page.evaluate(() => document.documentElement.scrollWidth > innerWidth),
            false,
            `${name}: no horizontal overflow`
          );
          if (output && save) {
            const file = `${name.toLowerCase().replace(/[^a-z0-9]+/g, '-')}.png`;
            const viewport = page.viewportSize()!;
            const height = Math.ceil((await permissions.boundingBox())!.height) + 300;
            await page.setViewportSize({ width: viewport.width, height });
            await permissions.evaluate((element) =>
              window.scrollTo(0, element.getBoundingClientRect().top + window.scrollY - 150)
            );
            await permissions.screenshot({ path: path.join(output, file), animations: 'disabled' });
            if (viewport.width === 390 && !name.startsWith('Action failed') && !name.startsWith('Disconnected')) {
              const primary =
                name.startsWith('First setup') || name.startsWith('Allowed') ? 'location' : rows[0]!.permission;
              const cardFile = file.replace('.png', '-card.png');
              await page
                .locator(`[data-permission="${primary}"]`)
                .screenshot({ path: path.join(output, cardFile), animations: 'disabled' });
              cards.push({ name: name.split(' — ')[0]!, file: cardFile, theme });
            }
            await page.setViewportSize(viewport);
            captures.push({ name, file });
          }
        };
        for (const width of [390, 1024, 320]) {
          await page.setViewportSize({ width, height: 844 });
          for (const mode of ['light', 'dark'] as const) {
            theme = mode;
            for (const scenario of cases) {
              rows = scenario.rows;
              await page.goto(`${endpoint.origin}/#/settings`);
              await page.locator(`[data-permission="${rows[0]!.permission}"]`).waitFor();
              await page.waitForFunction((mode) => document.documentElement.dataset.theme === mode, theme);
              assert.equal(await page.locator('[data-permission]').count(), rows.length);
              for (const value of rows) {
                const card = page.locator(`[data-permission="${value.permission}"]`);
                const status = await card.locator('[data-permission-status]').innerText();
                const statuses = {
                  notDetermined: 'Not requested',
                  granted: 'Allowed',
                  limited: 'Limited access',
                  denied: 'Not allowed',
                  restricted: 'Restricted',
                  unavailable: 'Unavailable',
                };
                assert.equal(status.trim(), value.pending ? 'Waiting…' : statuses[value.status]);
                assert.equal(await card.getByRole('button').count(), value.action ? 1 : 0);
                if (value.action) {
                  const button = card.getByRole('button');
                  assert.equal(
                    await button.isDisabled(),
                    rows.some((entry) => entry.pending)
                  );
                  assert.ok((await button.boundingBox())!.height >= 44, 'touch target is at least 44px');
                }
              }
              await capture(
                `${scenario.name} — ${mode} — ${width}px`,
                width === 390 || (width === 1024 && ['First setup', 'Allowed'].includes(scenario.name))
              );
            }
            rows = base;
            await page.reload();
            await page.locator('[data-permission="location"] button').waitFor();
            reject = true;
            await page.locator('[data-permission="location"] button').tap();
            await permissions.getByRole('alert').waitFor();
            if (width === 390) await capture(`Action failed — ${mode} — ${width}px`);
            reject = false;
            await page.reload();
            await page.locator('[data-permission="location"] button').waitFor();
            closeBackend();
            await permissions.getByRole('button', { name: 'Reconnect', exact: true }).waitFor();
            assert.equal(await page.locator('[data-permission="location"] button').isDisabled(), true);
            assert.equal(await permissions.getByRole('alert').count(), 0, 'reconnect guidance is shown once');
            if (width === 390) await capture(`Disconnected — ${mode} — ${width}px`);
          }
        }
        assert.deepEqual(errors, []);
        if (output) {
          const overviewNames = [
            'First setup',
            'Allowed',
            'Limited access',
            'Not allowed',
            'Restricted',
            'Unavailable',
            'Waiting for the system',
            'Services off',
          ];
          for (const mode of ['light', 'dark']) {
            const selected = cards.filter((item) => item.theme === mode && overviewNames.includes(item.name));
            const background = mode === 'light' ? '#f8fafc' : '#020617';
            const foreground = mode === 'light' ? '#0f172a' : '#f1f5f9';
            await writeFile(
              path.join(output, `overview-${mode}.html`),
              `<!doctype html><meta charset="utf-8"><title>Permission states — ${mode}</title><style>body{margin:24px;font:16px system-ui;background:${background};color:${foreground}}h1{font-size:24px}main{display:grid;grid-template-columns:repeat(4,1fr);gap:24px}figure{margin:0}figcaption{font-size:14px;font-weight:600;margin-bottom:12px}img{width:100%;height:auto;display:block}</style><h1>Permission cards — ${mode}</h1><main>${selected.map((item) => `<figure><figcaption>${item.name}</figcaption><img src="${item.file}" alt="${item.name}"></figure>`).join('')}</main>`
            );
            const overview = await browser.newPage({ viewport: { width: 1536, height: 1000 } });
            await overview.goto(new URL(`file://${path.resolve(output, `overview-${mode}.html`)}`).href);
            await overview.screenshot({ path: path.join(output, `overview-${mode}.png`), fullPage: true });
            await overview.close();
          }
          await writeFile(
            path.join(output, 'index.html'),
            `<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>Permission cards — iteration 1</title><style>body{font:16px system-ui;background:#e2e8f0;color:#0f172a;margin:24px}h1{font-size:24px}nav{display:flex;gap:12px;flex-wrap:wrap}nav a{color:#3730a3}main{display:grid;grid-template-columns:repeat(auto-fit,minmax(300px,1fr));gap:24px;margin-top:24px}figure{margin:0;align-self:start;background:white;border-radius:12px;padding:16px}img{width:100%;height:auto;display:block}figcaption{font-weight:600;margin-bottom:12px}</style><h1>Permission cards — iteration 1</h1><p>Actual frontend screenshots. All six statuses, pending actions, service warnings, OS guidance, errors and reconnect. Both themes; desktop and touch layouts.</p><p><a href="overview-light.png">Light state overview</a> · <a href="overview-dark.png">Dark state overview</a></p><nav>${captures.map((item, i) => `<a href="#state-${i}">${item.name}</a>`).join('')}</nav><main>${captures.map((item, i) => `<figure id="state-${i}"><figcaption>${item.name}</figcaption><a href="${item.file}"><img loading="lazy" src="${item.file}" alt="${item.name}"></a></figure>`).join('')}</main>`
          );
        }
      } finally {
        await browser.close();
        await stopMockServer();
      }
    }
  );
}
