import { writeFile } from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import type { Browser } from 'playwright';
import { liveViewports } from './live-viewports.ts';

/** An inspectable HTML report plus contact sheets made from the actual captures. */
export async function writeLiveViewportGallery(browser: Browser, output: string, engine: string) {
  const groups = {
    compact: 'Compact phones and browser heights',
    phones: 'Modern phones',
    large: 'Tablets and landscape',
    foldables: 'Square and unfolded shapes',
  };
  const report = `<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Float View · Live viewport matrix</title>
<style>
*{box-sizing:border-box}body{margin:0;padding:24px;background:#e8edf3;color:#0f172a;font:15px system-ui,sans-serif}header{max-width:1200px;margin:auto auto 24px}h1{font-size:26px;margin:0 0 8px}p{line-height:1.5}nav{display:flex;gap:16px;flex-wrap:wrap}label{display:grid;gap:4px;font-weight:600}select{padding:8px;border:1px solid #94a3b8;border-radius:8px;font:inherit}section{max-width:1200px;margin:0 auto 32px;padding:20px;background:#f8fafc;border-radius:16px}h2{margin:0 0 16px;font-size:20px}.cards{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:20px;align-items:start}figure{min-width:0;margin:0}figcaption{min-height:52px;font-weight:650}small{display:block;font-weight:400;color:#475569;margin-top:4px}a{color:inherit}img{width:100%;height:auto;display:block;border:1px solid #cbd5e1;border-radius:8px}body[data-theme=dark]{background:#020617;color:#e2e8f0}body[data-theme=dark] section{background:#0f172a}body[data-theme=dark] small{color:#94a3b8}@media(max-width:700px){.cards{grid-template-columns:repeat(2,minmax(0,1fr))}body{padding:12px}section{padding:12px}}
</style>
<body data-theme="light"><header><h1>Float View · Live viewport matrix</h1>
<p>${liveViewports.length} CSS viewports · Chromium and WebKit · light and dark · normal and detailed views.<br>Long board names, fixed live values, and long firmware identifiers. Screenshots emulate web content, not physical devices or OS safe-area insets.</p>
<nav><label>Browser<select id="engine"><option value="chromium">Chromium</option><option value="webkit">WebKit</option></select></label><label>Theme<select id="theme"><option>light</option><option>dark</option></select></label><label>Scenario<select id="scenario"><option value="saved">Saved board</option><option value="bms">BMS + unbroken name</option><option value="guest">Guest connection</option><option value="detailed">Detailed view · top</option><option value="detailed-end">Detailed view · bottom</option></select></label></nav>
<p>Tap any screenshot to inspect its original resolution. Exceptionally short guest/landscape views scroll vertically to keep readings and touch targets intact; the header stays fixed.</p></header>
${Object.entries(groups)
  .map(
    ([group, title]) =>
      `<section id="${group}"><h2>${title}</h2><div class="cards">${liveViewports
        .filter((v) => v.group === group)
        .map(
          (v) =>
            `<figure data-id="${v.id}"><figcaption>${v.label}<small>${v.viewport.width} × ${v.viewport.height} CSS px</small></figcaption><a class="capture" href="${engine}/${v.id}-light-saved.png"><img alt="${v.label} live dashboard" src="${engine}/${v.id}-light-saved.png" width="${v.viewport.width}" height="${v.viewport.height}"></a></figure>`
        )
        .join('')}</div></section>`
  )
  .join('')}
<script>
const engine=document.getElementById('engine'),theme=document.getElementById('theme'),scenario=document.getElementById('scenario');
engine.value=${JSON.stringify(engine)};
function refresh(){document.body.dataset.theme=theme.value;for(const f of document.querySelectorAll('figure')){const src=engine.value+'/'+f.dataset.id+'-'+theme.value+'-'+scenario.value+'.png';f.querySelector('img').src=src;f.querySelector('a.capture').href=src;}}
for(const s of [engine,theme,scenario])s.addEventListener('change',refresh);refresh();
</script></body></html>`;
  const reportPath = path.join(output, 'index.html');
  await writeFile(reportPath, report);
  const page = await browser.newPage({ viewport: { width: 1100, height: 900 }, deviceScaleFactor: 1 });
  try {
    await page.goto(pathToFileURL(reportPath).href);
    await page.evaluate(async () => Promise.all([...document.images].map((img) => img.decode())));
    for (const group of Object.keys(groups))
      await page.locator(`#${group}`).screenshot({ path: path.join(output, `${engine}-${group}.png`) });
    await page.selectOption('#scenario', 'detailed');
    await page.evaluate(async () => Promise.all([...document.images].map((img) => img.decode())));
    for (const group of ['phones', 'foldables', 'large'])
      await page.locator(`#${group}`).screenshot({ path: path.join(output, `${engine}-${group}-detailed.png`) });
    await page.selectOption('#theme', 'dark');
    await page.selectOption('#scenario', 'bms');
    await page.evaluate(async () => Promise.all([...document.images].map((img) => img.decode())));
    await page.locator('#compact').screenshot({ path: path.join(output, `${engine}-compact-dark-bms.png`) });
  } finally {
    await page.close();
  }
}
