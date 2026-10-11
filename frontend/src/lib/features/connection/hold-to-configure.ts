export const BOX_HOLD_MS = 600;
const MOVE_TOLERANCE_PX = 10;

/** A hold configures; only a stationary short release can tap. */
export function holdToConfigure(
  node: HTMLButtonElement,
  options: {
    key: string;
    onhold: () => void;
    ontap: () => void;
  }
) {
  let timer: ReturnType<typeof setTimeout> | undefined;
  let pointer: { id: number; x: number; y: number; held: boolean } | null = null;
  const cancel = () => {
    clearTimeout(timer);
    timer = undefined;
    pointer = null;
  };
  const down = (event: PointerEvent) => {
    if (!event.isPrimary || event.button !== 0) {
      cancel();
      return;
    }
    cancel();
    pointer = { id: event.pointerId, x: event.clientX, y: event.clientY, held: false };
    node.setPointerCapture?.(event.pointerId);
    timer = setTimeout(() => {
      if (!pointer) return;
      pointer.held = true;
      options.onhold();
    }, BOX_HOLD_MS);
  };
  const move = (event: PointerEvent) => {
    if (pointer?.id !== event.pointerId) return;
    if (Math.hypot(event.clientX - pointer.x, event.clientY - pointer.y) > MOVE_TOLERANCE_PX) cancel();
  };
  const up = (event: PointerEvent) => {
    if (pointer?.id !== event.pointerId) return;
    const tap =
      !pointer.held &&
      event.isPrimary &&
      event.button === 0 &&
      Math.hypot(event.clientX - pointer.x, event.clientY - pointer.y) <= MOVE_TOLERANCE_PX;
    cancel();
    if (tap) options.ontap();
  };
  const cancelPointer = (event: PointerEvent) => {
    if (pointer?.id === event.pointerId) cancel();
  };
  const contextMenu = (event: Event) => event.preventDefault();
  node.addEventListener('pointerdown', down);
  node.addEventListener('pointermove', move);
  node.addEventListener('pointerup', up);
  node.addEventListener('pointercancel', cancelPointer);
  node.addEventListener('lostpointercapture', cancelPointer);
  node.addEventListener('contextmenu', contextMenu);
  window.addEventListener('blur', cancel);
  document.addEventListener('visibilitychange', cancel);
  return {
    update(next: typeof options) {
      if (next.key !== options.key) cancel();
      options = next;
    },
    destroy() {
      cancel();
      node.removeEventListener('pointerdown', down);
      node.removeEventListener('pointermove', move);
      node.removeEventListener('pointerup', up);
      node.removeEventListener('pointercancel', cancelPointer);
      node.removeEventListener('lostpointercapture', cancelPointer);
      node.removeEventListener('contextmenu', contextMenu);
      window.removeEventListener('blur', cancel);
      document.removeEventListener('visibilitychange', cancel);
    },
  };
}
