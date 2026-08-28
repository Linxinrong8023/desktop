import {
  diffFileScrollTop,
  isDiffFileAligned,
  isDiffScrollAtEnd,
} from "./task-diff-scroll";

/** Upper bound of layout retries before a jump stops fighting a viewport that never settles. */
export const DIFF_FILE_SCROLL_MAX_ATTEMPTS = 90;

/**
 * Input events that mean a person, not the jump, is moving the viewport. A
 * programmatic `scrollTo` never emits these, so any of them is an unambiguous
 * signal to hand the scroll position back to the user immediately.
 */
const USER_SCROLL_INTENT_EVENT_TYPES = [
  "wheel",
  "touchstart",
  "pointerdown",
] as const;

export interface DiffFileScrollRunOptions {
  /** Resolves the Changes scroll container; null before the panel has mounted. */
  getRoot: () => HTMLElement | null;
  /** Resolves the target file's wrapper element; undefined before React commits it. */
  getTarget: () => HTMLElement | undefined;
  /**
   * Called one frame after arrival, so the run's final programmatic scroll
   * event is delivered while the viewport is still held and cannot race the
   * scroll spy into re-selecting a neighbour of the target.
   */
  onArrived: () => void;
  /** Called immediately when user input, the attempt budget, or `cancel` ends the run. */
  onInterrupted: () => void;
}

export interface DiffFileScrollRunHandle {
  /**
   * Ends the run and releases the viewport. While still running it reports
   * `onInterrupted`; after arrival it only drops the pending arrival delivery.
   * Idempotent.
   */
  cancel: () => void;
}

/**
 * Owns one programmatic "scroll this file to the top of the Changes viewport"
 * jump — shared by file-tree clicks and chat-link jumps so exactly one run can
 * hold the viewport at a time.
 *
 * Off-screen files render as estimated-height placeholders, so a single
 * `scrollTo` lands on stale geometry: while the jump animates nothing but
 * re-aligns, the sections it passes mount their real (shorter) content and
 * shift the target. The run therefore re-scrolls every frame — plus on every
 * resize of the container or target — until the target actually sits at the
 * viewport top, or the container is clamped at its end (a short last file can
 * never satisfy the alignment inset).
 *
 * The run never fights the user: wheel / touch / pointer input on the
 * container interrupts it at once. `cancel` must be called on unmount or when
 * another run replaces this one.
 */
export function runDiffFileScroll(
  options: DiffFileScrollRunOptions,
): DiffFileScrollRunHandle {
  let state: "running" | "arrived" | "interrupted" = "running";
  let attempts = 0;
  let frame: number | null = null;
  let observer: ResizeObserver | null = null;
  let listenedRoot: HTMLElement | null = null;

  /** Releases every timer, observer, and listener the run is holding. */
  const detach = () => {
    if (frame !== null) {
      cancelAnimationFrame(frame);
      frame = null;
    }
    observer?.disconnect();
    observer = null;
    if (listenedRoot !== null) {
      for (const type of USER_SCROLL_INTENT_EVENT_TYPES) {
        listenedRoot.removeEventListener(type, onUserScrollIntent);
      }
      listenedRoot = null;
    }
  };

  /** Hands viewport control back to the page without waiting a frame. */
  const interrupt = () => {
    if (state !== "running") return;
    state = "interrupted";
    detach();
    options.onInterrupted();
  };

  const onUserScrollIntent = () => interrupt();

  /**
   * Ends the run as arrived, one frame late: the final programmatic
   * `scrollTo` still delivers a scroll event, and that event must be swallowed
   * while the viewport is still held instead of letting it move the selection.
   */
  const arrive = () => {
    if (state !== "running") return;
    state = "arrived";
    detach();
    frame = requestAnimationFrame(() => {
      frame = null;
      options.onArrived();
    });
  };

  /** Places the target near the container top; false while either side lacks layout. */
  const alignTarget = () => {
    const root = options.getRoot();
    const target = options.getTarget();
    if (root === null || target === undefined) return false;
    if (typeof root.scrollTo !== "function") return false;
    const top = diffFileScrollTop(root, target);
    if (top === null) return false;
    root.scrollTo({ top, behavior: "auto" });
    return true;
  };

  /** True when the target is where the jump wanted it, or as close as scrolling allows. */
  const hasArrived = () => {
    const root = options.getRoot();
    const target = options.getTarget();
    if (root === null || target === undefined) return false;
    return isDiffFileAligned(root, target) || isDiffScrollAtEnd(root);
  };

  const attempt = () => {
    if (state !== "running") return;
    frame = null;
    if (listenedRoot === null) {
      const root = options.getRoot();
      if (root !== null) {
        listenedRoot = root;
        for (const type of USER_SCROLL_INTENT_EVENT_TYPES) {
          root.addEventListener(type, onUserScrollIntent, { passive: true });
        }
      }
    }
    if (alignTarget() && hasArrived()) {
      arrive();
      return;
    }
    if (++attempts >= DIFF_FILE_SCROLL_MAX_ATTEMPTS) {
      interrupt();
      return;
    }
    frame = requestAnimationFrame(attempt);
  };

  attempt();

  // Placeholder sections around the target take their real height after the
  // first landing; each such resize is a fresh chance to re-align without
  // waiting for the next frame budget. Skipped when the first attempt already
  // arrived — detach has run by then and nothing would disconnect this observer.
  const observedRoot = state === "running" ? options.getRoot() : null;
  if (typeof ResizeObserver !== "undefined" && observedRoot !== null) {
    const resizeObserver = new ResizeObserver(() => attempt());
    observer = resizeObserver;
    // Observing can deliver the callback synchronously (jsdom polyfill) and
    // finish the run mid-setup, which nulls `observer`; guard each observation
    // so a finished run never touches its disconnected observer again.
    const observe = (element: Element) => {
      if (state !== "running") return;
      resizeObserver.observe(element);
    };
    observe(observedRoot);
    const content = observedRoot.firstElementChild;
    if (content instanceof Element) observe(content);
    const target = options.getTarget();
    if (target !== undefined) observe(target);
  }

  return {
    cancel: () => {
      if (state === "running") {
        interrupt();
        return;
      }
      // An arrived run may still owe its one-frame delivery; the owner only
      // cancels after arrival when it is replacing or unmounting the run, so
      // dropping the delivery is the right end state.
      if (frame !== null) {
        cancelAnimationFrame(frame);
        frame = null;
      }
    },
  };
}
