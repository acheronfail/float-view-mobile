import type { LiveSample } from '$lib/backend/contract';

export function footpadPresentation(sample: LiveSample | null | undefined, side: 'left' | 'right') {
  const enabled = sample?.footpadState === 'both' || sample?.footpadState === side;
  if (enabled) return { background: 'bg-success', text: 'text-success', label: 'Enabled' };
  if ((sample?.speedKph ?? 0) > 2)
    return { background: 'bg-danger', text: 'text-danger', label: 'Disabled while moving' };
  return { background: 'bg-surface', text: 'text-muted', label: 'Disabled' };
}
