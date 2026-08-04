import { type SessionDataRange, type SessionGapPolicy, type SessionSeriesColumn } from './session-data.ts';
import {
  MAX_SESSION_CURSOR_BYTES,
  MAX_SESSION_EVENT_ROWS,
  MAX_SESSION_SAMPLE_ROWS,
  MAX_SESSION_SERIES_BUCKETS,
  MAX_SESSION_SERIES_OBSERVATIONS,
  type SessionPublishedSnapshot,
} from './contract.ts';

export interface ReferenceSample {
  sequence: number;
  elapsedMs: number;
  rideState?: string;
  values: Record<string, number | null>;
}

export interface ReferenceEvent {
  sequence: number;
  sourceSampleSequence: number;
  elapsedMs: number;
  kind: string;
  value: string;
}

export interface ReferenceReducerOptions {
  expectedPersistenceMs?: number;
  retainedResolutionSeconds?: number;
}

export function reduceSeries(
  input: readonly ReferenceSample[],
  fields: readonly string[],
  range: SessionDataRange,
  buckets: number,
  gapPolicy: SessionGapPolicy,
  options: ReferenceReducerOptions = {}
): Record<string, SessionSeriesColumn> {
  if (
    buckets < 1 ||
    !Number.isSafeInteger(buckets) ||
    buckets > MAX_SESSION_SERIES_BUCKETS ||
    fields.length < 1 ||
    fields.length > 16 ||
    new Set(fields).size !== fields.length
  )
    throw new RangeError('Invalid series bounds');
  const rows = input
    .filter((row) => row.elapsedMs >= range.startElapsedMs && row.elapsedMs <= range.endElapsedMs)
    .slice()
    .sort((left, right) => left.elapsedMs - right.elapsedMs || left.sequence - right.sequence);
  const span = BigInt(range.endElapsedMs - range.startElapsedMs) + 1n;
  const thresholdMs =
    gapPolicy.thresholdSeconds === null
      ? Math.max(
          1_000,
          3 * (options.expectedPersistenceMs ?? 1_000),
          3 * ((options.retainedResolutionSeconds ?? 0) * 1_000)
        )
      : Math.round(gapPolicy.thresholdSeconds * 1_000);
  const generations = new Map<string, number>();
  const selected = new Map<string, Map<number, { row: ReferenceSample; value: number; generation: number }>>();
  const byBucket = new Map<
    string,
    Map<number, Map<number, { row: ReferenceSample; value: number; generation: number }>>
  >();
  for (const field of fields) {
    selected.set(field, new Map());
    byBucket.set(field, new Map());
  }
  let lastElapsed: number | undefined;
  for (const row of rows) {
    const hasTimeGap = lastElapsed !== undefined && row.elapsedMs - lastElapsed > thresholdMs;
    if (hasTimeGap) fields.forEach((field) => generations.set(field, (generations.get(field) ?? 0) + 1));
    lastElapsed = row.elapsedMs;
    for (const field of fields) {
      const value = row.values[field] ?? null;
      if (value === null) {
        generations.set(field, (generations.get(field) ?? 0) + 1);
        continue;
      }
      const generation = generations.get(field) ?? 0;
      const bucket = bucketIndex(row.elapsedMs, range, buckets, span);
      const bucketMap = byBucket.get(field)!;
      const currentBucket = bucketMap.get(bucket) ?? new Map();
      const selectedField = selected.get(field)!;
      for (const sequence of currentBucket.keys()) selectedField.delete(sequence);
      currentBucket.set(row.sequence, { row, value, generation });
      bucketMap.set(bucket, currentBucket);
      const current = [...currentBucket.values()];
      const first = current.reduce((a, b) => (compareKey(a.row, b.row) <= 0 ? a : b));
      const last = current.reduce((a, b) => (compareKey(a.row, b.row) >= 0 ? a : b));
      const minimum = current.reduce((a, b) => (b.value < a.value ? b : a));
      const maximum = current.reduce((a, b) => (b.value > a.value ? b : a));
      const roles = [first, minimum, maximum, last];
      const unique = new Map(roles.map((candidate) => [candidate.row.sequence, candidate]));
      currentBucket.clear();
      for (const candidate of unique.values()) currentBucket.set(candidate.row.sequence, candidate);
      for (const candidate of currentBucket.values()) selectedField.set(candidate.row.sequence, candidate);
    }
  }
  const result: Record<string, SessionSeriesColumn> = {};
  for (const field of fields) {
    const rowsBySequence = [...selected.get(field)!.values()].sort((left, right) => compareKey(left.row, right.row));
    result[field] = {
      elapsedMs: rowsBySequence.map((item) => item.row.elapsedMs),
      sequence: rowsBySequence.map((item) => item.row.sequence),
      values: rowsBySequence.map((item) => item.value),
      generations: rowsBySequence.map((item) => item.generation),
    };
  }
  if (Object.values(result).reduce((sum, column) => sum + column.values.length, 0) > MAX_SESSION_SERIES_OBSERVATIONS)
    throw new RangeError('Series observation bound exceeded');
  return result;
}

