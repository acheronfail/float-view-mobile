import { describe, expect, it } from 'vitest';
import { NavigationCache, NavigationContext } from './navigation-context';
import type { SessionSummary } from '$lib/backend/contract';

describe('frontend browsing cache', () => {
  it('evicts the least recently used entry and replaces entries without growing', () => {
    const cache = new NavigationCache<number>(2);
    cache.set('a', 1);
    cache.set('b', 2);
    expect(cache.get('a')).toBe(1);
    cache.set('c', 3);
    expect(cache.get('b')).toBeUndefined();
    cache.set('a', 4);
    expect(cache.get('c')).toBe(3);
    expect(cache.get('a')).toBe(4);
    cache.delete('a');
    expect(cache.get('a')).toBeUndefined();
  });

  it('bounds retained boards and starts empty for each frontend owner', () => {
    const context = new NavigationContext();
    for (let i = 0; i < 9; i++)
      context.retainSessions(`board-${i}`, { items: [], catalogRevision: i, nextCursor: null, targetCount: 0 });
    expect(context.sessions.get('board-0')).toBeUndefined();
    expect(context.sessions.get('board-1')?.catalogRevision).toBe(1);
    expect(new NavigationContext().sessions.get('board-1')).toBeUndefined();
  });

  it('drops oversized lists rather than retaining unbounded pages or stale cursors', () => {
    const context = new NavigationContext();
    const items = Array.from({ length: context.maxRetainedSessions }, () => ({}) as SessionSummary);
    context.retainSessions('board', { items, catalogRevision: 1, nextCursor: 'cursor', targetCount: items.length });
    expect(context.sessions.get('board')?.items.length).toBe(context.maxRetainedSessions);
    context.retainSessions('board', {
      items: [...items, {} as SessionSummary],
      catalogRevision: 1,
      nextCursor: 'cursor',
      targetCount: items.length + 1,
    });
    expect(context.sessions.get('board')).toBeUndefined();
  });
});
