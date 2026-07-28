import { readdir, readFile, rm } from 'node:fs/promises';

const html = await readFile(new URL('../build/index.html', import.meta.url), 'utf8');
if (/(?:src|href)=(?:"|')\/?(?:_app|app)\//.test(html)) {
	throw new Error('build/index.html references an external runtime asset');
}
await rm(new URL('../build/_app/', import.meta.url), { recursive: true, force: true });
const entries = await readdir(new URL('../build/', import.meta.url), { recursive: true });
if (entries.length !== 1 || entries[0] !== 'index.html') {
	throw new Error(`Expected only build/index.html; found: ${entries.join(', ')}`);
}