export function selectPruneRows(
  input: readonly ReferenceSample[],
  fields: readonly string[],
  targetResolutionSeconds: number,
  protectedSequences: ReadonlySet<number> = new Set(),
  eventSourceSequences: ReadonlySet<number> = new Set()
): number[] {
  if (!Number.isSafeInteger(targetResolutionSeconds) || targetResolutionSeconds < 1 || targetResolutionSeconds > 300)
    throw new RangeError('Invalid prune resolution');
  const rows = input.slice().sort(compareKey);
  if (!rows.length) return [];
  const keep = new Set<number>([
    rows[0]!.sequence,
    rows.at(-1)!.sequence,
    ...protectedSequences,
    ...eventSourceSequences,
  ]);
  for (const field of fields) {
    const first = rows.find((row) => row.values[field] !== null && row.values[field] !== undefined);
    if (first) keep.add(first.sequence);
  }
  const buckets = new Map<number, ReferenceSample[]>();
  const width = targetResolutionSeconds * 1_000;
  for (const row of rows) {
    const bucket = Math.floor(row.elapsedMs / width);
    const list = buckets.get(bucket) ?? [];
    list.push(row);
    buckets.set(bucket, list);
  }
  for (const bucketRows of buckets.values()) {
    const roles = new Map<string, ReferenceSample>();
    const first = bucketRows[0]!;
    const last = bucketRows.at(-1)!;
    roles.set(`first`, first);
    roles.set(`last`, last);
    for (const field of fields) {
      const values = bucketRows.filter((row) => row.values[field] !== null && row.values[field] !== undefined);
      if (!values.length) continue;
      roles.set(
        `${field}:min`,
        values.reduce((a, b) => (b.values[field]! < a.values[field]! ? b : a))
      );
      roles.set(
        `${field}:max`,
        values.reduce((a, b) => (b.values[field]! > a.values[field]! ? b : a))
      );
    }
    keep.add(first.sequence);
    keep.add(last.sequence);
    const candidates = [...new Set(roles.values())].filter((row) => !keep.has(row.sequence));
    const selected: ReferenceSample[] = [];
    const uncovered = new Set(roles.keys());
    while (selected.length < 8 && uncovered.size && candidates.length) {
      candidates.sort((left, right) => {
        const leftCoverage = [...roles.entries()].filter(
          ([role, row]) => row.sequence === left.sequence && uncovered.has(role)
        ).length;
        const rightCoverage = [...roles.entries()].filter(
          ([role, row]) => row.sequence === right.sequence && uncovered.has(role)
        ).length;
        return rightCoverage - leftCoverage || compareKey(left, right);
      });
      const choice = candidates.shift()!;
      selected.push(choice);
      keep.add(choice.sequence);
      for (const [role, row] of roles) if (row.sequence === choice.sequence) uncovered.delete(role);
    }
  }
  return [...keep].sort((left, right) =>
    compareKey(rows.find((row) => row.sequence === left)!, rows.find((row) => row.sequence === right)!)
  );
}

export interface CursorState {
  version: 1;
  sessionId: string;
  snapshot: SessionPublishedSnapshot;
  range: SessionDataRange;
  elapsedMs: number;
  sequence: number;
}

