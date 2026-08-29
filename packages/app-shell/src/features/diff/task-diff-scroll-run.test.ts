import { afterEach, describe, expect, it, vi } from "vitest";
import {
  DIFF_FILE_SCROLL_MAX_ATTEMPTS,
  runDiffFileScroll,
  type DiffFileScrollRunHandle,
} from "./task-diff-scroll-run";

interface ScrollFixtureGeometry {
  rootClientHeight: number;
  rootScrollHeight: number;
  targetOffsetHeight: number;
  targetOffsetTop: number;
}

interface ScrollFixture {
  root: HTMLElement;
  target: HTMLElement;
  geometry: ScrollFixtureGeometry;
  scrollTo: ReturnType<typeof vi.fn>;
}

/**
 * Builds a scroll container and target whose geometry the test controls
 * directly; the target's viewport position follows `offsetTop - scrollTop`
 * like a statically laid out document.
 */
function createScrollFixture(options?: {
  onFirstScroll?: () => void;
}): ScrollFixture {
  const geometry: ScrollFixtureGeometry = {
    rootClientHeight: 400,
    rootScrollHeight: 5000,
    targetOffsetHeight: 120,
    targetOffsetTop: 3000,
  };
  let scrollTop = 0;
  let scrolled = false;
  const root = document.createElement("div");
  const target = document.createElement("div");
  root.append(target);
  const scrollTo = vi.fn((arg?: ScrollToOptions | number) => {
    const requested = typeof arg === "number" ? arg : (arg?.top ?? 0);
    scrollTop = Math.max(
      0,
      Math.min(
        requested,
        geometry.rootScrollHeight - geometry.rootClientHeight,
      ),
    );
    if (!scrolled) {
      scrolled = true;
      options?.onFirstScroll?.();
    }
  });
  Object.defineProperty(root, "clientHeight", {
    configurable: true,
    get: () => geometry.rootClientHeight,
  });
  Object.defineProperty(root, "scrollHeight", {
    configurable: true,
    get: () => geometry.rootScrollHeight,
  });
  Object.defineProperty(root, "scrollTop", {
    configurable: true,
    get: () => scrollTop,
  });
  root.scrollTo = scrollTo as unknown as HTMLElement["scrollTo"];
  root.getBoundingClientRect = () => boxRect(0, geometry.rootClientHeight);
  Object.defineProperty(target, "offsetHeight", {
    configurable: true,
    get: () => geometry.targetOffsetHeight,
  });
  Object.defineProperty(target, "offsetTop", {
    configurable: true,
    get: () => geometry.targetOffsetTop,
  });
  target.getBoundingClientRect = () =>
    boxRect(geometry.targetOffsetTop - scrollTop, geometry.targetOffsetHeight);
  return { root, target, geometry, scrollTo };
}

/** Axis-aligned box at `top` with the given size. */
function boxRect(top: number, height: number): DOMRect {
  return {
    x: 0,
    y: top,
    top,
    left: 0,
    right: 800,
    bottom: top + height,
    width: 800,
    height,
    toJSON() {
      return {};
    },
  } as DOMRect;
}

/** Lets the requested number of animation frames elapse. */
async function settleFrames(count: number): Promise<void> {
  for (let i = 0; i < count; i++) {
    await new Promise<void>((resolve) => {
      requestAnimationFrame(() => resolve());
    });
  }
}

interface RunSpy {
  spy: { arrived: number; interrupted: number };
  handle: DiffFileScrollRunHandle;
}

/** Starts a run against `fixture` while counting its terminal callbacks. */
function startSpiedRun(
  fixture: ScrollFixture,
  getTarget: () => HTMLElement | undefined = () => fixture.target,
): RunSpy {
  // The counters live in a nested object on purpose: a spread would snapshot
  // the numbers instead of sharing them with the callbacks.
  const spy = { arrived: 0, interrupted: 0 };
  const handle = runDiffFileScroll({
    getRoot: () => fixture.root,
    getTarget,
    onArrived: () => {
      spy.arrived += 1;
    },
    onInterrupted: () => {
      spy.interrupted += 1;
    },
  });
  return { spy, handle };
}

