export type ExactTooltipStatus = 'idle' | 'pending' | 'exact' | 'unavailable';

export interface ExactTooltipValue {
  elapsedMs: number;
  values: Readonly<Record<string, number | null>>;
}

export interface ExactTooltipSnapshot {
  status: ExactTooltipStatus;
  field: string;
  targetElapsedMs: number;
  value: ExactTooltipValue | null;
  pinned: boolean;
}

export type ExactTooltipLookup = (targetElapsedMs: number) => Promise<ExactTooltipValue | null>;

/**
 * Keeps exact inspection bounded to one request and one latest coalesced target.
 * The chart can move its crosshair immediately while this object prevents an
 * older nearest-sample response from replacing the newest inspection.
 */
export class ExactTooltipController {
  private disposed = false;
  private generation = 0;
  private inFlight = false;
  private queuedTarget: number | null = null;
  private current: ExactTooltipSnapshot;

  constructor(
    private readonly lookup: ExactTooltipLookup,
    private readonly onChange?: (snapshot: ExactTooltipSnapshot) => void
  ) {
    this.current = {
      status: 'idle',
      field: '',
      targetElapsedMs: 0,
      value: null,
      pinned: false,
    };
  }

  get snapshot(): ExactTooltipSnapshot {
    return this.current;
  }

  inspect(targetElapsedMs: number, field: string, pinned: boolean): void {
    if (this.disposed) return;
    const target = Math.max(0, Math.round(targetElapsedMs));
    this.current = {
      status: 'pending',
      field,
      targetElapsedMs: target,
      value: null,
      pinned,
    };
    this.emit();
    if (this.inFlight) {
      this.queuedTarget = target;
      return;
    }
    void this.run(target, this.generation);
  }

  dismiss(): void {
    if (this.disposed) return;
    this.generation += 1;
    this.queuedTarget = null;
    this.current = { ...this.current, status: 'idle', value: null, pinned: false };
    this.emit();
  }

  dispose(): void {
    this.disposed = true;
    this.generation += 1;
    this.queuedTarget = null;
  }

  private async run(targetElapsedMs: number, generation: number): Promise<void> {
    this.inFlight = true;
    try {
      let target = targetElapsedMs;
      let activeGeneration = generation;
      while (true) {
        const value = await this.lookup(target);
        if (this.disposed) return;
        if (activeGeneration !== this.generation) {
          if (this.queuedTarget === null) return;
          target = this.queuedTarget;
          this.queuedTarget = null;
          activeGeneration = this.generation;
          continue;
        }
        if (this.queuedTarget !== null) {
          target = this.queuedTarget;
          this.queuedTarget = null;
          continue;
        }
        const exact = value?.values[this.current.field] ?? null;
        this.current = {
          ...this.current,
          status: exact === null ? 'unavailable' : 'exact',
          value,
        };
        this.emit();
        return;
      }
    } catch {
      if (this.disposed) return;
      if (generation !== this.generation && this.queuedTarget === null) return;
      if (this.queuedTarget !== null) {
        const next = this.queuedTarget;
        this.queuedTarget = null;
        await this.run(next, this.generation);
        return;
      }
      this.current = { ...this.current, status: 'unavailable', value: null };
      this.emit();
    } finally {
      this.inFlight = false;
    }
  }

  private emit(): void {
    this.onChange?.(this.current);
  }
}

export interface TooltipPlacement {
  leftPx: number;
  side: 'before' | 'after';
}

export function tooltipPlacement(anchorPx: number, tooltipWidthPx: number, viewportWidthPx: number): TooltipPlacement {
  const viewport = Math.max(1, viewportWidthPx);
  const tooltip = Math.max(1, tooltipWidthPx);
  const anchor = Math.max(0, Math.min(viewport, anchorPx));
  const gap = 4;
  const after = anchor + gap + tooltip <= viewport;
  const preferred = after ? anchor + gap : anchor - gap - tooltip;
  return {
    leftPx: Math.max(0, Math.min(Math.max(0, viewport - tooltip), preferred)),
    side: after ? 'after' : 'before',
  };
}

export function formatElapsedForAssistive(elapsedMs: number, durationMs: number): string {
  const elapsed = Math.max(0, Math.round(elapsedMs));
  const duration = Math.max(1, Math.round(durationMs));
  const format = (value: number) => {
    const minutes = Math.floor(value / 60_000);
    const seconds = Math.floor((value % 60_000) / 1_000);
    return minutes ? `${minutes}m ${seconds}s` : `${seconds}s`;
  };
  return `Elapsed ${format(elapsed)} of ${format(duration)}`;
}