export function encodeCursor(value: Omit<CursorState, 'version'>): string {
  const encoded = toBase64Url(JSON.stringify({ version: 1, ...value }));
  if (new TextEncoder().encode(encoded).byteLength > MAX_SESSION_CURSOR_BYTES)
    throw new RangeError('Cursor exceeds its bound');
  return encoded;
}

export function decodeCursor(value: string): CursorState {
  if (new TextEncoder().encode(value).byteLength > MAX_SESSION_CURSOR_BYTES)
    throw new RangeError('Cursor exceeds its bound');
  let parsed: unknown;
  try {
    parsed = JSON.parse(fromBase64Url(value));
  } catch {
    throw new RangeError('Invalid cursor');
  }
  if (!parsed || typeof parsed !== 'object' || (parsed as CursorState).version !== 1)
    throw new RangeError('Invalid cursor');
  const cursor = parsed as CursorState;
  if (
    !Number.isSafeInteger(cursor.elapsedMs) ||
    !Number.isSafeInteger(cursor.sequence) ||
    typeof cursor.sessionId !== 'string'
  )
    throw new RangeError('Invalid cursor');
  return cursor;
}

export function paginateSamples(
  rows: readonly ReferenceSample[],
  limit: number,
  cursor: CursorState | null
): { rows: ReferenceSample[]; nextCursor: CursorState | null } {
  if (!Number.isSafeInteger(limit) || limit < 1 || limit > MAX_SESSION_SAMPLE_ROWS)
    throw new RangeError('Invalid sample page size');
  const sorted = rows.slice().sort(compareKey);
  const start = cursor ? sorted.findIndex((row) => compareKey(row, cursor) > 0) : 0;
  const offset = start < 0 ? sorted.length : start;
  const page = sorted.slice(offset, offset + limit);
  const last = page.at(-1);
  return {
    rows: page,
    nextCursor:
      last && offset + page.length < sorted.length && cursor
        ? { ...cursor, elapsedMs: last.elapsedMs, sequence: last.sequence }
        : last && offset + page.length < sorted.length
          ? null
          : null,
  };
}

export function paginateEvents(
  rows: readonly ReferenceEvent[],
  limit: number,
  cursor: CursorState | null
): { rows: ReferenceEvent[]; nextCursor: CursorState | null } {
  if (!Number.isSafeInteger(limit) || limit < 1 || limit > MAX_SESSION_EVENT_ROWS)
    throw new RangeError('Invalid event page size');
  const sorted = rows.slice().sort((left, right) => left.elapsedMs - right.elapsedMs || left.sequence - right.sequence);
  const offset = cursor
    ? Math.max(
        0,
        sorted.findIndex(
          (row) =>
            row.elapsedMs > cursor.elapsedMs || (row.elapsedMs === cursor.elapsedMs && row.sequence > cursor.sequence)
        )
      )
    : 0;
  const page = sorted.slice(offset, offset + limit);
  const last = page.at(-1);
  return {
    rows: page,
    nextCursor:
      last && offset + page.length < sorted.length && cursor
        ? { ...cursor, elapsedMs: last.elapsedMs, sequence: last.sequence }
        : null,
  };
}

function compareKey(
  left: { elapsedMs: number; sequence: number },
  right: { elapsedMs: number; sequence: number }
): number {
  return left.elapsedMs - right.elapsedMs || left.sequence - right.sequence;
}

function bucketIndex(elapsedMs: number, range: SessionDataRange, buckets: number, span: bigint): number {
  const offset = BigInt(elapsedMs - range.startElapsedMs);
  return Math.min(buckets - 1, Number((offset * BigInt(buckets)) / span));
}

function toBase64Url(value: string): string {
  const bytes = new TextEncoder().encode(value);
  let binary = '';
  bytes.forEach((byte) => {
    binary += String.fromCharCode(byte);
  });
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/g, '');
}

function fromBase64Url(value: string): string {
  const padded = value.replace(/-/g, '+').replace(/_/g, '/') + '='.repeat((4 - (value.length % 4)) % 4);
  const binary = atob(padded);
  return new TextDecoder().decode(Uint8Array.from(binary, (character) => character.charCodeAt(0)));
}
