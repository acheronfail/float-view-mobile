/** A presentation capability only: hosts never receive routes or navigation commands. */
export function publishHostBackNavigation(available: boolean) {
  const host = globalThis as typeof globalThis & {
    webkit?: { messageHandlers?: { floatViewBackNavigation?: { postMessage(value: boolean): void } } };
  };
  try {
    host.webkit?.messageHandlers?.floatViewBackNavigation?.postMessage(available);
  } catch {
    // Ordinary links remain available if the host bridge is unavailable.
  }
}
