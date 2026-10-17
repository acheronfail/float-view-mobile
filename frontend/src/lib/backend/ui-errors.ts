import type { DiagnosticUiError } from './contract';

/** Deliberately receives no exception message, stack, URL or DOM value. */
export function installUiErrorReporting(
  send: (error: DiagnosticUiError) => Promise<unknown>,
  target: Window = window,
  now: () => number = () => performance.now()
): () => void {
  let windowStart = now();
  let count = 0;
  let sending = false;
  let stopped = false;
  const queue: DiagnosticUiError[] = [];
  const flush = async () => {
    if (sending || stopped) return;
    sending = true;
    try {
      while (queue.length && !stopped) {
        try {
          await send(queue[0]!);
          queue.shift();
        } catch {
          break;
        } // Retry on the next event/timer; never report this failure.
      }
    } finally {
      sending = false;
    }
  };
  const report = (category: DiagnosticUiError['category']) => {
    if (now() - windowStart >= 60_000) {
      windowStart = now();
      count = 0;
    }
    if (count >= 10 || queue.length >= 10) return;
    count++;
    queue.push({ category, feature: 'app', line: null, column: null });
    void flush();
  };
  const exception = () => report('exception');
  const rejection = () => report('rejection');
  target.addEventListener('error', exception);
  target.addEventListener('unhandledrejection', rejection);
  const retry = target.setInterval(() => void flush(), 5_000);
  return () => {
    stopped = true;
    queue.length = 0;
    target.removeEventListener('error', exception);
    target.removeEventListener('unhandledrejection', rejection);
    target.clearInterval(retry);
  };
}
