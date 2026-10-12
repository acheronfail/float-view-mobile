export type Activation = () => void | Promise<void>;

export function isPrimaryPointerActivation(event: Pick<PointerEvent, 'button' | 'isPrimary'>): boolean {
  return event.isPrimary && event.button === 0;
}

export function isKeyboardOrAssistiveClick(event: Pick<MouseEvent, 'detail'>): boolean {
  return event.detail === 0;
}

export function activateFromPointer(
  event: PointerEvent,
  action: Activation,
  { scrollable = false }: { scrollable?: boolean } = {}
): void {
  if (!isPrimaryPointerActivation(event)) return;
  if (!scrollable) event.preventDefault();
  const target = event.currentTarget;
  if (!(target instanceof EventTarget)) {
    void action();
    return;
  }
  const pointerId = event.pointerId;
  let settled = false;
  const cleanup = () => {
    target.removeEventListener('pointerup', finish);
    target.removeEventListener('pointercancel', cancel);
    target.removeEventListener('lostpointercapture', cancel);
    target.removeEventListener('pointermove', move);
  };
  const cancel = (next: Event) => {
    if ('pointerId' in next && next.pointerId !== pointerId) return;
    settled = true;
    cleanup();
  };
  const finish = (next: Event) => {
    if (settled || !('pointerId' in next) || next.pointerId !== pointerId || ('isPrimary' in next && !next.isPrimary)) {
      return;
    }
    if (moved(next)) {
      cancel(next);
      return;
    }
    settled = true;
    cleanup();
    void action();
  };
  const moved = (next: Event) =>
    scrollable &&
    'clientX' in next &&
    'clientY' in next &&
    Math.hypot(Number(next.clientX) - event.clientX, Number(next.clientY) - event.clientY) > 10;
  const move = (next: Event) => {
    if ('pointerId' in next && next.pointerId === pointerId && moved(next)) cancel(next);
  };
  target.addEventListener('pointerup', finish);
  target.addEventListener('pointercancel', cancel);
  target.addEventListener('lostpointercapture', cancel);
  if (scrollable) target.addEventListener('pointermove', move);
  if ('setPointerCapture' in target && typeof target.setPointerCapture === 'function') {
    target.setPointerCapture(pointerId);
  }
}

export function activateFromKeyboardOrAssistiveClick(event: MouseEvent, action: Activation): void {
  if (!isKeyboardOrAssistiveClick(event)) return;
  void action();
}
