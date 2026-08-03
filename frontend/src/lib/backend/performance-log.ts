type PerformanceMessageHandler = {
  postMessage(message: unknown): void;
};

type WebKitGlobal = {
  messageHandlers?: {
    floatViewPerformance?: PerformanceMessageHandler;
  };
};

type AndroidPerformanceBridge = {
  report(message: string): void;
};

function webKitGlobal(): WebKitGlobal | undefined {
  return (globalThis as typeof globalThis & { webkit?: WebKitGlobal }).webkit;
}

function androidPerformanceBridge(): AndroidPerformanceBridge | undefined {
  return (globalThis as typeof globalThis & { FloatViewPerformance?: AndroidPerformanceBridge }).FloatViewPerformance;
}

export function reportFrontendPerformance(event: string, detail: Record<string, unknown>): void {
  const message = { event, ...detail };
  try {
    webKitGlobal()?.messageHandlers?.floatViewPerformance?.postMessage(message);
  } catch {
    // Performance diagnostics must never affect session review.
  }
  try {
    androidPerformanceBridge()?.report(JSON.stringify(message));
  } catch {
    // Performance diagnostics must never affect session review.
  }
  if (import.meta.env.DEV) console.debug(`[Float View performance] ${event}`, detail);
}
