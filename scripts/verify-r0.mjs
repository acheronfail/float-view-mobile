import { createHash } from 'node:crypto';
import { readFile, readdir } from 'node:fs/promises';
import { resolve } from 'node:path';

const root = resolve(import.meta.dirname, '..');

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function crc16(bytes) {
  let crc = 0;
  for (const byte of bytes) {
    crc ^= byte << 8;
    for (let bit = 0; bit < 8; bit += 1) {
      crc = ((crc & 0x8000) !== 0 ? (crc << 1) ^ 0x1021 : crc << 1) & 0xffff;
    }
  }
  return crc;
}

function parseFrame(hex) {
  assert(/^[0-9a-f]+$/i.test(hex) && hex.length % 2 === 0, 'frameHex must be even hexadecimal');
  const bytes = Uint8Array.from(Buffer.from(hex, 'hex'));
  assert(bytes.length >= 5, 'frame is too short');
  let index = 0;
  const start = bytes[index++];
  assert(start === 2 || start === 3, `invalid frame start ${start}`);
  const length = start === 2 ? bytes[index++] : (bytes[index++] << 8) | bytes[index++];
  assert(bytes.length === index + length + 3, 'declared frame length does not match bytes');
  const payload = bytes.slice(index, index + length);
  index += length;
  const declaredCrc = (bytes[index++] << 8) | bytes[index++];
  assert(bytes[index] === 3, 'invalid frame terminator');
  return {
    payload,
    validCrc: crc16(payload) === declaredCrc,
    sha256: createHash('sha256').update(bytes).digest('hex')
  };
}

async function json(path) {
  return JSON.parse(await readFile(resolve(root, path), 'utf8'));
}

const upstreams = await json('references/upstreams.json');
assert(upstreams.repositories.length === 4, 'all four pinned references are required');
for (const repository of upstreams.repositories) {
  assert(/^[0-9a-f]{40}$/.test(repository.revision), `invalid revision for ${repository.repository}`);
}

const inventory = await json('references/legacy-inventory.json');
assert(inventory.entries.length > 0, 'legacy inventory is empty');
for (const entry of inventory.entries) {
  assert(['retain', 'replace', 'extract-fixture'].includes(entry.action), `invalid action for ${entry.path}`);
}

const fixtureDirectories = ['fixtures/vesc', 'fixtures/refloat'];
let checked = 0;
for (const directory of fixtureDirectories) {
  for (const name of await readdir(resolve(root, directory))) {
    if (!name.endsWith('.json') || name === 'gatt-profile.json') continue;
    const fixture = await json(`${directory}/${name}`);
    const cases = fixture.cases ?? [fixture];
    for (const item of cases) {
      const parsed = parseFrame(item.frameHex);
      assert(parsed.validCrc === item.validCrc, `${fixture.id}/${item.name ?? name} CRC expectation failed`);
      assert(item.chunkSizes.reduce((sum, size) => sum + size, 0) === item.frameHex.length / 2, `${fixture.id} chunks do not cover frame`);
      assert(parsed.payload[0] === (item.commandId ?? fixture.expected?.commandId), `${fixture.id} command mismatch`);
      checked += 1;
    }
  }
}

const serializedFixtures = await Promise.all(
  fixtureDirectories.map(async (directory) =>
    Promise.all((await readdir(resolve(root, directory))).map((name) => readFile(resolve(root, directory, name), 'utf8')))
  )
);
const fixtureText = serializedFixtures.flat().join('\n');
assert(!/[0-9A-F]{2}(:[0-9A-F]{2}){5}/i.test(fixtureText), 'fixture contains a MAC address');
assert(!/session(token)?["']?\s*[:=]\s*["'][^"']+/i.test(fixtureText), 'fixture contains a session token');

console.log(`R0 evidence verified: ${inventory.entries.length} inventory entries, ${checked} framed fixture cases.`);
