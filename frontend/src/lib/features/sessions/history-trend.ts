import type { SessionSeriesColumn } from '$lib/backend/session-data';
import { buildPathSegments, type PlotPathOptions } from './history-geometry';
import { timeline } from './map-filter';

/** Average the retained, piecewise-linear readings over ~8 screen pixels.
 * The backend's extrema reduction is not a uniformly sampled series: weighting
 * by elapsed time avoids giving clustered spike samples disproportionate weight.
 * Zooming reduces the time window; below one second we show the retained line.
 */
export function historyTrend(
  column: SessionSeriesColumn,
  options: PlotPathOptions,
  screenWidth: number,
  enabled = true
) {
  const axis = timeline(options.intervals ?? [options.viewport]);
  const duration = axis.duration;
  const windowMs = (duration * 8) / Math.max(1, screenWidth);
  const raw = () => ({ paths: buildPathSegments(column, options), bands: [] as string[], smoothed: false });
  if (!enabled || !options.domain || windowMs <= 1000) return raw();

  const paths: string[] = [];
  const bands: string[] = [];
  let smoothed = false;
  let run: { time: number; value: number; generation: number; interval: number }[] = [];
  const y = (value: number) =>
    options.height -
    ((value - options.domain!.min) / Math.max(Number.EPSILON, options.domain!.max - options.domain!.min)) *
      (options.height - 4) -
    2;
  const x = (time: number) => (axis.ratioAt(time) ?? 0) * options.width;
  const point = (time: number, value: number) => `${x(time).toFixed(2)} ${y(value).toFixed(2)}`;
  function flush() {
    if (!run.length) return;
    if (run.length < 4) {
      paths.push(
        `M ${run.map((sample) => point(sample.time, sample.value)).join(' L ')}${run.length === 1 ? ' l 0 0' : ''}`
      );
      run = [];
      return;
    }
    const start = run[0].time;
    const end = run.at(-1)!.time;
    const bins: { time: number; mean: number; min: number; max: number }[] = [];
    let edge = 1;
    for (let left = start; left < end; left += windowMs) {
      const right = Math.min(end, left + windowMs);
      let area = 0;
      let min = Infinity;
      let max = -Infinity;
      while (edge < run.length) {
        const a = run[edge - 1];
        const b = run[edge];
        const from = Math.max(left, a.time);
        const to = Math.min(right, b.time);
        if (to > from) {
          const at = (time: number) => a.value + ((time - a.time) / (b.time - a.time)) * (b.value - a.value);
          const low = at(from),
            high = at(to);
          area += ((low + high) / 2) * (to - from);
          min = Math.min(min, low, high);
          max = Math.max(max, low, high);
        }
        if (b.time >= right) break;
        edge++;
      }
      if (min !== Infinity) bins.push({ time: (left + right) / 2, mean: area / (right - left), min, max });
    }
    if (bins.length) {
      smoothed = true;
      // Extend only to this continuous run's endpoints, never across a gap.
      const extended = [{ ...bins[0], time: start }, ...bins, { ...bins.at(-1)!, time: end }];
      paths.push(curvedTrendPath(extended.map((bin) => ({ x: x(bin.time), y: y(bin.mean) }))));
      bands.push(
        `M ${extended.map((bin) => point(bin.time, bin.max)).join(' L ')} L ${extended
          .toReversed()
          .map((bin) => point(bin.time, bin.min))
          .join(' L ')} Z`
      );
    }
    run = [];
  }
  column.values.forEach((value, index) => {
    const time = column.elapsedMs[index];
    const interval = axis.indexAt(time);
    if (value === null || !Number.isFinite(value) || interval < 0) {
      flush();
      return;
    }
    const previous = run.at(-1);
    const generation = column.generations[index];
    if (previous && (previous.generation !== generation || previous.interval !== interval || time <= previous.time))
      flush();
    run.push({ time, value, generation, interval });
  });
  flush();
  return { paths, bands, smoothed };
}

/** Cubic Hermite segments with shared, bounded tangents. Flatten extrema and
 * plateaus so interpolation cannot invent peaks between averaged readings.
 */
function curvedTrendPath(points: { x: number; y: number }[]): string {
  const slopes = points.slice(1).map((point, index) => (point.y - points[index].y) / (point.x - points[index].x));
  const tangents = points.map((_, index) => {
    if (index === 0) return slopes[0];
    if (index === points.length - 1) return slopes.at(-1)!;
    const before = slopes[index - 1];
    const after = slopes[index];
    if (before * after <= 0) return 0;
    // The harmonic mean is at most twice the smaller adjacent slope, which
    // bounds each Bezier control point within its segment's endpoint values.
    return (2 * before * after) / (before + after);
  });
  const pair = (x: number, y: number) => `${x.toFixed(2)} ${y.toFixed(2)}`;
  let path = `M ${pair(points[0].x, points[0].y)}`;
  for (let index = 1; index < points.length; index++) {
    const left = points[index - 1];
    const right = points[index];
    const third = (right.x - left.x) / 3;
    path += ` C ${pair(left.x + third, left.y + tangents[index - 1] * third)} ${pair(right.x - third, right.y - tangents[index] * third)} ${pair(right.x, right.y)}`;
  }
  return path;
}
