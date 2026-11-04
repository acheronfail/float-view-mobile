import assert from 'node:assert/strict';
import { readFile, writeFile, mkdir, mkdtemp, rename, stat, copyFile } from 'node:fs/promises';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { parseArgs } from 'node:util';
import { execFileSync } from 'node:child_process';
import { webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from './client.ts';
import { reduceSeries } from '../../frontend/src/lib/backend/session-reference.ts';
import { closedSideLayout } from '../test/support/closed-side-layout.ts';
import { MockBattery } from '../../frontend/backend-mock/battery.ts';
// Run through `just store-screenshots` so the frontend build is current.
const root = fileURLToPath(new URL('../../', import.meta.url));
const { values: options } = parseArgs({
  options: {
    ride: { type: 'string', default: '.local/store-screenshots-input/ride-sanitized.json' },
    output: { type: 'string' },
    help: { type: 'boolean', short: 'h' },
  },
});
if (options.help) {
  console.log(
    'Usage: just store-screenshots [output-folder] [ride-json]\nDirect: node --strip-types conformance/src/store-screenshots.mjs [--output folder] [--ride sanitized-json]'
  );
  process.exit(0);
}
const runId = new Date()
  .toISOString()
  .replace(/[-:]/g, '')
  .replace(/\.\d+Z$/, 'Z');
const destination = path.resolve(root, options.output ?? `.local/store-screenshots-${runId}`);
const input = path.resolve(root, options.ride);
let ride;
try {
  ride = JSON.parse(await readFile(input, 'utf8'));
} catch (error) {
  throw new Error(
    `Cannot read sanitized screenshot ride at ${input}. Supply --ride or see conformance/README.md#store-screenshots.`,
    { cause: error }
  );
}
assert(
  ride.meta?.boardProfileSnapshot && ride.marker && ride.samples?.length && ride.fields?.length,
  'Expected sanitized ride JSON with meta, marker, samples and fields'
);
const html = await readFile(path.join(root, 'frontend/build/index.html'), 'utf8');
try {
  await stat(destination);
  throw new Error(`Output already exists: ${destination}. Choose a new folder.`);
} catch (error) {
  if (error.code !== 'ENOENT') throw error;
}
await mkdir(path.join(root, 'conformance/generated/store-screenshots'), { recursive: true });
const work = await mkdtemp(path.join(root, 'conformance/generated/store-screenshots/run-'));
const folder = pathToFileURL(work + path.sep),
  output = new URL('images/', folder);
await mkdir(output, { recursive: true });
const uploads = path.join(work, 'upload');
const revision = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim();
const devices = [
  { name: 'iphone', group: 'ios/iphone', width: 393, height: 852, header: 120, scale: 3 },
  { name: 'ipad', group: 'ios/ipad', width: 1024, height: 1366, header: 190, scale: 2 },
  { name: 'mac', group: 'macos/mac', width: 1440, height: 900, header: 116, scale: 2 },
  { name: 'duo-outer', group: 'ios/iphone-duo-outer', width: 466, height: 678, header: 100, scale: 3 },
];
// The mock's browser origin is fixed; serialize with other captures using 11002.
const endpoint = getMockBootstrap(await startMockServer(11002, { safetyDisclaimerAccepted: true }));
let client, browser;
try {
  const manifest = [];
  const m = ride.meta,
    c = ride.marker;
  const summary = {
    sessionId: 'session-1',
    deviceHistoryId: 'history-board',
    displayName: 'Example board',
    startedAt: new Date(m.startedAtMs).toISOString(),
    endedAt: new Date(m.endedAtMs).toISOString(),
    state: 'completed',
    integrity: 'complete',
    sampleCount: m.sampleCount,
    eventCount: 0,
    totalElapsedMs: m.totalElapsedMs,
    ridingElapsedMs: m.ridingElapsedMs,
    distanceM: m.distanceM,
    maxSpeedMps: m.maxSpeedMps,
    maxDutyCycle: m.maxDutyCycle,
  };
  const history = {
    sessionCount: 1,
    totalRidingElapsedMs: m.ridingElapsedMs,
    totalDistanceM: m.distanceM,
    mostRecentSessionAt: summary.startedAt,
    totalStorageBytes: 4_800_000,
  };
  const snapshot = {
    historyRevision: 0,
    committedSampleSequence: c.committed_sample_sequence,
    committedEventSequence: 0,
    committedElapsedMs: c.committed_elapsed_ms,
    committedLocationSequence: 0,
    committedLocationElapsedMs: 0,
  };
  client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
  browser = await webkit.launch();
  await client.request('bluetooth.requestPermission', {});
  await client.request('bluetooth.scan', { timeoutMs: 100 });
  const scanned = await client.waitForState((s) => s.discoveredDevices.length > 0);
  await client.request('device.connect', { deviceId: scanned.discoveredDevices[0].deviceId, trackingMode: 'saved' });
  const ready = await client.waitForState((s) => s.activeConnection?.state === 'ready');
  const battery = new MockBattery();
  const estimates = ride.samples.map((row) =>
    battery.update(
      {
        ...ready.activeConnection,
        currentLimits: {
          ...ready.activeConnection.currentLimits,
          batterySeriesCells: m.boardProfileSnapshot.seriesCount,
        },
        latestSample: {
          ...row.values,
          ...row.states,
          sequence: row.sequence,
          elapsedMs: row.elapsedMs,
          observedAt: new Date(row.values.observedAtMs).toISOString(),
          slowObservedAt: new Date(row.values.observedAtMs).toISOString(),
          extensions: {},
        },
      },
      { revision: 3, values: m.boardProfileSnapshot }
    )
  );
  const initial = ride.samples.findIndex(
    (row) =>
      row.elapsedMs > 450000 &&
      row.rideState === 'riding' &&
      row.values.speedKph > 25 &&
      row.values.bmsCircuitryTempC != null
  );
  assert(initial > 0);
  await client.request('refloat.configuration.open', {});
  const tune = await client.request('refloat.configuration.read', { source: 'current' });
  const values = Object.fromEntries(Object.entries(tune.values).filter(([key]) => key.startsWith('leds.')));
  for (const [name, description] of [
    ['Everyday', 'Smooth and relaxed for daily rides.'],
    ['Trail', 'Responsive and playful on the trail.'],
  ])
    await client.request('refloat.tune.create', {
      name,
      description,
      identity: tune.identity,
      sourceRevision: tune.revision,
      values,
    });
  const views = [
    { name: 'live', route: 'live', title: 'Your board, at a glance' },
    { name: 'bms', route: 'live', title: 'Know your battery' },
    { name: 'charts', route: 'sessions/session-1', title: 'See the detail in every ride' },
    { name: 'configuration', route: 'live', title: 'Set up your board' },
    { name: 'tunes', route: 'live', title: 'Save/restore Refloat tunes' },
    { name: 'settings', route: 'settings', title: 'Heaps of customisation' },
  ];
  for (const device of devices) {
    console.log(`Capturing ${device.group}…`);
    const duo = device.name === 'duo-outer';
    const renderedHtml = duo
      ? html.replace(
          /env\(safe-area-inset-(top|right|bottom|left)(?:,\s*[^)]*)?\)/g,
          (_, edge) => closedSideLayout.safeArea[edge] + 'px'
        )
      : html;
    for (const theme of ['dark', 'light']) {
      await client.request('settings.update', {
        expectedRevision: client.currentState().settings.revision,
        patch: { themePreference: theme, locationRecordingEnabled: false },
      });
      const context = await browser.newContext({
        viewport: { width: device.width, height: duo ? device.height : device.height - device.header },
        deviceScaleFactor: device.scale,
        hasTouch: device.name !== 'mac',
        colorScheme: theme,
        reducedMotion: 'reduce',
      });
      const page = await context.newPage();
      page.setDefaultTimeout(8000);
      if (duo)
        await page.addInitScript((layout) => {
          window.floatViewHostLayout = layout;
        }, closedSideLayout);
      const errors = [];
      page.on('pageerror', (e) => errors.push(e.message));
      await page.route(endpoint.origin + '/', (route) =>
        route.fulfill({ contentType: 'text/html', body: renderedHtml })
      );
      if (device.name === 'mac')
        await page.route(endpoint.origin + '/api/bootstrap', async (route) => {
          const response = await route.fetch();
          await route.fulfill({ json: { ...(await response.json()), platform: 'macos' } });
        });
      await page.route('https://tile.openstreetmap.org/**', (route) => route.abort());
      await page.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        server.onMessage((raw) => {
          const msg = JSON.parse(String(raw));
          if (msg.result?.summary?.sessionId === 'session-1') {
            msg.result = {
              ...msg.result,
              summary,
              fields: ride.fields.map((field) => ({ name: field.field_name, source: field.source })),
              eventCount: 0,
              recordedSampleCount: m.sampleCount,
              storedSampleCount: m.sampleCount,
              prune: {
                targetResolutionSeconds: null,
                recordedSampleCount: m.sampleCount,
                storedSampleCount: m.sampleCount,
                policy: null,
              },
              snapshot,
              location: { available: false, count: 0, lastSequence: 0, lastElapsedMs: 0, status: 'disabled' },
            };
          }
          if (msg.result?.items?.some((item) => item.tuneId))
            msg.result.items.sort((a, b) => a.name.localeCompare(b.name));
          if (msg.result?.items && msg.result?.catalogRevision !== undefined) msg.result.items = [summary];
          if (msg.result?.savedDeviceId && msg.result?.values)
            Object.assign(msg.result, {
              values: { ...m.boardProfileSnapshot, motor: 'superflux_mk2_hs' },
              missingFields: [],
            });
          const state = msg.event === 'state.changed' ? msg.data : msg.result?.activeConnection ? msg.result : null;
          if (state) {
            state.savedDevices = state.savedDevices.map((board) => ({
              ...board,
              displayName: 'Example board',
              customDisplayName: 'Example board',
              advertisedName: 'Example board',
              profileComplete: true,
              summary: history,
            }));
          }
          if (state?.activeConnection?.latestSample) {
            const row = ride.samples[initial],
              v = row.values,
              a = state.activeConnection,
              now = new Date().toISOString();
            Object.assign(a.latestSample, v, row.states, {
              fatalErrorLatched: false,
              lastFirmwareFault: null,
              faultHistory: [],
              imuTimeoutActive: false,
              imuTimeoutHistory: [],
              sequence: row.sequence,
              elapsedMs: row.elapsedMs,
              ridingElapsedMs: row.elapsedMs,
              observedAt: now,
              slowObservedAt: now,
              extensions: { motorLoad: v.motorLoad, batteryLoad: v.batteryLoad },
            });
            a.bms = {
              status: 'attached',
              connection: { kind: 'can', canId: 10 },
              candidates: [],
              latestSnapshot: {
                observedAt: now,
                status: 'DISCHARGING',
                dataVersion: 1,
                totalVoltageV: 76.7,
                chargeVoltageV: 84,
                inputCurrentA: 27,
                inputIcCurrentA: 27,
                sessionAmpHoursAh: 1.8,
                sessionWattHoursWh: 139,
                cellVoltagesV: Array.from({ length: 20 }, (_, i) => 3.835 + [0, 0.002, -0.003, 0.001, -0.001][i % 5]),
                balancing: Array(20).fill(false),
                temperaturesC: [29.2, 30.1, 29.8],
                icTemperatureC: 32.4,
                humiditySensorTemperatureC: 29.7,
                humidityPercent: 38,
                highestCellTemperatureC: 30.1,
                highestCircuitryTemperatureC: 32.4,
                stateOfCharge: 0.9,
                stateOfHealth: 0.99,
                reportedCanId: 10,
                lifetimeChargeAmpHoursAh: 124,
                lifetimeChargeWattHoursWh: 9550,
                lifetimeDischargeAmpHoursAh: 119,
                lifetimeDischargeWattHoursWh: 9160,
                pressurePa: 101325,
              },
            };
            a.displayName = 'Example board';
            a.liveWidgetSlots = m.boardProfileSnapshot.liveWidgetSlots;
            a.currentLimits.batterySeriesCells = m.boardProfileSnapshot.seriesCount;
            a.batteryEstimate = { ...estimates[initial], sampleSequence: row.sequence };
            if (a.activeSession)
              Object.assign(a.activeSession, {
                displayName: 'Example board',
                totalElapsedMs: row.elapsedMs,
                ridingElapsedMs: row.elapsedMs,
                distanceM: v.distanceM,
              });
          }
          socket.send(JSON.stringify(msg));
        });
      });
      await page.route('**/api/session-data', async (route) => {
        const req = route.request().postDataJSON();
        if (req.sessionId === 'session-1' && req.kind === 'series') {
          await route.fulfill({
            json: {
              kind: 'series',
              sessionId: req.sessionId,
              snapshot: req.snapshot,
              range: req.range,
              gapPolicy: req.gapPolicy,
              fields: req.fields.map((name) => ({
                name,
                source: ride.fields.find((field) => field.field_name === name)?.source ?? 'stable',
              })),
              columns: reduceSeries(ride.samples, req.fields, req.range, req.buckets, req.gapPolicy, {
                expectedPersistenceMs: 1000,
              }),
            },
          });
          return;
        }
        await route.fallback();
      });
      for (const view of views) {
        await page.goto(endpoint.origin + '/#/' + view.route);
        await page.waitForFunction((theme) => document.documentElement.dataset.theme === theme, theme);
        if (view.name === 'live') await page.getByRole('button', { name: 'Board actions', exact: true }).waitFor();
        if (view.name === 'bms') {
          await page.getByRole('button', { name: 'Board actions', exact: true }).click();
          await page.getByRole('button', { name: 'BMS', exact: true }).click();
          await page.getByRole('progressbar', { name: 'Cell 1 voltage', exact: true }).waitFor();
        }
        if (view.name === 'charts')
          await page.getByRole('img', { name: 'Speed over elapsed time', exact: true }).waitFor();
        if (view.name === 'settings') await page.getByRole('group', { name: 'Theme', exact: true }).waitFor();
        if (view.name === 'configuration') {
          await page.getByRole('button', { name: 'Board actions', exact: true }).click();
          await page.getByRole('button', { name: 'Configure board', exact: true }).click();
          await page.getByRole('dialog', { name: 'Board configuration', exact: true }).waitFor();
          await page
            .getByRole('button', { name: 'Basic', exact: true })
            .evaluate((element) => element.scrollIntoView({ block: 'start' }));
        }
        if (view.name === 'tunes') {
          await page.getByRole('button', { name: 'Board actions', exact: true }).click();
          await page.getByRole('button', { name: 'Tunes…', exact: true }).click();
          await page
            .getByRole('dialog', { name: 'Tunes', exact: true })
            .getByRole('button', { name: 'Done', exact: true })
            .waitFor();
          await page
            .locator('[aria-label="Saved tunes"]')
            .getByRole('button', { name: /^Everyday/ })
            .waitFor();
        }
        if (duo) await page.waitForFunction(() => document.documentElement.dataset.controlsEdge === 'right');
        if (duo && view.name === 'charts')
          assert(
            (await page.locator('.app-back-control').boundingBox()).x >= 386,
            'back control belongs in side region'
          );
        if (duo && view.name === 'settings')
          for (const tab of await page.getByRole('navigation', { name: 'Primary' }).getByRole('link').all())
            assert((await tab.boundingBox()).x >= 386, 'tabs belong in side region');
        await page.evaluate(() => document.fonts.ready);
        await page.waitForTimeout(300);
        assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), 'horizontal overflow');
        const raw = await page.screenshot({ animations: 'disabled' });
        const name = device.name + '-' + theme + '-' + view.name;
        await writeFile(new URL(name + '-raw.png', output), raw);
        const card = await context.newPage();
        await card.setViewportSize({ width: device.width, height: device.height });
        const large = device.name === 'ipad' || device.name === 'mac',
          light = theme === 'light';
        const label =
          view.name === 'tunes'
            ? 'EXAMPLE PRESETS'
            : view.name === 'configuration'
              ? 'EXAMPLE SETUP'
              : 'RECORDED EXAMPLE';
        await card.setContent(
          `<!doctype html><meta charset="utf-8"><style>*{box-sizing:border-box}html,body{margin:0;background:${light ? '#f6f5fb' : '#0b0b11'};color:${light ? '#171329' : '#fff'};font-family:Arial,sans-serif}header{height:${device.header}px;padding:${device.name === 'mac' ? 22 : large ? 28 : 14}px ${large ? 40 : 20}px;background:${light ? 'linear-gradient(120deg,#ede6ff,#faf9ff)' : 'linear-gradient(120deg,#241640,#101019)'}}.eyebrow{font-size:${large ? 16 : 10}px;color:${light ? '#5c398e' : '#c6afff'};font-weight:700;letter-spacing:2px;margin:0 0 ${large ? 12 : 6}px}h1{font-size:${device.name === 'mac' ? 38 : large ? 46 : 27}px;line-height:1.05;letter-spacing:-.8px;margin:0 0 ${large ? 12 : 6}px}img{display:block;width:${device.width}px;height:${device.height - device.header}px;object-fit:contain}</style><header><div class="eyebrow">FLOAT VIEW</div><h1>${view.title}</h1></header><img alt="Actual shared UI preview" src="data:image/png;base64,${raw.toString('base64')}">`
        );
        await card.locator('img').evaluate((image) => image.decode());
        await card.screenshot({ type: 'jpeg', quality: 100 });
        await card.waitForTimeout(100);
        await card.screenshot({
          path: new URL(name + '.jpg', output).pathname,
          type: 'jpeg',
          quality: 100,
          animations: 'disabled',
        });
        manifest.push({
          file: name + '.jpg',
          rawFile: name + '-raw.png',
          device: device.name,
          theme,
          view: view.name,
          width: device.width * device.scale,
          height: device.height * device.scale,
          source: revision,
          data:
            view.name === 'bms'
              ? 'Illustrative BMS readings'
              : label === 'EXAMPLE PRESETS'
                ? 'Illustrative presets'
                : label === 'EXAMPLE SETUP'
                  ? 'Illustrative motor selection and recorded battery profile'
                  : 'Recorded ride telemetry; identities/date anonymized',
          renderer: duo
            ? 'WebKit with measured native CI side-control geometry; uniformly fitted under heading; OS chrome omitted'
            : 'WebKit shared UI; OS chrome omitted',
        });
        await card.close();
        if (view.name === 'bms') await page.getByRole('button', { name: 'Done', exact: true }).click();
        if (view.name === 'configuration')
          await page
            .getByRole('dialog', { name: 'Board configuration', exact: true })
            .getByRole('button', { name: 'Done', exact: true })
            .click();
        if (view.name === 'tunes')
          await page
            .getByRole('dialog', { name: 'Tunes', exact: true })
            .getByRole('button', { name: 'Done', exact: true })
            .click();
      }
      assert.deepEqual(errors, []);
      await context.close();
    }
  }
  await writeFile(new URL('manifest.json', folder), JSON.stringify(manifest, null, 2) + '\n');
  await exportScreenshots(browser, devices, views, manifest, output, uploads);
  await writeFile(
    new URL('source.json', folder),
    JSON.stringify(
      {
        source: revision,
        rideSha256: createHash('sha256')
          .update(await readFile(input))
          .digest('hex'),
        data: 'Anonymized ride telemetry; illustrative BMS, setup and tunes; no GPS',
        duoGeometry: 'conformance/test/support/closed-side-layout.ts',
      },
      null,
      2
    ) + '\n'
  );
  await mkdir(path.dirname(destination), { recursive: true });
  await rename(uploads, destination);
  console.log(`Saved 36 upload JPEGs to ${destination}\nReview captures and manifest: ${work}`);
} finally {
  await browser?.close();
  client?.close();
  await stopMockServer();
}

