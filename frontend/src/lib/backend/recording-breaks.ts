import { MAX_SESSION_RECORDING_BREAKS } from './contract';

export function validRecordingBreaks(value: unknown, committedElapsedMs: number): boolean {
  if (!Array.isArray(value) || value.length > MAX_SESSION_RECORDING_BREAKS) return false;
  let previous = 0;
  for (const row of value) {
    if (!Array.isArray(row) || row.length !== 3) return false;
    const [start, end, reason] = row;
    if (
      !Number.isSafeInteger(start) ||
      !Number.isSafeInteger(end) ||
      start < previous ||
      end <= start ||
      end > committedElapsedMs ||
      !['pause', 'connectionLoss', 'unknown'].includes(reason)
    )
      return false;
    previous = end;
  }
  return true;
}
