import type { Activation } from './activation';

type Dismissal = { close: Activation; enabled?: boolean };
const surfaces = new Map<HTMLElement, Dismissal>();

/** Register the same close path used by the visible Done/Cancel control. */
export function backDismissal(node: HTMLElement, dismissal: Dismissal) {
  surfaces.set(node, dismissal);
  return {
    update(value: Dismissal) {
      surfaces.set(node, value);
    },
    destroy() {
      surfaces.delete(node);
    },
  };
}

export function dismissTopSurface(): boolean {
  const visible = [...surfaces].filter(
    ([node, value]) => value.enabled !== false && node.isConnected && !node.inert && node.getClientRects().length > 0
  );
  const layer = (node: HTMLElement) => {
    if (node.closest('dialog:modal')) return Number.MAX_SAFE_INTEGER;
    let z = 0;
    for (let parent: HTMLElement | null = node; parent; parent = parent.parentElement) {
      z = Math.max(z, Number.parseInt(getComputedStyle(parent).zIndex) || 0);
    }
    return z;
  };
  visible.sort(
    ([a], [b]) => layer(a) - layer(b) || (a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING ? -1 : 1)
  );
  const top = visible.at(-1);
  if (!top) return false;
  void top[1].close();
  return true;
}

/** Escape dismisses surfaces; native Back also follows the current route's return link. */
export function installBackNavigation(back: () => boolean) {
  const escape = (event: KeyboardEvent) => {
    if (event.key !== 'Escape' || event.isComposing || event.defaultPrevented || !dismissTopSurface()) return;
    event.preventDefault();
    event.stopImmediatePropagation();
  };
  const nativeBack = (event: Event) => {
    if (dismissTopSurface() || back()) event.preventDefault();
  };
  window.addEventListener('keydown', escape, true);
  window.addEventListener('floatview:back', nativeBack);
  return () => {
    window.removeEventListener('keydown', escape, true);
    window.removeEventListener('floatview:back', nativeBack);
  };
}
