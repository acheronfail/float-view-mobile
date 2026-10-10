import type { SessionMetadata, SessionLocationPoint, SessionLocationGap } from '$lib/backend/contract';
import type { SessionReviewController } from '$lib/backend/session-controller';
import { sessionDataDecodedBytes } from '$lib/backend/session-data';

export async function loadLocationTrack(
  controller: Pick<SessionReviewController, 'loadLocations'>,
  metadata: SessionMetadata
) {
  const points: SessionLocationPoint[] = [],
    gaps: SessionLocationGap[] = [];
  const cursors = new Set<string>();
  let cursor: string | null = null,
    bytes = 0;
  if (!metadata.location.available) return { points, gaps };
  for (let page = 0; page < 128; page++) {
    const result = await controller.loadLocations({
      kind: 'locations',
      range: { startElapsedMs: 0, endElapsedMs: metadata.summary.totalElapsedMs },
      limit: 2000,
      cursor,
    });
    bytes += sessionDataDecodedBytes(result) ?? JSON.stringify(result).length * 2;
    if (
      points.length + result.points.length > 100_000 ||
      gaps.length + result.gaps.length > 100_000 ||
      bytes > 16 * 1024 * 1024
    )
      throw new Error('Track exceeds map review limits. Charts remain available.');
    points.push(...result.points);
    gaps.push(...result.gaps);
    cursor = result.nextCursor;
    if (!cursor) return { points, gaps };
    if (cursors.has(cursor)) throw new Error('Track pagination did not advance. Retry loading the track.');
    cursors.add(cursor);
  }
  throw new Error('Track exceeds map review page limit. Charts remain available.');
}
