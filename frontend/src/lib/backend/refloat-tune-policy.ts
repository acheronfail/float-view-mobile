import type { RefloatConfigurationIdentity } from './contract.ts';

/** Audited releases whose configuration writes preserve the running state. */
export function supportsRidingTuneApply(packageVersion: string | null | undefined): boolean {
  return [
    '1.1.0',
    '1.1.1',
    '1.1.2',
    '1.2.0',
    '1.2.1',
    '1.2.2',
    '1.2.3',
    '1.3.0',
    '1.3.1-beta2',
    '1.3.1-beta3',
    '1.3.1',
  ].includes(packageVersion ?? '');
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
