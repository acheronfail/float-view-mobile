import type { RefloatConfigurationIdentity } from './contract.ts';

/** Stable supported minors preserve running state from Refloat 1.1 onward. */
export function supportsRidingTuneApply(packageVersion: string | null | undefined): boolean {
  return (
    /^(?:1\.[123]\.(?:0|[1-9]\d{0,2})|1\.3\.1-beta[23])$/.test(packageVersion ?? '') &&
    Number((packageVersion ?? '').split(/[.-]/)[2]) <= 255
  );
}

/** Preserve the existing public/stored identity normalization for audited betas. */
export function tunePackageVersion(version: string): string {
  return version === '1.3.1-beta2' || version === '1.3.1-beta3' ? '1.3.1' : version;
}

/** Compatibility key only; saved and observed patch identities remain unchanged. */
export function tuneCompatibilityVersion(version: string): string {
  const canonical = tunePackageVersion(version);
  const match = /^(1\.[0-3])\.(0|[1-9]\d{0,2})$/.exec(canonical);
  return match && Number(match[2]) <= 255 ? `${match[1]}.x` : canonical;
}

// Audited full-schema equivalence within stable minors. Keep aligned with the
// configuration service catalogue; conformance checks these against source fixtures.
const tuneSchemas: Record<string, { signature: number; fingerprints: string[] }> = {
  '1.0.x': {
    signature: 4090797430,
    fingerprints: [
      '324e7f5ef1ea1b2038478af7e6f43c91d63ec06a6caf2907c5b7e6c812382ad9',
      '887e7b03c03304a2f8455e71b369b89830e8d6446a73aa598ab067ac81944526',
    ],
  },
  '1.1.x': {
    signature: 15843867,
    fingerprints: ['b5849e38371a65721bb48182048023d1c707012228554878ce263b1c49ee168f'],
  },
  '1.2.x': {
    signature: 2427955642,
    fingerprints: [
      '2561e00a5b6694de0ac272bae077e06050cfc00cf4ba371bb1d5d9616e137f7e',
      '9b8979a36759810a7fa9c19d01534ed52ec9ed8ac4a2d3ab3aefc594ba3b93cc',
      'd7ca6a2c06ce5701858eebfaaf224e97e08a212d1dab60b664d268a90763eb39',
      'fa40717969d1d86b429d7c78b59144b8387b4850be83a72fd3c7cf15ffecd01c',
    ],
  },
  '1.3.x': {
    signature: 421162011,
    fingerprints: [
      '62681208dc609eb19420c89df65105f018499aa9f5741f295f00d62ea5a1bceb',
      '6bc6bfa433f8205519bc931723e39252aa71c9f538a0ed7c200d688486c63e73',
    ],
  },
};

function tuneSchemaKey(identity: RefloatConfigurationIdentity): string {
  const schemas = tuneSchemas[tuneCompatibilityVersion(identity.packageVersion)];
  return schemas?.signature === identity.wireSignature && schemas.fingerprints.includes(identity.schemaFingerprint)
    ? schemas.fingerprints[0]!
    : identity.schemaFingerprint;
}

export function sameRefloatTuneIdentity(
  left: RefloatConfigurationIdentity,
  right: RefloatConfigurationIdentity
): boolean {
  return (
    left.packageName === right.packageName &&
    tuneCompatibilityVersion(left.packageVersion) === tuneCompatibilityVersion(right.packageVersion) &&
    tuneSchemaKey(left) === tuneSchemaKey(right) &&
    left.wireSignature === right.wireSignature
  );
}
