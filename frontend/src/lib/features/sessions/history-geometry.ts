import type { SessionSeriesColumn } from '$lib/backend/session-data';
import type { Viewport } from './viewport';
import type { SessionAxisPolicy } from './session-fields';

export interface NumericDomain {
  min: number;
  max: number;
}

export interface PlotPathOptions {
  width: number;
  height: number;
  viewport: Viewport;
  domain?: NumericDomain;
}

export interface PlotPoint {
  elapsedMs: number;
  value: number;
  x: number;
  y: number;
}

export function finiteDomain(
  values: readonly (number | null)[],
  policy: SessionAxisPolicy = { kind: 'auto' }
): NumericDomain | null {
  if (policy.kind === 'fixed') {
    if (!Number.isFinite(policy.min) || !Number.isFinite(policy.max) || policy.max <= policy.min) return null;
    return { min: policy.min, max: policy.max };
  }
  const finite = values.filter((value): value is number => value !== null && Number.isFinite(value));
  if (!finite.length) return null;
  const minimum = Math.min(...finite);
  const maximum = Math.max(...finite);
  const padding = minimum === maximum ? Math.max(1, Math.abs(minimum) * 0.05) : (maximum - minimum) * 0.05;
  return { min: minimum - padding, max: maximum + padding };
}

export function buildPathSegments(column: SessionSeriesColumn, options: PlotPathOptions): string[] {
  const domain = options.domain ?? finiteDomain(column.values);
  if (!domain) return [];
  const span = Math.max(1, options.viewport.endElapsedMs - options.viewport.startElapsedMs);
  const height = Math.max(2, options.height);
  const width = Math.max(1, options.width);
  const segments: string[] = [];
  let current = '';
  const flush = () => {
    // A zero-length line with a round cap renders isolated readings as dots.
    if (current) segments.push(current.includes(' L') ? current : `${current} l 0 0`);
    current = '';
  };
  column.values.forEach((value, index) => {
    const previousGeneration = index ? column.generations[index - 1] : column.generations[index];
    const generationBreak = index > 0 && column.generations[index] !== previousGeneration;
    if (generationBreak) flush();
    if (value === null || !Number.isFinite(value)) {
      flush();
      return;
    }
    const elapsed = column.elapsedMs[index] ?? options.viewport.startElapsedMs;
    const x = ((elapsed - options.viewport.startElapsedMs) / span) * width;
    const y = height - ((value - domain.min) / Math.max(Number.EPSILON, domain.max - domain.min)) * (height - 4) - 2;
    current += `${current ? ' L' : 'M'} ${x.toFixed(2)} ${y.toFixed(2)}`;
  });
  flush();
  return segments;
}

export function pointAtElapsed(
  column: SessionSeriesColumn,
  elapsedMs: number,
  domain: NumericDomain,
  options: PlotPathOptions
): PlotPoint | null {
  const span = Math.max(1, options.viewport.endElapsedMs - options.viewport.startElapsedMs);
  const height = Math.max(2, options.height);
  const width = Math.max(1, options.width);
  const toPoint = (pointElapsedMs: number, value: number): PlotPoint => ({
    elapsedMs: pointElapsedMs,
    value,
    x: ((pointElapsedMs - options.viewport.startElapsedMs) / span) * width,
    y: height - ((value - domain.min) / Math.max(Number.EPSILON, domain.max - domain.min)) * (height - 4) - 2,
  });

  for (let index = 0; index < column.values.length; index += 1) {
    const value = column.values[index];
    if (column.elapsedMs[index] === elapsedMs && value !== null && Number.isFinite(value)) {
      return toPoint(elapsedMs, value);
    }
  }

  for (let index = 1; index < column.values.length; index += 1) {
    const leftValue = column.values[index - 1];
    const rightValue = column.values[index];
    const leftElapsedMs = column.elapsedMs[index - 1];
    const rightElapsedMs = column.elapsedMs[index];
    if (
      leftValue === null ||
      rightValue === null ||
      !Number.isFinite(leftValue) ||
      !Number.isFinite(rightValue) ||
      column.generations[index] !== column.generations[index - 1] ||
      rightElapsedMs <= leftElapsedMs ||
      elapsedMs < leftElapsedMs ||
      elapsedMs > rightElapsedMs
    ) {
      continue;
    }
    const ratio = (elapsedMs - leftElapsedMs) / (rightElapsedMs - leftElapsedMs);
    return toPoint(elapsedMs, leftValue + (rightValue - leftValue) * ratio);
  }
  return null;
}

export function elapsedTicks(viewport: Viewport, count = 4): number[] {
  const ticks = Math.max(2, Math.round(count));
  const span = Math.max(1, viewport.endElapsedMs - viewport.startElapsedMs);
  return Array.from({ length: ticks }, (_, index) =>
    Math.round(viewport.startElapsedMs + (span * index) / (ticks - 1))
  );
}

export function axisTicks(domain: NumericDomain | null, count = 5): number[] {
  if (!domain) return [];
  const ticks = Math.max(2, Math.round(count));
  const span = domain.max - domain.min;
  if (!Number.isFinite(span) || span <= 0) return [domain.min];
  return Array.from({ length: ticks }, (_, index) => domain.min + (span * index) / (ticks - 1));
}

export function formatAxisTick(value: number): string {
  if (!Number.isFinite(value)) return '';
  return Number(value.toPrecision(3)).toString();
}

export function zeroReference(domain: NumericDomain | null): number | null {
  if (!domain || domain.min > 0 || domain.max < 0) return null;
  return domain.max === domain.min ? 0.5 : (0 - domain.min) / (domain.max - domain.min);
}
