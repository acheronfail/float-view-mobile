/** Audited releases whose configuration writes preserve the running state. */
export function supportsRidingTuneApply(packageVersion: string | null | undefined): boolean {
  return ['1.1.0', '1.1.1', '1.1.2', '1.2.0', '1.2.1', '1.2.2', '1.2.3', '1.3.0'].includes(packageVersion ?? '');
}
