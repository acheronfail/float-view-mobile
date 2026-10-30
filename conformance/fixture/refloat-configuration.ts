import { readFileSync } from 'node:fs';
import { RefloatConfigurationValuesLayout } from './refloat-configuration-values.ts';

export type StableRefloatConfigurationVersion =
  '1.0.0' | '1.0.1' | '1.1.0' | '1.1.1' | '1.1.2' | '1.2.0' | '1.2.1' | '1.2.2' | '1.2.3' | '1.3.0' | '1.3.1-beta2';

export type RefloatConfigurationOperation = 'schema' | 'current' | 'default' | 'write';

export interface RefloatConfigurationCounters {
  schema: number;
  current: number;
  default: number;
  write: number;
  rejected: number;
}

interface ManifestRelease {
  version: StableRefloatConfigurationVersion;
  schemaFingerprintSha256: string;
  wireSignature: number;
  writeNormalization: Array<{ field: string; value: number }>;
  files: {
    schema: string;
    default: string;
    current: string;
  };
}

interface Manifest {
  releases: ManifestRelease[];
}

const root = new URL('../../backends/fixtures/refloat-configuration/', import.meta.url);
const manifest = JSON.parse(readFileSync(new URL('manifest.json', root), 'utf8')) as Manifest;
const MAX_SCHEMA_BYTES = 32_768;
const MAX_VALUE_BYTES = 1_024;
const MAX_CHUNK_BYTES = 400;

/**
 * Stateful, source-fixture-backed implementation of VESC custom configuration
 * commands 92–95. One instance belongs to one semantic controller identity.
 */
export class RefloatConfigurationModel {
  readonly version: StableRefloatConfigurationVersion;
  readonly fingerprint: string;
  readonly wireSignature: number;
  readonly counters: RefloatConfigurationCounters = { schema: 0, current: 0, default: 0, write: 0, rejected: 0 };

  private readonly schema: Uint8Array;
  private readonly defaults: Uint8Array;
  private readonly initialCurrent: Uint8Array;
  private readonly layout: RefloatConfigurationValuesLayout;
  private readonly normalization: ManifestRelease['writeNormalization'];
  rejectionReason: string | undefined;
  private current: Uint8Array;

  constructor(version: StableRefloatConfigurationVersion) {
    const release = manifest.releases.find((item) => item.version === version);
    if (!release) throw new Error(`Missing Refloat configuration release ${version}`);
    this.version = version;
    this.fingerprint = release.schemaFingerprintSha256;
    this.wireSignature = release.wireSignature;
    this.schema = boundedFile(release.files.schema, MAX_SCHEMA_BYTES);
    this.defaults = boundedFile(release.files.default, MAX_VALUE_BYTES);
    this.initialCurrent = boundedFile(release.files.current, MAX_VALUE_BYTES);
    this.layout = new RefloatConfigurationValuesLayout(this.schema, this.fingerprint);
    this.normalization = release.writeNormalization;
    this.current = this.initialCurrent.slice();
  }

  fork(): RefloatConfigurationModel {
    const copy = Object.assign(Object.create(RefloatConfigurationModel.prototype), this, {
      current: this.current.slice(),
      counters: { ...this.counters },
    }) as RefloatConfigurationModel;
    return copy;
  }

  reset() {
    this.current = this.initialCurrent.slice();
    this.rejectionReason = undefined;
    Object.assign(this.counters, { schema: 0, current: 0, default: 0, write: 0, rejected: 0 });
  }

  snapshot() {
    return {
      version: this.version,
      fingerprint: this.fingerprint,
      wireSignature: this.wireSignature,
      currentBytes: this.current.length,
      defaultBytes: this.defaults.length,
      schemaBytes: this.schema.length,
      counters: { ...this.counters },
    };
  }

  respond(payload: Uint8Array, safe: boolean | 'riding' | 'flywheel'): Uint8Array | undefined {
    this.rejectionReason = undefined;
    const command = payload[0];
    if ((command === 93 || command === 95) && safe !== true) {
      return this.reject(
        safe === 'flywheel'
          ? 'flywheel mode is active'
          : safe === 'riding'
            ? 'controller is not idle'
            : 'controller is unsafe or flywheel mode is active'
      );
    }
    if (command !== undefined && command >= 92 && command <= 95 && payload[1] !== 0) {
      return this.reject('custom configuration index must be 0');
    }
    if (command === 92) return this.schemaResponse(payload);
    if (command === 93) {
      if (payload.length !== 2) return this.reject('current read must contain exactly 2 bytes');
      this.counters.current += 1;
      return Uint8Array.from([93, 0, ...this.current]);
    }
    if (command === 94) {
      if (payload.length !== 2) return this.reject('default read must contain exactly 2 bytes');
      this.counters.default += 1;
      return Uint8Array.from([94, 0, ...this.defaults]);
    }
    if (command === 95) {
      const values = payload.subarray(2);
      const reason = this.layout.rejection(values, this.wireSignature);
      if (reason) return this.reject(reason);
      this.current = this.layout.normalize(values, this.normalization);
      this.counters.write += 1;
      return Uint8Array.of(95);
    }
    return undefined;
  }

  private schemaResponse(payload: Uint8Array): Uint8Array | undefined {
    if (payload.length !== 10) return this.reject('schema request must contain exactly 10 bytes');
    const length = i32(payload, 2);
    const offset = i32(payload, 6);
    if (length < 1 || length > MAX_CHUNK_BYTES) return this.reject('schema chunk length must be between 1 and 400');
    if (offset < 0 || offset >= this.schema.length) return this.reject('schema chunk offset is out of bounds');
    const chunk = this.schema.slice(offset, Math.min(this.schema.length, offset + length));
    this.counters.schema += 1;
    return Uint8Array.from([92, 0, ...encodeI32(this.schema.length), ...encodeI32(offset), ...chunk]);
  }

  private reject(reason: string): undefined {
    this.rejectionReason = reason;
    this.counters.rejected += 1;
    return undefined;
  }
}

function boundedFile(path: string, maximum: number) {
  const bytes = new Uint8Array(readFileSync(new URL(path, root)));
  if (bytes.length < 1 || bytes.length > maximum) throw new Error(`Invalid bounded Refloat fixture ${path}`);
  return bytes;
}

function i32(bytes: Uint8Array, offset: number) {
  const unsigned =
    ((bytes[offset]! << 24) | (bytes[offset + 1]! << 16) | (bytes[offset + 2]! << 8) | bytes[offset + 3]!) >>> 0;
  return unsigned <= 0x7fffffff ? unsigned : -1;
}

function encodeI32(value: number) {
  return [value >>> 24, (value >>> 16) & 0xff, (value >>> 8) & 0xff, value & 0xff];
}
