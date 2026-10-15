// Pointer Events still own inspection. This action only arbitrates native scrolling:
// cancelling pointermove or changing touch-action mid-gesture cannot stop a scroll.
export function chartTouchInspection(node: SVGElement, enabled: boolean) {
  let start: { id: number; x: number; y: number } | null = null;
  let intent: 'pending' | 'inspect' | 'scroll' = 'pending';

  function reset() {
    start = null;
    intent = 'pending';
  }

  function begin(event: TouchEvent) {
    if (!enabled || start || event.touches.length !== 1) return;
    const touch = event.touches[0]!;
    start = { id: touch.identifier, x: touch.clientX, y: touch.clientY };
  }

  function move(event: TouchEvent) {
    if (!start) return;
    const touch = Array.from(event.touches).find((touch) => touch.identifier === start!.id);
    if (!touch) return;
    if (intent === 'pending') {
      if (event.touches.length !== 1) {
        intent = 'scroll';
        return;
      }
      const dx = Math.abs(touch.clientX - start.x);
      const dy = Math.abs(touch.clientY - start.y);
      // Decide early, before the browser's scroll threshold. Once chosen, retain
      // ownership until release so vertical finger drift cannot steal inspection.
      if (Math.max(dx, dy) < 4) return;
      intent = dx > dy ? 'inspect' : 'scroll';
    }
    if (intent === 'inspect' && event.cancelable) event.preventDefault();
  }

  function end(event: TouchEvent) {
    if (!event.touches.length) reset();
  }

  node.addEventListener('touchstart', begin, { passive: true });
  node.addEventListener('touchmove', move, { passive: false });
  node.addEventListener('touchend', end);
  node.addEventListener('touchcancel', reset);
  return {
    update(value: boolean) {
      enabled = value;
      reset();
    },
    destroy() {
      node.removeEventListener('touchstart', begin);
      node.removeEventListener('touchmove', move);
      node.removeEventListener('touchend', end);
      node.removeEventListener('touchcancel', reset);
    },
  };
}
