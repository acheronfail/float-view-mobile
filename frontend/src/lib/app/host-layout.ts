declare global {
  interface Window {
    floatViewHostLayout?: unknown;
  }
}

/** Read-only host UI facts. This is separate from authenticated product state. */
export type HostLayout = {
  version: 1;
  edge: 'left' | 'right' | null;
  bounds: { width: number; height: number };
  safeArea: { top: number; right: number; bottom: number; left: number };
  occlusions: { x: number; y: number; width: number; height: number }[];
};

export type SideControls = {
  edge: 'left' | 'right';
  x: number;
  y: number;
  width: number;
  height: number;
  space: number;
};
const object = (value: unknown): value is Record<string, unknown> => !!value && typeof value === 'object';
const finite = (value: unknown): value is number => typeof value === 'number' && Number.isFinite(value);

export function readHostLayout(value: unknown): HostLayout | null {
  if (!object(value) || value.version !== 1 || (value.edge !== 'left' && value.edge !== 'right' && value.edge !== null))
    return null;
  if (!object(value.bounds) || !object(value.safeArea) || !Array.isArray(value.occlusions)) return null;
  const { width, height } = value.bounds;
  if (!finite(width) || !finite(height) || width <= 0 || height <= 0 || width > 10000 || height > 10000) return null;
  for (const side of ['top', 'right', 'bottom', 'left']) {
    const inset = value.safeArea[side];
    if (!finite(inset) || inset < 0 || inset > (side === 'left' || side === 'right' ? width : height)) return null;
  }
  if (value.occlusions.length > 16) return null;
  for (const rect of value.occlusions) {
    if (!object(rect) || !['x', 'y', 'width', 'height'].every((key) => finite(rect[key]))) return null;
    if (
      Number(rect.width) < 0 ||
      Number(rect.height) < 0 ||
      ['x', 'y', 'width', 'height'].some((key) => Math.abs(rect[key] as number) > 20000)
    )
      return null;
  }
  const valid = value as HostLayout;
  return {
    version: 1,
    edge: valid.edge,
    bounds: { width, height },
    safeArea: {
      top: valid.safeArea.top,
      right: valid.safeArea.right,
      bottom: valid.safeArea.bottom,
      left: valid.safeArea.left,
    },
    occlusions: valid.occlusions.map(({ x, y, width, height }) => ({ x, y, width, height })),
  };
}

/** Select the largest clear vertical interval in the system's side column. */
export function sideControls(
  layout: HostLayout | null,
  viewport: { width: number; height: number; fontSize?: number; scale?: number }
): SideControls | null {
  if (!layout?.edge || (viewport.scale ?? 1) !== 1) return null;
  if (Math.abs(layout.bounds.width - viewport.width) > 1 || Math.abs(layout.bounds.height - viewport.height) > 1)
    return null;
  const fontSize = viewport.fontSize ?? 16;
  if (!finite(viewport.width) || !finite(viewport.height) || !finite(fontSize) || fontSize <= 0) return null;
  const space = layout.safeArea[layout.edge];
  // These are control-size requirements, not device dimensions. Large text falls back safely.
  const width = space - 8;
  if (width < 4 * fontSize || space > viewport.width / 2) return null;
  const x = layout.edge === 'right' ? viewport.width - space + 4 : 4;
  const top = Math.max(8, layout.safeArea.top + 8);
  const bottom = viewport.height - Math.max(8, layout.safeArea.bottom + 8);
  let intervals: [number, number][] = [[top, bottom]];
  for (const rect of layout.occlusions) {
    if (rect.width <= 0 || rect.height <= 0 || rect.x >= x + width || rect.x + rect.width <= x) continue;
    const start = rect.y - 8;
    const end = rect.y + rect.height + 8;
    // Discard empty ranges so nested occlusions cannot multiply zero-length intervals.
    intervals = intervals
      .flatMap<[number, number]>(([a, b]) =>
        end <= a || start >= b
          ? [[a, b]]
          : [
              [a, Math.max(a, start)],
              [Math.min(b, end), b],
            ]
      )
      .filter(([a, b]) => b > a);
  }
  const best = intervals.sort((a, b) => b[1] - b[0] - (a[1] - a[0]))[0];
  if (!best || best[1] - best[0] < 20 * fontSize) return null;
  return { edge: layout.edge, x, y: best[0], width, height: best[1] - best[0], space };
}

export function installHostLayout() {
  const root = document.documentElement;
  let layout: HostLayout | null = null;
  const probe = document.createElement('div');
  probe.setAttribute('aria-hidden', 'true');
  probe.style.cssText = 'position:fixed;top:0;left:0;width:1rem;height:1rem;visibility:hidden;pointer-events:none';
  document.body.append(probe);
  const clear = () => {
    delete root.dataset.controlsEdge;
    for (const name of ['x', 'y', 'width', 'height', 'space-left', 'space-right'])
      root.style.removeProperty(`--app-controls-${name}`);
  };
  const update = () => {
    const region = sideControls(layout, {
      width: innerWidth,
      height: innerHeight,
      fontSize: parseFloat(getComputedStyle(root).fontSize),
      scale: window.visualViewport?.scale ?? 1,
    });
    if (!region) {
      clear();
      return;
    }
    root.dataset.controlsEdge = region.edge;
    for (const [name, value] of Object.entries({
      x: region.x,
      y: region.y,
      width: region.width,
      height: region.height,
      'space-left': region.edge === 'left' ? region.space : 0,
      'space-right': region.edge === 'right' ? region.space : 0,
    })) {
      root.style.setProperty(`--app-controls-${name}`, `${value}px`);
    }
  };
  const receive = () => {
    layout = readHostLayout(window.floatViewHostLayout);
    update();
  };
  const observer = new ResizeObserver(update);
  observer.observe(probe);
  window.addEventListener('floatview:layout', receive);
  window.addEventListener('resize', update);
  window.visualViewport?.addEventListener('resize', update);
  receive();
  return () => {
    observer.disconnect();
    probe.remove();
    window.removeEventListener('floatview:layout', receive);
    window.removeEventListener('resize', update);
    window.visualViewport?.removeEventListener('resize', update);
    clear();
  };
}
