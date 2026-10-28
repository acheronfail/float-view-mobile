import { BOX_HOLD_MS } from './hold-to-configure';

type Options = { key: string | null; disabled: boolean; onedit: () => void };

/** Hold empty dashboard space while leaving native scrolling and child controls intact. */
export function dashboardBackgroundGesture(node: HTMLElement, initial: Options) {
  let options = initial;
  let pointer: { id: number; x: number; y: number } | null = null;
  let timer: ReturnType<typeof setTimeout> | undefined;
  const cancel = () => {
    clearTimeout(timer);
    pointer = null;
  };
  const down = (event: PointerEvent) => {
    cancel();
    if (
      options.disabled ||
      !event.isPrimary ||
      event.button !== 0 ||
      (event.target as Element).closest('[data-dashboard-widget], button, a, input, select, textarea')
    )
      return;
    pointer = { id: event.pointerId, x: event.clientX, y: event.clientY };
    timer = setTimeout(() => {
      cancel();
      options.onedit();
    }, BOX_HOLD_MS);
  };
  const move = (event: PointerEvent) => {
    if (pointer?.id === event.pointerId && Math.hypot(event.clientX - pointer.x, event.clientY - pointer.y) > 10)
      cancel();
  };
  const end = (event: PointerEvent) => {
    if (pointer?.id === event.pointerId) cancel();
  };
  const contextMenu = (event: Event) => {
    if (!(event.target as Element).closest('[data-dashboard-widget], button, a, input, select, textarea'))
      event.preventDefault();
  };
  node.addEventListener('pointerdown', down);
  window.addEventListener('pointermove', move);
  window.addEventListener('pointerup', end);
  window.addEventListener('pointercancel', end);
  node.addEventListener('lostpointercapture', end);
  node.addEventListener('contextmenu', contextMenu);
  node.addEventListener('scroll', cancel);
  window.addEventListener('blur', cancel);
  window.addEventListener('resize', cancel);
  document.addEventListener('visibilitychange', cancel);
  return {
    update(next: Options) {
      if (next.key !== options.key || next.disabled) cancel();
      options = next;
    },
    destroy() {
      cancel();
      node.removeEventListener('pointerdown', down);
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', end);
      window.removeEventListener('pointercancel', end);
      node.removeEventListener('lostpointercapture', end);
      node.removeEventListener('contextmenu', contextMenu);
      node.removeEventListener('scroll', cancel);
      window.removeEventListener('blur', cancel);
      window.removeEventListener('resize', cancel);
      document.removeEventListener('visibilitychange', cancel);
    },
  };
}
