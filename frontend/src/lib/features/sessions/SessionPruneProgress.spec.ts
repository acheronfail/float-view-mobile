import { render } from 'svelte/server';
import { describe, expect, it, vi } from 'vitest';
import type { SessionOperationState } from '$lib/backend/contract';
import SessionPruneProgress from './SessionPruneProgress.svelte';

const operation: SessionOperationState = {
  operationId: 'prune-1',
  kind: 'prune',
  status: 'reclaiming',
  sessionId: 'session-1',
  scope: 'one',
  targetResolutionSeconds: 5,
  completedSessions: 0,
  eligibleSessions: 1,
  error: null,
};

describe('SessionPruneProgress', () => {
  it.each([
    ['queued', 'Pruning sessions…'],
    ['running', 'Pruning sessions…'],
    ['reclaiming', 'Reclaiming space…'],
    ['cancelling', 'Cancelling prune…'],
  ] as const)('shows %s progress and cancellation', (status, label) => {
    const { body } = render(SessionPruneProgress, {
      props: { operation: { ...operation, status }, oncancel: vi.fn() },
    });
    expect(body).toContain(label);
    expect(body).toContain('role="status"');
    expect(body).toContain('>Cancel</button>');
    expect(body.includes('disabled')).toBe(status === 'cancelling');
  });

  it.each(['completed', 'cancelled', 'failed'] as const)('hides terminal %s progress', (status) => {
    expect(
      render(SessionPruneProgress, {
        props: { operation: { ...operation, status }, oncancel: vi.fn() },
      }).body
    ).not.toContain('role="status"');
  });
});
