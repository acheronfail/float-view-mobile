import { readFile, readdir } from 'node:fs/promises';
import { resolve } from 'node:path';

const root = resolve(import.meta.dirname, '..');
const forbidden = [
  /['"]\/ws['"]/,
  /backend-client\.types/,
  /Generated_Backend/,
  /VescBlePacketLayer/,
  /LocalWebSocket/,
  /LocalServer\.shared/
];

async function files(directory) {
  const result = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = resolve(directory, entry.name);
    if (entry.isDirectory()) result.push(...(await files(path)));
    else result.push(path);
  }
  return result;
}

for (const directory of ['web/src', 'web/backend-mock', 'android', 'ios', 'apple']) {
  for (const path of await files(resolve(root, directory))) {
    if (!/\.(?:ts|svelte|kt|swift)$/.test(path)) continue;
    const source = await readFile(path, 'utf8');
    for (const pattern of forbidden) {
      if (pattern.test(source)) {
        throw new Error(`Legacy runtime reference ${String(pattern)} remains in ${path}`);
      }
    }
  }
}

const required = [
  'apple/Sources/FloatViewCore/ApplicationService.swift',
  'apple/Sources/FloatViewCore/Domain.swift',
  'android/core/src/main/kotlin/fail/acheron/floatview/core/ApplicationService.kt',
  'android/protocol/build.gradle.kts',
  'android/logging/build.gradle.kts',
  'android/bluetooth/build.gradle.kts',
  'web/src/lib/api/store.svelte.ts'
];

for (const path of required) {
  await readFile(resolve(root, path));
}

console.log(`R1 boundaries verified: ${required.length} required modules, no legacy runtime references.`);
