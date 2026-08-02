import type { SessionSeriesColumn } from '$lib/backend/session-data';
import type { Viewport } from './viewport';

export interface NumericDomain {
  min: number;
  max: number;
}

export interface PlotPathOptions {
  width: number;
  height: number;
  viewport: Viewport;
}

export function finiteDomain(values: readonly (number | null)[]): NumericDomain | null {
  const finite = values.filter((value): value is number => value !== null && Number.isFinite(value));
  if (!finite.length) return null;
  const minimum = Math.min(...finite);
  const maximum = Math.max(...finite);
  const padding = minimum === maximum ? Math.max(1, Math.abs(minimum) * 0.05) : (maximum - minimum) * 0.05;
  return { min: minimum - padding, max: maximum + padding };
}

export function buildPathSegments(column: SessionSeriesColumn, options: PlotPathOptions): string[] {
  const domain = finiteDomain(column.values);
  if (!domain) return [];
  const span = Math.max(1, options.viewport.endElapsedMs - options.viewport.startElapsedMs);
  const height = Math.max(2, options.height);
  const width = Math.max(1, options.width);
  const segments: string[] = [];
  let current = '';
  column.values.forEach((value, index) => {
    const previousGeneration = index ? column.generations[index - 1] : column.generations[index];
    const generationBreak = index > 0 && column.generations[index] !== previousGeneration;
    if (value === null || !Number.isFinite(value) || generationBreak) {
      if (current) segments.push(current);
      current = '';
      return;
    }
    const elapsed = column.elapsedMs[index] ?? options.viewport.startElapsedMs;
    const x = ((elapsed - options.viewport.startElapsedMs) / span) * width;
    const y = height - ((value - domain.min) / Math.max(Number.EPSILON, domain.max - domain.min)) * (height - 4) - 2;
    current += `${current ? ' L' : 'M'} ${x.toFixed(2)} ${y.toFixed(2)}`;
  });
  if (current) segments.push(current);
  return segments;
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
