import type { RefloatConfigurationIdentity } from './contract.ts';

/** Stable supported minors preserve running state from Refloat 1.1 onward. */
export function supportsRidingTuneApply(packageVersion: string | null | undefined): boolean {
  return (
    /^(?:1\.[123]\.(?:0|[1-9]\d{0,2})|1\.3\.1-beta[23])$/.test(packageVersion ?? '') &&
    Number((packageVersion ?? '').split(/[.-]/)[2]) <= 255
  );
}

/** Only these source-audited releases share Tune configuration semantics. */
export function tunePackageVersion(version: string): string {
  return version === '1.3.1-beta2' || version === '1.3.1-beta3' ? '1.3.1' : version;
}

export function sameRefloatTuneIdentity(
  left: RefloatConfigurationIdentity,
  right: RefloatConfigurationIdentity
): boolean {
  return (
    left.packageName === right.packageName &&
    tunePackageVersion(left.packageVersion) === tunePackageVersion(right.packageVersion) &&
    left.schemaFingerprint === right.schemaFingerprint &&
    left.wireSignature === right.wireSignature
  );
}
