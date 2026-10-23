import { onMount, untrack } from 'svelte';
import { backendState } from '$lib/backend/state.svelte';
import { LiveHistoryBuffer, type LiveHistory } from './live-history';
import type { LiveChartPoint } from './live-charts';

export function createLiveHistory(): LiveHistory {
  const buffer = new LiveHistoryBuffer();
  let points = $state<LiveChartPoint[]>([]);
  let now = $state(0);
  let visible = $state(false);
  $effect(() => {
    const connection = backendState.snapshot?.activeConnection;
    const status = backendState.status;
    const foreground = visible;
    untrack(() => {
      now = performance.now();
      buffer.accept(connection, Date.now(), now, status === 'connected' && foreground);
      points = buffer.points;
    });
  });
  onMount(() => {
    let timer: ReturnType<typeof setInterval> | undefined;
    const visibility = () => {
      visible = !document.hidden;
      clearInterval(timer);
      if (visible)
        timer = setInterval(() => {
          now = performance.now();
          buffer.prune(now);
          points = buffer.points;
        }, 100);
    };
    visibility();
    document.addEventListener('visibilitychange', visibility);
    return () => {
      clearInterval(timer);
      document.removeEventListener('visibilitychange', visibility);
      buffer.points = [];
    };
  });
  return {
    get points() {
      return points;
    },
    get now() {
      return now;
    },
    get stale() {
      return (
        backendState.snapshot?.activeConnection?.state !== 'ready' ||
        !points.length ||
        now - points[points.length - 1].time > 1500
      );
    },
  };
}
