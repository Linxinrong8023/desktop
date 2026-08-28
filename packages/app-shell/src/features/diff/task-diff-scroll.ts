/** Keeps the file heading slightly below the viewport top. */
export const DIFF_FILE_SCROLL_INSET_PX = 16;

/**
 * Computes the scrollTop that places `element` near the top of `root`.
 * Chat links remount Changes inside a panel that is often still 0-height on
 * the first layout; returning a number in that state would "succeed" at 0 and
 * leave the first file on screen. Callers must retry until this returns a value.
 *
 * A destination at or below the container top is a legitimate result (jumping
 * to a file near the list start) and clamps to 0 — callers verify the landing
 * and retry, so no "unlaid-out" heuristic is applied here. One used to reject
 * `top <= 0` whenever `offsetTop > 0`, which permanently killed every jump
 * whose true destination was the list start (offsetTop counts from the nearest
 * positioned ancestor, which sits above the scroll container).
 */
export function diffFileScrollTop(
  root: HTMLElement,
  element: HTMLElement,
): number | null {
  if (root.clientHeight <= 0 || element.offsetHeight <= 0) return null;
  const top =
    element.getBoundingClientRect().top -
    root.getBoundingClientRect().top +
    root.scrollTop -
    DIFF_FILE_SCROLL_INSET_PX;
  if (!Number.isFinite(top)) return null;
  return Math.max(0, top);
}

/**
 * Computes the scrollTop that vertically centers `element` inside `root`,
 * leaving the horizontal scroll position untouched. A line jump must never
 * shift wide code sideways, so callers persist `scrollLeft` while centering.
 */
export function diffLineScrollTop(
  root: HTMLElement,
  element: HTMLElement,
): number | null {
  if (root.clientHeight <= 0 || element.offsetHeight <= 0) return null;
  const offset =
    element.getBoundingClientRect().top -
    root.getBoundingClientRect().top +
    root.scrollTop;
  const top = offset - (root.clientHeight - element.offsetHeight) / 2;
  if (!Number.isFinite(top)) return null;
  return Math.max(0, top);
}

const DIFF_FILE_ALIGN_TOLERANCE_PX = 64;

/** Slack for the bottom clamp so "cannot scroll further" beats pixel rounding. */
const DIFF_SCROLL_END_TOLERANCE_PX = 2;

/**
 * True when `element` is actually sitting near the top of `root`.
 * Calling scrollTo is not enough: virtualized placeholders above the file can
 * shrink after the first jump and leave the requested section off-screen.
 */
export function isDiffFileAligned(
  root: HTMLElement,
  element: HTMLElement,
): boolean {
  if (root.clientHeight <= 0 || element.offsetHeight <= 0) return false;
  const offset =
    element.getBoundingClientRect().top - root.getBoundingClientRect().top;
  if (!Number.isFinite(offset)) return false;
  return (
    Math.abs(offset - DIFF_FILE_SCROLL_INSET_PX) <= DIFF_FILE_ALIGN_TOLERANCE_PX
  );
}

/**
 * True when the container is scrolled to (or clamped at) its end.
 * A file near the list end can never satisfy the alignment inset because
 * scrollTop clamps before the header reaches the viewport top, so the clamped
 * position is as close as a jump can get and counts as arrived.
 */
export function isDiffScrollAtEnd(root: HTMLElement): boolean {
  return (
    root.scrollHeight - root.scrollTop - root.clientHeight <=
    DIFF_SCROLL_END_TOLERANCE_PX
  );
}
