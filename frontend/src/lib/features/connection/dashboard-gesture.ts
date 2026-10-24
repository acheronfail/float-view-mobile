import { BOX_HOLD_MS } from './hold-to-configure';

const EDIT_HOLD_MS = 300;

type Options = {
  id: string;
  scroll: HTMLElement | undefined;
  editing: boolean;
  disabled: boolean;
  onedit: () => void;
  onconfigure: () => void;
  onmove: (id: string, index: number) => void;
  ondrag?: (active: boolean) => void;
};
/** Whole-card arbitration: hold enters editing; early movement scrolls without activating controls. */
export function dashboardGesture(node: HTMLElement, initial: Options) {
  let options = initial;
  let pointer: {
    id: number;
    x: number;
    y: number;
    lastY: number;
    lastX: number;
    editing: boolean;
    control: HTMLButtonElement | null;
    touch: boolean;
    mode: 'pending' | 'scrolling' | 'dragging';
  } | null = null;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let frame: number | undefined;
  let lifted: HTMLElement | null = null;
  let placement: HTMLElement[] = [];
  const animations = new Map<HTMLElement, Animation>();
  let slots = new Map<HTMLElement, DOMRect>();
  let slotScrollTop = 0;
  function preview(x: number, y: number) {
    const scrollBounds = options.scroll?.getBoundingClientRect();
    const inside =
      scrollBounds &&
      x >= scrollBounds.left &&
      x <= scrollBounds.right &&
      y >= scrollBounds.top &&
      y <= scrollBounds.bottom;
    const scrollDelta = slotScrollTop - (options.scroll?.scrollTop ?? 0);
    // Hit-test final slots, not neighbours still animating past the pointer.
    const next = slots.size
      ? ([...slots].find(
          ([, rect]) =>
            inside && x >= rect.left && x <= rect.right && y >= rect.top + scrollDelta && y <= rect.bottom + scrollDelta
        )?.[0] ?? null)
      : (document.elementFromPoint(x, y)?.closest<HTMLElement>('[data-dashboard-widget]') ?? null);
    const target = next && options.scroll?.contains(next) ? next : null;
    if (!target) return;
    const from = placement.indexOf(node);
    const to = placement.indexOf(target);
    if (from < 0 || to < 0 || from === to) return;
    const before = new Map(placement.map((card) => [card, card.getBoundingClientRect()]));
    placement.splice(from, 1);
    placement.splice(to, 0, node);
    // Preview order in CSS, leaving keyed DOM and pointer capture intact until drop.
    placement.forEach((card, index) => {
      card.style.order = String(index);
    });
    animations.forEach((animation) => animation.cancel());
    slots = new Map(placement.map((card) => [card, card.getBoundingClientRect()]));
    slotScrollTop = options.scroll?.scrollTop ?? 0;
    for (const card of placement) {
      if (card === node || matchMedia('(prefers-reduced-motion: reduce)').matches) continue;
      const old = before.get(card)!;
      const bounds = slots.get(card)!;
      animations.set(
        card,
        card.animate(
          [
            { transform: `translate(${old.left - bounds.left}px, ${old.top - bounds.top}px)` },
            { transform: 'translate(0, 0)' },
          ],
          { duration: 180, easing: 'ease-out' }
        )
      );
    }
  }
  function cancel() {
    clearTimeout(timer);
    if (frame !== undefined) cancelAnimationFrame(frame);
    frame = undefined;
    const id = pointer?.id;
    pointer = null;
    if (id !== undefined && node.hasPointerCapture?.(id)) node.releasePointerCapture(id);
    animations.forEach((animation) => animation.cancel());
    animations.clear();
    placement.forEach((card) => card.style.removeProperty('order'));
    placement = [];
    slots.clear();
    lifted?.remove();
    lifted = null;
    delete node.dataset.pressing;
    delete node.dataset.dragging;
    delete node.dataset.drop;
    options.ondrag?.(false);
  }
  function edgeScroll() {
    if (!pointer || pointer.mode !== 'dragging' || !options.scroll) return;
    const bounds = options.scroll.getBoundingClientRect();
    const y = pointer.lastY;
    const delta = y < bounds.top + 48 ? -8 : y > bounds.bottom - 48 ? 8 : 0;
    options.scroll.scrollTop += delta;
    if (delta) preview(pointer.lastX, pointer.lastY);
    frame = requestAnimationFrame(edgeScroll);
  }
  function arm() {
    if (!pointer || pointer.mode !== 'pending') return;
    delete node.dataset.pressing;
    options.onedit();
    pointer.mode = 'dragging';
    const bounds = node.getBoundingClientRect();
    lifted = node.cloneNode(true) as HTMLElement;
    lifted.inert = true;
    lifted.setAttribute('aria-hidden', 'true');
    lifted.removeAttribute('data-dashboard-widget');
    lifted.querySelectorAll('[data-dashboard-remove]').forEach((el) => el.remove());
    lifted.querySelectorAll('[id]').forEach((el) => el.removeAttribute('id'));
    const reducedMotion = matchMedia('(prefers-reduced-motion: reduce)').matches;
    lifted.dataset.dashboardLifted = 'true';
    lifted.style.cssText = `position:fixed;left:${bounds.left}px;top:${bounds.top}px;width:${bounds.width}px;height:${bounds.height}px;pointer-events:none;z-index:100;box-sizing:border-box;animation:none;--widget-wiggle:none;rotate:0deg;scale:${reducedMotion ? 1 : 1.035};opacity:1;outline:2px solid var(--color-accent);outline-offset:1px;box-shadow:0 16px 36px #0005,0 0 0 3px color-mix(in srgb,var(--color-accent) 18%,transparent);`;
    document.body.append(lifted);
    if (!reducedMotion) lifted.animate([{ scale: 0.97 }, { scale: 1.035 }], { duration: 150, easing: 'ease-out' });
    placement = [...(node.parentElement?.querySelectorAll<HTMLElement>('[data-dashboard-widget]') ?? [])];
    node.dataset.dragging = 'true';
    node.dataset.drop = 'true';
    options.ondrag?.(true);
    edgeScroll();
  }
  const down = (e: PointerEvent) => {
    if (options.disabled || !options.scroll) return;
    if (!e.isPrimary || e.button !== 0) {
      cancel();
      return;
    }
    if ((e.target as Element).closest('[data-dashboard-remove]')) {
      cancel();
      return;
    }
    // Own the stream before a nested lighting button can register its activation.
    e.stopPropagation();
    cancel();
    pointer = {
      id: e.pointerId,
      x: e.clientX,
      y: e.clientY,
      lastY: e.clientY,
      lastX: e.clientX,
      editing: options.editing,
      control: (e.target as Element).closest<HTMLButtonElement>('button'),
      touch: e.pointerType === 'touch',
      mode: 'pending',
    };
    node.setPointerCapture?.(e.pointerId);
    node.dataset.pressing = 'true';
    timer = setTimeout(arm, pointer.editing ? EDIT_HOLD_MS : BOX_HOLD_MS);
  };
  const move = (e: PointerEvent) => {
    if (!pointer || pointer.id !== e.pointerId) return;
    const distance = Math.hypot(e.clientX - pointer.x, e.clientY - pointer.y);
    if (pointer.mode === 'pending' && distance > 10) {
      clearTimeout(timer);
      delete node.dataset.pressing;
      if (pointer.touch || !pointer.editing) pointer.mode = 'scrolling';
      else arm();
    }
    if (pointer.mode === 'scrolling' && options.scroll) options.scroll.scrollTop += pointer.lastY - e.clientY;
    pointer.lastY = e.clientY;
    pointer.lastX = e.clientX;
    if (pointer.mode === 'dragging') {
      if (lifted) lifted.style.transform = `translate(${e.clientX - pointer.x}px, ${e.clientY - pointer.y}px)`;
      preview(e.clientX, e.clientY);
    }
  };
  const up = (e: PointerEvent) => {
    if (!pointer || pointer.id !== e.pointerId) return;
    const tap = pointer.mode === 'pending' && Math.hypot(e.clientX - pointer.x, e.clientY - pointer.y) <= 10;
    const index = pointer.mode === 'dragging' ? placement.indexOf(node) : null;
    const id = options.id;
    const editing = pointer.editing;
    const control = pointer.control;
    cancel();
    if (index !== null && Number.isInteger(index)) options.onmove(id, index);
    else if (tap && editing) options.onconfigure();
    else if (tap && control && node.contains(control)) control.click();
  };
  const pointerCancel = (e: PointerEvent) => {
    if (pointer?.id === e.pointerId) cancel();
  };
  const contextMenu = (e: Event) => e.preventDefault();
  node.addEventListener('pointerdown', down, true);
  node.addEventListener('pointermove', move);
  node.addEventListener('pointerup', up);
  node.addEventListener('pointercancel', pointerCancel);
  node.addEventListener('lostpointercapture', pointerCancel);
  node.addEventListener('contextmenu', contextMenu);
  window.addEventListener('blur', cancel);
  document.addEventListener('visibilitychange', cancel);
  // Entering edit mode changes toolbar height; only viewport changes cancel the lift.
  window.addEventListener('resize', cancel);
  let width = node.parentElement?.clientWidth;
  const observer = new ResizeObserver(() => {
    const nextWidth = node.parentElement?.clientWidth;
    if (nextWidth !== width) cancel();
    width = nextWidth;
  });
  if (node.parentElement) observer.observe(node.parentElement);
  return {
    update(next: Options) {
      if (
        next.id !== options.id ||
        next.scroll !== options.scroll ||
        next.disabled ||
        (options.editing && !next.editing)
      )
        cancel();
      options = next;
    },
    destroy() {
      cancel();
      observer.disconnect();
      node.removeEventListener('pointerdown', down, true);
      node.removeEventListener('pointermove', move);
      node.removeEventListener('pointerup', up);
      node.removeEventListener('pointercancel', pointerCancel);
      node.removeEventListener('lostpointercapture', pointerCancel);
      node.removeEventListener('contextmenu', contextMenu);
      window.removeEventListener('blur', cancel);
      window.removeEventListener('resize', cancel);
      document.removeEventListener('visibilitychange', cancel);
    },
  };
}
