import type { DiagnosticUiError } from './contract';

/** Receives only closed categories. Never accepts an Error, payload, URL or identifier. */
export class RequestFailureReporter {
  private readonly queue: DiagnosticUiError[] = [];
  private sender: ((failure: DiagnosticUiError) => Promise<unknown>) | null = null;
  private sending = false;
  private windowStart = 0;
  private count = 0;

  constructor(private readonly now: () => number = () => performance.now()) {}

  attach(sender: (failure: DiagnosticUiError) => Promise<unknown>): () => void {
    this.sender = sender;
    this.flush();
    return () => {
      if (this.sender === sender) this.sender = null;
    };
  }

  record(category: 'request' | 'protocol', feature: DiagnosticUiError['feature'] = 'app') {
    if (this.now() - this.windowStart >= 60_000) {
      this.windowStart = this.now();
      this.count = 0;
    }
    if (this.count >= 10 || this.queue.length >= 10) return;
    this.count++;
    this.queue.push({ category, feature, line: null, column: null });
    this.flush();
  }

  flush() {
    if (this.sending || !this.sender || !this.queue.length) return;
    this.sending = true;
    void (async () => {
      try {
        while (this.sender && this.queue.length) {
          const sender = this.sender;
          try {
            await sender(this.queue[0]!);
            this.queue.shift();
          } catch {
            if (this.sender && this.sender !== sender) continue;
            break;
          } // Retry on the next ordinary request/reconnection, never recursively.
        }
      } finally {
        this.sending = false;
      }
    })();
  }
}

export const requestFailures = new RequestFailureReporter();

/** Includes HTTP response/body validation failures; deliberate cancellation is not an app failure. */
export async function reportSessionRequestFailure<T>(action: () => Promise<T>, signal?: AbortSignal): Promise<T> {
  try {
    return await action();
  } catch (error) {
    if (!signal?.aborted) requestFailures.record('request', 'sessions');
    throw error;
  }
}
