/** One visible-page metadata request at a time; late responses never publish. */
export function startRideRefresh(options: {
  shouldRefresh: () => boolean;
  refresh: (accept: () => boolean) => Promise<void>;
  onerror: (reason: unknown) => void;
}) {
  let active = true;
  let pending = false;
  let visibilityGeneration = 0;
  const refresh = async () => {
    if (!active || pending || document.hidden || !options.shouldRefresh()) return;
    const generation = visibilityGeneration;
    const accept = () => active && !document.hidden && generation === visibilityGeneration && options.shouldRefresh();
    pending = true;
    try {
      await options.refresh(accept);
    } catch (reason) {
      if (accept()) options.onerror(reason);
    } finally {
      pending = false;
    }
  };
  const visibilityChanged = () => {
    visibilityGeneration++;
    if (!document.hidden) void refresh();
  };
  const timer = setInterval(() => void refresh(), 3_000);
  document.addEventListener('visibilitychange', visibilityChanged);
  return {
    refresh,
    dispose() {
      active = false;
      clearInterval(timer);
      document.removeEventListener('visibilitychange', visibilityChanged);
    },
  };
}
