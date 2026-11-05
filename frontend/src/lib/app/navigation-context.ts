import type { SessionSummary } from '$lib/backend/contract';

export const NAVIGATION_CONTEXT = Symbol('navigation-context');

/** Ephemeral LRU storage; dropping an entry always falls back to ordinary navigation. */
export class NavigationCache<T> {
  private entries = new Map<string, T>();
  constructor(private readonly limit: number) {}
  get(key: string): T | undefined {
    const value = this.entries.get(key);
    if (value !== undefined) this.set(key, value);
    return value;
  }
  set(key: string, value: T) {
    this.entries.delete(key);
    this.entries.set(key, value);
    if (this.entries.size > this.limit) this.entries.delete(this.entries.keys().next().value!);
  }
  delete(key: string) {
    this.entries.delete(key);
  }
}

type Anchor = { href: string; top: number };
type Position = { y: number; anchors: Anchor[]; focus: string | null };
export type RetainedSessions = {
  items: SessionSummary[];
  catalogRevision: number;
  nextCursor: string | null;
  targetCount: number;
};

export class NavigationContext {
  readonly sessions = new NavigationCache<RetainedSessions>(8);
  readonly maxRetainedSessions = 4000;
  private positions = new NavigationCache<Position>(32);
  private stopRestore: (() => void) | undefined;

  retainSessions(board: string, value: RetainedSessions) {
    if (value.items.length > this.maxRetainedSessions) this.sessions.delete(board);
    else this.sessions.set(board, value);
  }

  has(url: URL) {
    const key = this.key(url);
    const board = key.match(/^\/sessions\/boards\/([^?]+)/)?.[1];
    if (board) {
      try {
        if (this.sessions.get(decodeURIComponent(board))) return this.positions.get(key) !== undefined;
      } catch {
        // Malformed route parameters have no retained browsing context.
      }
      this.positions.delete(key);
      return false;
    }
    return this.positions.get(key) !== undefined;
  }

  private key(url: URL) {
    // Route and its query live inside the first hash; a second hash is an in-page anchor.
    return url.hash.startsWith('#/') ? url.hash.slice(1).split('#')[0]! : url.pathname + url.search;
  }

  capture(url: URL, content: HTMLElement) {
    // Do not replace a retained position with an unfinished restoration.
    if (this.stopRestore) return;
    const top = document.querySelector('header')?.getBoundingClientRect().bottom ?? 0;
    const anchors = Array.from(content.querySelectorAll<HTMLAnchorElement>('li a[href]'))
      .filter((node) => node.getBoundingClientRect().bottom > top)
      .slice(0, 6)
      .map((node) => ({ href: node.getAttribute('href')!, top: node.getBoundingClientRect().top }));
    const active = document.activeElement;
    this.positions.set(this.key(url), {
      y: window.scrollY,
      anchors,
      focus:
        document.documentElement.dataset.keyboardNavigation &&
        active instanceof HTMLAnchorElement &&
        content.contains(active)
          ? active.getAttribute('href')
          : null,
    });
  }

  cancel() {
    this.stopRestore?.();
  }

  restore(url: URL, content: HTMLElement, reveal: () => void) {
    this.cancel();
    const position = this.has(url) ? this.positions.get(this.key(url)) : undefined;
    if (!position) {
      window.scrollTo({ top: 0, behavior: 'instant' });
      reveal();
      return;
    }
    let frame = 0;
    const finish = () => {
      cancelAnimationFrame(frame);
      clearTimeout(timeout);
      observer.disconnect();
      for (const event of ['pointerdown', 'wheel', 'keydown', 'touchstart'])
        window.removeEventListener(event, finish, true);
      this.stopRestore = undefined;
      // Reveal synchronously so restored keyboard focus can target the content.
      content.style.visibility = '';
      reveal();
    };
    const apply = () => {
      frame = 0;
      if (content.querySelector('[data-navigation-pending="true"]')) return;
      const links = Array.from(content.querySelectorAll<HTMLAnchorElement>('a[href]'));
      const anchor = position.anchors
        .map((saved) => ({ saved, node: links.find((node) => node.getAttribute('href') === saved.href) }))
        .find((anchor) => anchor.node);
      window.scrollTo({
        top: anchor ? window.scrollY + anchor.node!.getBoundingClientRect().top - anchor.saved.top : position.y,
        behavior: 'instant',
      });
      finish();
      if (position.focus && document.documentElement.dataset.keyboardNavigation)
        links.find((node) => node.getAttribute('href') === position.focus)?.focus({ preventScroll: true });
    };
    const schedule = () => {
      if (!frame) frame = requestAnimationFrame(apply);
    };
    const observer = new MutationObserver(schedule);
    observer.observe(content, {
      childList: true,
      subtree: true,
      attributes: true,
      attributeFilter: ['data-navigation-pending'],
    });
    const timeout = setTimeout(finish, 8000);
    for (const event of ['pointerdown', 'wheel', 'keydown', 'touchstart'])
      window.addEventListener(event, finish, { capture: true, passive: true });
    this.stopRestore = finish;
    schedule();
  }
}
