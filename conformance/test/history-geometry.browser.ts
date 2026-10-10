import assert from 'node:assert/strict';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
// This frontend module is checked by Svelte/Vite's resolver, not NodeNext.
const { buildPathSegments } = await import(
  new URL('../../frontend/src/lib/features/sessions/history-geometry.ts', import.meta.url).href
);

for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: isolated history samples remain visible without bridging gaps`, async () => {
    const paths: string[] = buildPathSegments(
      { elapsedMs: [0, 1500, 3000], sequence: [1, 2, 3], values: [1, 1, 1], generations: [0, 1, 2] },
      { width: 100, height: 50, viewport: { startElapsedMs: 0, endElapsedMs: 3000 }, domain: { min: 0, max: 2 } }
    );
    const browser = await engine.launch();
    try {
      const page = await browser.newPage();
      await page.setContent(`<svg xmlns="http://www.w3.org/2000/svg" width="120" height="60" viewBox="-10 -5 120 60">
        ${paths.map((d) => `<path d="${d}" fill="none" stroke="black" stroke-width="1.5" stroke-linecap="round"/>`).join('')}
      </svg>`);
      const pixels = await page.evaluate(async () => {
        const svg = new XMLSerializer().serializeToString(document.querySelector('svg')!);
        const image = new Image();
        image.src = `data:image/svg+xml;base64,${btoa(svg)}`;
        await image.decode();
        const canvas = document.createElement('canvas');
        canvas.width = 120;
        canvas.height = 60;
        const context = canvas.getContext('2d')!;
        context.drawImage(image, 0, 0);
        return [10, 35, 60, 85, 110].map((x) => context.getImageData(x, 30, 1, 1).data[3]);
      });
      assert.ok(pixels[0]! > 0 && pixels[2]! > 0 && pixels[4]! > 0, 'every recorded sample is visible');
      assert.equal(pixels[1], 0, 'first gap stays empty');
      assert.equal(pixels[3], 0, 'second gap stays empty');
    } finally {
      await browser.close();
    }
  });
}
