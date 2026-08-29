import { describe, expect, it } from "vitest";
import {
  diffFileScrollTop,
  diffLineScrollTop,
  isDiffFileAligned,
  isDiffScrollAtEnd,
} from "./task-diff-scroll";

/** Builds a stub element whose geometry tests can set independently of jsdom layout. */
function geometryElement(values: {
  clientHeight?: number;
  offsetHeight?: number;
  offsetTop?: number;
  scrollHeight?: number;
  scrollTop?: number;
  top: number;
  height: number;
}): HTMLElement {
  const element = document.createElement("div");
  Object.defineProperty(element, "clientHeight", {
    configurable: true,
    value: values.clientHeight ?? values.height,
  });
  Object.defineProperty(element, "offsetHeight", {
    configurable: true,
    value: values.offsetHeight ?? values.height,
  });
  Object.defineProperty(element, "offsetTop", {
    configurable: true,
    value: values.offsetTop ?? values.top,
  });
  Object.defineProperty(element, "scrollHeight", {
    configurable: true,
    value: values.scrollHeight ?? values.height + (values.scrollTop ?? 0),
  });
  Object.defineProperty(element, "scrollTop", {
    configurable: true,
    value: values.scrollTop ?? 0,
  });
  element.getBoundingClientRect = () =>
    ({
      x: 0,
      y: values.top,
      top: values.top,
      left: 0,
      right: 800,
      bottom: values.top + values.height,
      width: 800,
      height: values.height,
      toJSON() {
        return {};
      },
    }) as DOMRect;
  return element;
}

describe("diffFileScrollTop", () => {
  it("returns null while the diff viewport has not received a height", () => {
    const root = geometryElement({
      clientHeight: 0,
      height: 0,
      top: 0,
    });
    const file = geometryElement({
      offsetHeight: 120,
      offsetTop: 800,
      height: 120,
      top: 800,
    });

    expect(diffFileScrollTop(root, file)).toBeNull();
  });

  it("returns null while the requested file section has not laid out", () => {
    const root = geometryElement({
      clientHeight: 400,
      height: 400,
      top: 0,
    });
    const file = geometryElement({
      offsetHeight: 0,
      offsetTop: 0,
      height: 0,
      top: 0,
    });

    expect(diffFileScrollTop(root, file)).toBeNull();
  });

  it("clamps a jump whose destination is the list start to the container top", () => {
    // Mirrors the live repro: the first list file sits above the viewport, so
    // the honest destination is scrollTop ≈ 0, not a rejection.
    const root = geometryElement({
      clientHeight: 727,
      height: 727,
      top: 104,
      scrollTop: 1559,
    });
    const file = geometryElement({
      offsetHeight: 329,
      offsetTop: 1563,
      height: 329,
      top: -1454,
    });

    expect(diffFileScrollTop(root, file)).toBe(0);
  });

  it("places a later file near the top of the viewport", () => {
    const root = geometryElement({
      clientHeight: 400,
      height: 400,
      top: 0,
      scrollTop: 0,
    });
    const file = geometryElement({
      offsetHeight: 120,
      offsetTop: 800,
      height: 120,
      top: 800,
    });

    expect(diffFileScrollTop(root, file)).toBe(784);
  });

  it("keeps the first file at the top of the viewport", () => {
    const root = geometryElement({
      clientHeight: 400,
      height: 400,
      top: 0,
      scrollTop: 0,
    });
    const file = geometryElement({
      offsetHeight: 120,
      offsetTop: 0,
      height: 120,
      top: 0,
    });

    expect(diffFileScrollTop(root, file)).toBe(0);
  });
});

describe("diffLineScrollTop", () => {
  it("returns null while the diff viewport has not received a height", () => {
    const root = geometryElement({
      clientHeight: 0,
      height: 0,
      top: 0,
      scrollTop: 500,
    });
    const line = geometryElement({
      offsetHeight: 24,
      height: 24,
      top: 1000,
    });

    expect(diffLineScrollTop(root, line)).toBeNull();
  });

  it("returns null while the target line has not laid out", () => {
    const root = geometryElement({
      clientHeight: 400,
      height: 400,
      top: 0,
      scrollTop: 500,
    });
    const line = geometryElement({
      offsetHeight: 0,
      height: 0,
      top: 1000,
    });

    expect(diffLineScrollTop(root, line)).toBeNull();
  });

  it("vertically centers a scrolled line inside the viewport", () => {
    const root = geometryElement({
      clientHeight: 400,
      height: 400,
      top: 0,
      scrollTop: 500,
    });
    const line = geometryElement({
      offsetHeight: 24,
      height: 24,
      top: 1000,
    });

    // content offset 1000 - 0 + 500 = 1500, pulled up by (400 - 24) / 2 = 188.
    expect(diffLineScrollTop(root, line)).toBe(1312);
  });
});

describe("isDiffFileAligned", () => {
  it("is false while a later file still sits below the viewport top", () => {
    const root = geometryElement({
      clientHeight: 400,
      height: 400,
      top: 0,
    });
    const file = geometryElement({
      offsetHeight: 120,
      offsetTop: 800,
      height: 120,
      top: 800,
    });

    expect(isDiffFileAligned(root, file)).toBe(false);
  });

  it("is false after a jump based on placeholder height overshoots the file", () => {
    const root = geometryElement({
      clientHeight: 727,
      height: 727,
      top: 105,
    });
    const file = geometryElement({
      offsetHeight: 544,
      offsetTop: 3525,
      height: 544,
      top: -842,
    });

    expect(isDiffFileAligned(root, file)).toBe(false);
  });

  it("is true when the requested file sits near the viewport top", () => {
    const root = geometryElement({
      clientHeight: 400,
      height: 400,
      top: 0,
    });
    const file = geometryElement({
      offsetHeight: 120,
      offsetTop: 800,
      height: 120,
      top: 16,
    });

    expect(isDiffFileAligned(root, file)).toBe(true);
  });
});

describe("isDiffScrollAtEnd", () => {
  it("is false while the container can still scroll toward the target", () => {
    const root = geometryElement({
      clientHeight: 400,
      height: 400,
      scrollHeight: 5000,
      scrollTop: 0,
      top: 0,
    });

    expect(isDiffScrollAtEnd(root)).toBe(false);
  });

  it("is true when the bottom clamp prevents further scrolling", () => {
    const root = geometryElement({
      clientHeight: 400,
      height: 400,
      scrollHeight: 5000,
      scrollTop: 4600,
      top: 0,
    });

    expect(isDiffScrollAtEnd(root)).toBe(true);
  });

  it("is true when all content already fits the viewport", () => {
    const root = geometryElement({
      clientHeight: 400,
      height: 400,
      scrollHeight: 300,
      scrollTop: 0,
      top: 0,
    });

    expect(isDiffScrollAtEnd(root)).toBe(true);
  });
});
