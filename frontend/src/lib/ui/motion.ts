import { getContext } from 'svelte';
import { cubicOut } from 'svelte/easing';
import { fade, fly } from 'svelte/transition';
import type { TransitionConfig } from 'svelte/transition';

export const MOTION_CONTEXT = Symbol('motion');
export type MotionContext = { enabled(): boolean };
type Surface = 'panel' | 'dialog' | 'menu' | 'content';

function duration(node: HTMLElement, token: 'fast' | 'normal' | 'panel'): number {
  if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return 0;
  // The CSS optimizer may serialize 220ms as .22s.
  const value = getComputedStyle(node).getPropertyValue(`--motion-${token}`).trim();
  return (parseFloat(value) || 0) * (value.endsWith('ms') ? 1 : 1000);
}

// Height changes are reserved for explicit disclosure controls.
function disclosure(node: HTMLElement, enabled: boolean): TransitionConfig {
  const height = node.getBoundingClientRect().height;
  const style = getComputedStyle(node);
  const top = parseFloat(style.paddingTop) || 0;
  const bottom = parseFloat(style.paddingBottom) || 0;
  return {
    duration: enabled ? duration(node, 'normal') : 0,
    easing: cubicOut,
    css: (t) =>
      `height: ${t * height}px; padding-top: ${t * top}px; padding-bottom: ${t * bottom}px; opacity: ${t}; overflow: hidden;`,
  };
}

// Read context during component initialization, never from a transition callback.
export function createMotion() {
  const context = getContext<MotionContext | undefined>(MOTION_CONTEXT);
  const enabled = () => context?.enabled() ?? true;
  const running = new WeakMap<HTMLElement, Animation>();
  const leaving = new WeakMap<HTMLElement, { hidden: string | null; modal: string | null }>();
  function restore(node: HTMLElement) {
    const attributes = leaving.get(node);
    if (!attributes) return;
    node.inert = false;
    for (const [name, value] of [
      ['aria-hidden', attributes.hidden],
      ['aria-modal', attributes.modal],
    ] as const) {
      if (value === null) node.removeAttribute(name);
      else node.setAttribute(name, value);
    }
    node.style.pointerEvents = '';
    leaving.delete(node);
  }
  function disable(node: HTMLElement) {
    if (!leaving.has(node))
      leaving.set(node, { hidden: node.getAttribute('aria-hidden'), modal: node.getAttribute('aria-modal') });
    node.inert = true;
    node.setAttribute('aria-hidden', 'true');
    node.removeAttribute('aria-modal');
    node.style.pointerEvents = 'none';
  }

  function enter(node: HTMLElement, kind: Surface) {
    running.get(node)?.cancel();
    const ms = enabled() ? duration(node, kind === 'panel' ? 'panel' : kind === 'dialog' ? 'normal' : 'fast') : 0;
    if (!ms) return;
    const offset = kind === 'panel' ? 8 : kind === 'dialog' ? 6 : kind === 'menu' ? -4 : 0;
    const animation = node.animate(
      kind === 'content'
        ? [{ opacity: 0 }, { opacity: 1 }]
        : [
            { opacity: 0, transform: `translateY(${offset}px)` },
            { opacity: 1, transform: 'translateY(0)' },
          ],
      { duration: ms, easing: getComputedStyle(node).getPropertyValue('--motion-ease').trim() }
    );
    running.set(node, animation);
  }

  return {
    appear(node: HTMLElement, kind: Surface): TransitionConfig {
      restore(node);
      const ms = enabled() ? duration(node, kind === 'panel' ? 'panel' : kind === 'dialog' ? 'normal' : 'fast') : 0;
      if (kind === 'content') return fade(node, { duration: ms, easing: cubicOut });
      return fly(node, {
        y: kind === 'panel' ? 8 : kind === 'dialog' ? 6 : kind === 'menu' ? -4 : 0,
        duration: ms,
        easing: cubicOut,
      });
    },
    surface(node: HTMLElement, kind: Surface) {
      // Native dialogs are mounted while closed. Animate only when they open.
      const nativeDialog = node instanceof HTMLDialogElement;
      if (!nativeDialog || node.open) enter(node, kind);
      const observer = nativeDialog
        ? new MutationObserver(() => {
            if (node.open) enter(node, kind);
            else running.get(node)?.cancel();
          })
        : undefined;
      observer?.observe(node, { attributes: true, attributeFilter: ['open'] });
      return {
        destroy() {
          observer?.disconnect();
          running.get(node)?.cancel();
        },
      };
    },
    change(node: HTMLElement, key: string) {
      let previous = key;
      return {
        update(next: string) {
          if (next === previous) return;
          previous = next;
          enter(node, 'content');
        },
        destroy() {
          running.get(node)?.cancel();
        },
      };
    },
    exit(node: HTMLElement): TransitionConfig {
      running.get(node)?.cancel();
      // Outgoing DOM is visual only. Focus and actions transfer immediately,
      // including when a menu is replaced by a confirmation or another panel.
      disable(node);
      return fade(node, { duration: enabled() ? duration(node, 'fast') : 0, easing: cubicOut });
    },
    expand(node: HTMLElement): TransitionConfig {
      restore(node);
      return disclosure(node, enabled());
    },
    collapse(node: HTMLElement): TransitionConfig {
      disable(node);
      return disclosure(node, enabled());
    },
    discover(node: HTMLElement): TransitionConfig {
      return fly(node, { y: 0, duration: enabled() ? duration(node, 'fast') : 0, easing: cubicOut });
    },
  };
}
