import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(
    `${name}: app export, merge, clear and re-import work with touch and keyboard`,
    { timeout: 60_000 },
    async () => {
      const endpoint = getMockBootstrap(await startMockServer(11002, { safetyDisclaimerAccepted: true }));
      const browser = await engine.launch();
      try {
        const page = await browser.newPage({
          viewport: { width: 390, height: 844 },
          hasTouch: true,
          reducedMotion: 'reduce',
        });
        page.setDefaultTimeout(10_000);
        if (name === 'chromium')
          await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
        await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
        await page.goto(`${endpoint.origin}/#/settings/data`);
        const data = page.getByRole('region', { name: 'Settings', exact: true });
        const exportButton = data.getByRole('button', { name: 'Create backup', exact: true });
        await exportButton.waitFor();
        assert.deepEqual(
          (await data.getByRole('heading', { level: 3 }).allTextContents()).map((text) => text.trim()),
          ['Backups', 'CSV exports', 'Clear app data']
        );
        await exportButton.click({ button: 'right' });
        assert.equal(await page.getByRole('dialog').count(), 0);
        const pointer = { pointerId: 12, pointerType: 'touch', isPrimary: false, button: 0 };
        await exportButton.dispatchEvent('pointerdown', pointer);
        await exportButton.dispatchEvent('pointerup', pointer);
        assert.equal(await page.getByRole('dialog').count(), 0);
        const downloaded = page.waitForEvent('download');
        await exportButton.tap();
        const download = await downloaded;
        assert.equal(download.suggestedFilename(), 'float-view-app-transfer.zip');
        const bytes = await readFile((await download.path())!);
        await page.getByRole('dialog', { name: 'Creating backup' }).waitFor({ state: 'detached' });
        const picker = data.getByLabel('Backup file');
        await picker.setInputFiles({ name: 'app-export.zip', mimeType: 'application/zip', buffer: bytes });
        let dialog = page.getByRole('dialog', { name: 'Restore backup', exact: true });
        assert.equal(await dialog.getByRole('checkbox', { name: 'Also restore preferences' }).isChecked(), false);
        await dialog.getByRole('button', { name: 'Restore backup', exact: true }).focus();
        await page.keyboard.press('Enter');
        await data.getByText('Sessions: 0 added, 2 already saved.', { exact: true }).waitFor();
        await data.getByRole('button', { name: 'Clear app data…', exact: true }).tap();
        dialog = page.getByRole('dialog', { name: 'Clear app data', exact: true });
        await page.keyboard.press('Escape');
        await dialog.waitFor({ state: 'detached' });
        await data.getByRole('button', { name: 'Clear app data…', exact: true }).focus();
        await page.keyboard.press('Enter');
        await dialog.getByRole('button', { name: 'Delete app data', exact: true }).tap();
        await data.getByText('App data cleared.', { exact: true }).waitFor();
        await picker.setInputFiles({ name: 'app-export.zip', mimeType: 'application/zip', buffer: bytes });
        dialog = page.getByRole('dialog', { name: 'Restore backup', exact: true });
        await dialog.getByRole('checkbox', { name: 'Also restore preferences' }).check();
        await dialog.getByRole('button', { name: 'Restore backup', exact: true }).tap();
        await data.getByText('Sessions: 2 added, 0 already saved.', { exact: true }).waitFor();
        await data.getByText('Preferences restored.', { exact: true }).waitFor();
        await picker.setInputFiles({
          name: 'invalid.zip',
          mimeType: 'application/zip',
          buffer: Buffer.from('bad zip'),
        });
        await page
          .getByRole('dialog', { name: 'Restore backup' })
          .getByRole('button', { name: 'Restore backup', exact: true })
          .tap();
        await page.getByRole('alert').filter({ hasText: 'HTTP 400' }).waitFor();
      } finally {
        await browser.close();
        await stopMockServer();
      }
    }
  );
}
