export type Activation = () => void | Promise<void>;

export function isPrimaryPointerActivation(event: Pick<PointerEvent, 'button' | 'isPrimary'>): boolean {
  return event.isPrimary && event.button === 0;
}

export function isKeyboardOrAssistiveClick(event: Pick<MouseEvent, 'detail'>): boolean {
  return event.detail === 0;
}

export function activateFromPointer(event: PointerEvent, action: Activation): void {
  if (!isPrimaryPointerActivation(event)) return;
  event.preventDefault();
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
    settled = true;
    cleanup();
    void action();
  };
  target.addEventListener('pointerup', finish);
  target.addEventListener('pointercancel', cancel);
  target.addEventListener('lostpointercapture', cancel);
  if ('setPointerCapture' in target && typeof target.setPointerCapture === 'function') {
    target.setPointerCapture(pointerId);
  }
}

export function activateFromKeyboardOrAssistiveClick(event: MouseEvent, action: Activation): void {
  if (!isKeyboardOrAssistiveClick(event)) return;
  void action();
}
