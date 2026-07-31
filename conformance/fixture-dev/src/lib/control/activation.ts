export type Activation = () => void | Promise<void>;

export interface PointerActivation {
  pointerId: number;
}

export function beginPointerActivation(event: PointerEvent): PointerActivation | undefined {
  if (!event.isPrimary || event.button !== 0) return undefined;
  event.preventDefault();
  return { pointerId: event.pointerId };
}

export function completePointerActivation(
  event: PointerEvent,
  pending: PointerActivation | undefined,
  action: Activation
) {
  if (!pending || !event.isPrimary || event.pointerId !== pending.pointerId || event.button !== 0) return;
  event.preventDefault();
  void action();
}

export function activateFromKeyboardOrAssistiveClick(event: MouseEvent, action: Activation) {
  if (event.detail !== 0) return;
  void action();
}