// Decode and warm up WebKit's compositor before saving (decode alone can precede paint).
async function paintedJpeg(page) {
  await page.locator('img').evaluateAll((images) => Promise.all(images.map((image) => image.decode())));
  await page.evaluate(() => document.fonts.ready);
  await page.screenshot({ type: 'jpeg' });
  await page.waitForTimeout(150);
  return page.screenshot({ type: 'jpeg', quality: 100, animations: 'disabled' });
}

async function validateJpeg(page, bytes, width, height) {
  const dimensions = await page.evaluate(async (base64) => {
    const img = new Image();
    img.src = `data:image/jpeg;base64,${base64}`;
    await img.decode();
    const canvas = document.createElement('canvas');
    canvas.width = img.width;
    canvas.height = img.height;
    const ctx = canvas.getContext('2d');
    ctx.drawImage(img, 0, 0);
    const colours = new Set();
    for (let y = Math.floor(img.height * 0.25); y < img.height; y += 19)
      for (let x = 0; x < img.width; x += 19) {
        const pixel = ctx.getImageData(x, y, 1, 1).data;
        colours.add(
          Array.from(pixel.slice(0, 3))
            .map((v) => Math.round(v / 16))
            .join(',')
        );
      }
    return { width: img.width, height: img.height, colours: colours.size };
  }, bytes.toString('base64'));
  assert.equal(dimensions.width, width);
  assert.equal(dimensions.height, height);
  assert(dimensions.colours > 20, 'Export appears blank below its marketing heading');
}