describe("runDiffFileScroll", () => {
  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("scrolls the target to the alignment inset and reports arrival once", async () => {
    const fixture = createScrollFixture();

    const run = startSpiedRun(fixture);
    await settleFrames(4);

    expect(fixture.scrollTo).toHaveBeenCalledTimes(1);
    expect(fixture.scrollTo).toHaveBeenCalledWith({
      top: 2984,
      behavior: "auto",
    });
    expect(run.spy.arrived).toBe(1);
    expect(run.spy.interrupted).toBe(0);
  });

  it("re-aligns when late layout shifts the target after the first landing", async () => {
    // A placeholder section between the container top and the target takes its
    // real height during the first landing, moving the target down 500px.
    const fixture = createScrollFixture({
      onFirstScroll: () => {
        fixture.geometry.targetOffsetTop = 3500;
      },
    });

    const run = startSpiedRun(fixture);
    await settleFrames(6);

    expect(fixture.scrollTo).toHaveBeenCalledTimes(2);
    expect(fixture.scrollTo).toHaveBeenLastCalledWith({
      top: 3484,
      behavior: "auto",
    });
    expect(run.spy.arrived).toBe(1);
    expect(run.spy.interrupted).toBe(0);
  });

  it("reports arrival when the bottom clamp prevents further scrolling", async () => {
    const fixture = createScrollFixture();
    // All content fits the viewport, so the alignment inset is unreachable and
    // the stub clamps the requested scrollTop to 0.
    fixture.geometry.rootScrollHeight = 400;

    const run = startSpiedRun(fixture);
    await settleFrames(4);

    expect(fixture.scrollTo).toHaveBeenCalledWith({
      top: 2984,
      behavior: "auto",
    });
    expect(run.spy.arrived).toBe(1);
    expect(run.spy.interrupted).toBe(0);
  });

  it("interrupts when the user scrolls while the run is in flight", async () => {
    const fixture = createScrollFixture();
    // No viewport height: the run keeps retrying without ever landing.
    fixture.geometry.rootClientHeight = 0;

    const run = startSpiedRun(fixture);
    fixture.root.dispatchEvent(new Event("wheel"));
    await settleFrames(4);

    expect(fixture.scrollTo).not.toHaveBeenCalled();
    expect(run.spy.interrupted).toBe(1);
    expect(run.spy.arrived).toBe(0);
  });

  it("interrupts when pointer input lands in the container", async () => {
    const fixture = createScrollFixture();
    fixture.geometry.rootClientHeight = 0;

    const run = startSpiedRun(fixture);
    fixture.root.dispatchEvent(new Event("pointerdown"));
    await settleFrames(4);

    expect(run.spy.interrupted).toBe(1);
    expect(run.spy.arrived).toBe(0);
  });

  it("gives up after the attempt budget when the viewport never lays out", async () => {
    vi.useFakeTimers({
      toFake: ["requestAnimationFrame", "cancelAnimationFrame"],
    });
    const fixture = createScrollFixture();
    fixture.geometry.rootClientHeight = 0;

    const run = startSpiedRun(fixture);
    // The first attempt runs synchronously; the rest need one timer tick each.
    for (
      let i = 0;
      i < DIFF_FILE_SCROLL_MAX_ATTEMPTS && run.spy.interrupted === 0;
      i++
    ) {
      vi.advanceTimersByTime(16);
    }

    expect(run.spy.interrupted).toBe(1);
    expect(run.spy.arrived).toBe(0);
  });

  it("cancel ends a running run without further scrolling or arrival", async () => {
    const fixture = createScrollFixture();
    fixture.geometry.rootClientHeight = 0;

    const run = startSpiedRun(fixture);
    run.handle.cancel();
    fixture.geometry.rootClientHeight = 400;
    await settleFrames(4);

    expect(fixture.scrollTo).not.toHaveBeenCalled();
    expect(run.spy.interrupted).toBe(1);
    expect(run.spy.arrived).toBe(0);
  });

  it("cancel after arrival only drops the pending arrival delivery", async () => {
    const fixture = createScrollFixture();

    const run = startSpiedRun(fixture);
    run.handle.cancel();
    await settleFrames(4);

    expect(run.spy.arrived).toBe(0);
    expect(run.spy.interrupted).toBe(0);
  });
});