async function exportScreenshots(browser, devices, views, captures, images, uploads) {
  const manifest = [];
  const order = ['live', 'charts', 'tunes', 'configuration', 'bms', 'settings'];
  for (const device of devices) {
    await mkdir(path.join(uploads, device.group), { recursive: true });
    const page = await browser.newPage({
      viewport: { width: device.width * device.scale, height: device.height * device.scale },
      deviceScaleFactor: 1,
    });
    try {
      for (const [index, view] of order.entries()) {
        const [dark, light] = await Promise.all(
          ['dark', 'light'].map(async (theme) => {
            const row = captures.find((row) => row.device === device.name && row.theme === theme && row.view === view);
            return (await readFile(new URL(row.file, images))).toString('base64');
          })
        );
        const width = device.width * device.scale,
          height = device.height * device.scale;
        await page.setContent(
          `<!doctype html><style>html,body{margin:0;overflow:hidden;background:#fff}img,svg{position:absolute;inset:0;width:100%;height:100%}.dark{clip-path:polygon(0 0,100% 0,0 100%)}</style><img src="data:image/jpeg;base64,${light}"><img class="dark" src="data:image/jpeg;base64,${dark}"><svg viewBox="0 0 ${width} ${height}"><line x1="0" y1="${height}" x2="${width}" y2="0" stroke="#c6afff" stroke-width="${width / 300}"/></svg>`
        );
        const bytes = await paintedJpeg(page);
        await validateJpeg(page, bytes, width, height);
        const filename = `${String(index + 1).padStart(2, '0')}-${view}.jpg`;
        const relative = `${device.group}/${filename}`;
        await writeFile(path.join(uploads, relative), bytes);
        manifest.push({ file: relative, width, height, view, title: views.find((item) => item.name === view).title });
      }
    } finally {
      await page.close();
    }
  }
  // Preserve the approved Android composition: tablet matches iPad; phone adds
  // edge-extended background to retain the entire image without stretching it.
  for (const device of ['phone', 'tablet']) {
    const group = `android/${device}`;
    await mkdir(path.join(uploads, group), { recursive: true });
    const width = device === 'phone' ? 1440 : 2048,
      height = device === 'phone' ? 2560 : 2732;
    const sourceGroup = device === 'phone' ? 'ios/iphone' : 'ios/ipad';
    const page = await browser.newPage({ viewport: { width, height }, deviceScaleFactor: 1 });
    try {
      for (const item of [...manifest].filter((item) => item.file.startsWith(sourceGroup + '/'))) {
        const source = path.join(uploads, item.file),
          relative = `${group}/${path.basename(item.file)}`;
        if (device === 'tablet') await copyFile(source, path.join(uploads, relative));
        else {
          const base64 = (await readFile(source)).toString('base64');
          await page.setContent(
            `<style>html,body{margin:0;overflow:hidden}canvas{display:block}</style><canvas width="${width}" height="${height}"></canvas>`
          );
          await page.evaluate(
            async ({ base64, width, height }) => {
              const image = new Image();
              image.src = `data:image/jpeg;base64,${base64}`;
              await image.decode();
              const canvas = document.querySelector('canvas'),
                ctx = canvas.getContext('2d', { alpha: false });
              const scale = Math.min(width / image.width, height / image.height),
                w = Math.round(image.width * scale),
                h = Math.round(image.height * scale),
                x = Math.floor((width - w) / 2),
                y = Math.floor((height - h) / 2);
              if (x > 0) {
                ctx.drawImage(image, 0, 0, 1, image.height, 0, 0, x, height);
                ctx.drawImage(image, image.width - 1, 0, 1, image.height, x + w, 0, width - x - w, height);
              }
              if (y > 0) {
                ctx.drawImage(image, 0, 0, image.width, 1, x, 0, w, y);
                ctx.drawImage(image, 0, image.height - 1, image.width, 1, x, y + h, w, height - y - h);
              }
              ctx.drawImage(image, x, y, w, h);
            },
            { base64, width, height }
          );
          const bytes = await paintedJpeg(page);
          await validateJpeg(page, bytes, width, height);
          await writeFile(path.join(uploads, relative), bytes);
        }
        manifest.push({ ...item, file: relative, width, height });
      }
    } finally {
      await page.close();
    }
  }
  assert.equal(manifest.length, 36);
  await writeFile(new URL('upload-manifest.json', folder), JSON.stringify(manifest, null, 2) + '\n');
  const cards = manifest
    .map(
      (item) =>
        `<figure><a href="${path.relative(work, path.join(uploads, item.file))}"><img src="${path.relative(work, path.join(uploads, item.file))}" alt="${item.file}"></a><figcaption>${item.file} · ${item.width} × ${item.height}</figcaption></figure>`
    )
    .join('');
  // Gallery paths refer to the final output location, after the atomic rename.
  const gallery = cards.replaceAll('upload/', path.relative(work, destination).split(path.sep).join('/') + '/');
  await writeFile(
    new URL('index.html', folder),
    `<!doctype html><meta charset="utf-8"><title>Float View store screenshots</title><style>body{background:#15131c;color:#eee;font:16px system-ui;margin:24px}main{display:grid;grid-template-columns:repeat(6,1fr);gap:16px}figure{margin:0}img{width:100%}figcaption{padding:8px 0}a{color:#c6afff}</style><h1>Float View store screenshots</h1><p>WebKit shared UI with anonymized recorded telemetry and illustrative BMS, setup and tunes. Duo outer uses measured native CI geometry. OS chrome omitted; these are not native-device captures.</p><main>${gallery}</main>`
  );
}
