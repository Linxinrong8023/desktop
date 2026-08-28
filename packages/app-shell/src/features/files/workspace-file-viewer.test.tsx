import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { appI18n } from "../../i18n/i18n-instance";
import { WorkspaceFileViewer } from "./workspace-file-viewer";
import { utf8ByteColumnToStringIndex } from "./workspace-file-viewer-utils";

vi.mock("../chat/add-composer-file-selection", () => ({
  addComposerFileSelections: vi.fn(),
}));

const addComposerFileSelections = vi.mocked(
  await import("../chat/add-composer-file-selection"),
).addComposerFileSelections;

afterEach(() => {
  vi.restoreAllMocks();
  addComposerFileSelections.mockClear();
});

describe("WorkspaceFileViewer", () => {
  it("converts UTF-8 byte columns before locating a browser string index", () => {
    expect(utf8ByteColumnToStringIndex("α main", 4)).toBe(2);
  });

  it("scrolls to and highlights the exact selected search match", async () => {
    const scrollIntoView = vi.spyOn(Element.prototype, "scrollIntoView");

    render(
      <WorkspaceFileViewer
        content={"first\nα main()\nlast"}
        path="src/main.rs"
        target={{ line: 2, column: 4, matchedText: "main" }}
      />,
    );

    await waitFor(() => expect(screen.getByText("main")).toBeInTheDocument());
    expect(screen.getByText("main").tagName).toBe("MARK");
    expect(
      screen.getByText("main").closest("[aria-current=location]"),
    ).not.toBeNull();
    expect(scrollIntoView).toHaveBeenCalledWith({
      block: "center",
      inline: "nearest",
    });
  });

  it("highlights every line in a cited range and scrolls to the start", async () => {
    const scrollIntoView = vi.spyOn(Element.prototype, "scrollIntoView");

    const { container } = render(
      <WorkspaceFileViewer
        content={"one\ntwo\nthree\nfour"}
        path="README.md"
        target={{ line: 2, column: 1, matchedText: "", endLine: 4 }}
      />,
    );

    await waitFor(() =>
      expect(container.querySelector('[data-line-number="2"]')).not.toBeNull(),
    );
    expect(container.querySelector('[data-line-number="2"]')).toHaveAttribute(
      "aria-current",
      "location",
    );
    expect(
      container.querySelectorAll("[data-cited-range='true']"),
    ).toHaveLength(3);
    expect(
      [...container.querySelectorAll("[data-cited-range='true']")].map((node) =>
        node.getAttribute("data-line-number"),
      ),
    ).toEqual(["2", "3", "4"]);
    expect(
      container.querySelectorAll(
        "[data-cited-range='true'] [data-quote-gutter]",
      ),
    ).toHaveLength(3);
    expect(
      container.querySelector('[data-line-number="1"]'),
    ).not.toHaveAttribute("aria-current");
    expect(scrollIntoView).toHaveBeenCalledWith({
      block: "center",
      inline: "nearest",
    });
  });

  it("keeps a search match when clicking another line", async () => {
    const { container } = render(
      <WorkspaceFileViewer
        content={"first\nα main()\nlast"}
        path="src/main.rs"
        target={{ line: 2, column: 4, matchedText: "main" }}
      />,
    );

    await waitFor(() => expect(screen.getByText("main")).toBeInTheDocument());
    fireEvent.mouseDown(container.querySelector('[data-line-number="1"]')!, {
      button: 0,
    });

    expect(screen.getByText("main").closest("mark")).not.toBeNull();
    expect(container.querySelector('[data-line-number="2"]')).toHaveAttribute(
      "aria-current",
      "location",
    );
    expect(container.querySelector("[data-cited-range='true']")).toBeNull();
  });

  it("does not dismiss a cited range when clicking the gutter + control", async () => {
    const { container } = render(
      <WorkspaceFileViewer
        content={"one\ntwo\nthree\nfour"}
        path="README.md"
        target={{ line: 2, column: 1, matchedText: "", endLine: 4 }}
      />,
    );

    await waitFor(() =>
      expect(
        screen.getByRole("button", {
          name: appI18n.t("files.quoteLineToChat", { line: 1 }),
        }),
      ).toBeInTheDocument(),
    );
    fireEvent.mouseDown(
      screen.getByRole("button", {
        name: appI18n.t("files.quoteLineToChat", { line: 1 }),
      }),
      { button: 0 },
    );

    expect(
      container.querySelectorAll("[data-cited-range='true']"),
    ).toHaveLength(3);
  });

  it("clears the cited-range highlight when clicking another line", async () => {
    const { container } = render(
      <WorkspaceFileViewer
        content={"one\ntwo\nthree\nfour"}
        path="README.md"
        target={{ line: 2, column: 1, matchedText: "", endLine: 4 }}
      />,
    );

    await waitFor(() =>
      expect(
        container.querySelectorAll("[data-cited-range='true']"),
      ).toHaveLength(3),
    );

    fireEvent.mouseDown(container.querySelector('[data-line-number="1"]')!, {
      button: 0,
    });

    expect(
      container.querySelectorAll("[data-cited-range='true']"),
    ).toHaveLength(0);
    expect(
      container.querySelector('[data-line-number="2"]'),
    ).not.toHaveAttribute("aria-current");
  });

  it("keeps the cited-range highlight when clicking inside it", async () => {
    const { container } = render(
      <WorkspaceFileViewer
        content={"one\ntwo\nthree\nfour"}
        path="README.md"
        target={{ line: 2, column: 1, matchedText: "", endLine: 4 }}
      />,
    );

    await waitFor(() =>
      expect(container.querySelector('[data-line-number="3"]')).not.toBeNull(),
    );

    fireEvent.mouseDown(container.querySelector('[data-line-number="3"]')!, {
      button: 0,
    });

    expect(
      container.querySelectorAll("[data-cited-range='true']"),
    ).toHaveLength(3);
  });

  it("restores the cited-range highlight when a new jump target arrives", async () => {
    const { container, rerender } = render(
      <WorkspaceFileViewer
        content={"one\ntwo\nthree\nfour"}
        path="README.md"
        target={{ line: 2, column: 1, matchedText: "", endLine: 4 }}
      />,
    );

    await waitFor(() =>
      expect(
        container.querySelectorAll("[data-cited-range='true']"),
      ).toHaveLength(3),
    );
    fireEvent.mouseDown(container.querySelector('[data-line-number="1"]')!, {
      button: 0,
    });
    expect(
      container.querySelectorAll("[data-cited-range='true']"),
    ).toHaveLength(0);

    rerender(
      <WorkspaceFileViewer
        content={"one\ntwo\nthree\nfour"}
        path="README.md"
        target={{ line: 2, column: 1, matchedText: "", endLine: 4 }}
      />,
    );

    expect(
      container.querySelectorAll("[data-cited-range='true']"),
    ).toHaveLength(3);
  });

  it("enables horizontal scrolling for long source lines", async () => {
    const { container } = render(
      <WorkspaceFileViewer
        content={"a".repeat(300)}
        path="README.md"
        target={null}
      />,
    );

    await waitFor(() =>
      expect(
        container.querySelector(
          '[data-slot="scroll-area"][data-scrollbars="both"]',
        ),
      ).not.toBeNull(),
    );
  });

  it("switches large files to plain text mode", async () => {
    const { container } = render(
      <WorkspaceFileViewer
        content={"a".repeat(512 * 1024 + 1)}
        path="large.log"
        target={null}
      />,
    );

    await waitFor(() =>
      expect(
        container.querySelector("[data-large-file-notice]"),
      ).not.toBeNull(),
    );
  });

  it("pins a clicked line number and extends with shift-click", async () => {
    const { container } = render(
      <WorkspaceFileViewer
        content={"one\ntwo\nthree\nfour"}
        path="README.md"
        target={null}
      />,
    );

    await waitFor(() =>
      expect(
        screen.getByRole("button", {
          name: appI18n.t("files.selectLine", { line: 2 }),
        }),
      ).toBeInTheDocument(),
    );
    const clickNumber = (line: number, shiftKey = false) => {
      const number = screen.getByRole("button", {
        name: appI18n.t("files.selectLine", { line }),
      });
      // Click always trails a mousedown/mouseup pair in real browsers.
      fireEvent.mouseDown(number, { button: 0 });
      fireEvent.mouseUp(number);
      fireEvent.click(number, { shiftKey });
    };
    clickNumber(2);
    clickNumber(4, true);

    const pinned = container.querySelectorAll("[data-quote-pinned]");
    expect(pinned).toHaveLength(3);
    expect(addComposerFileSelections).not.toHaveBeenCalled();
  });

  it("quotes a single line through the gutter + control", async () => {
    render(
      <WorkspaceFileViewer
        content={"one\ntwo\nthree"}
        path="README.md"
        target={null}
      />,
    );

    await waitFor(() =>
      expect(
        screen.getByRole("button", {
          name: appI18n.t("files.quoteLineToChat", { line: 2 }),
        }),
      ).toBeInTheDocument(),
    );
    fireEvent.click(
      screen.getByRole("button", {
        name: appI18n.t("files.quoteLineToChat", { line: 2 }),
      }),
    );

    expect(addComposerFileSelections).toHaveBeenCalledWith([
      { path: "README.md", startLine: 2, endLine: 2 },
    ]);
  });

  it("quotes a dragged line range tracked through code cells via mousemove", async () => {
    const { container } = render(
      <WorkspaceFileViewer
        content={"one\ntwo\nthree\nfour"}
        path="README.md"
        target={null}
      />,
    );

    await waitFor(() =>
      expect(container.querySelector('[data-quote-key="2"]')).not.toBeNull(),
    );
    const gutter = container.querySelector(
      '[data-quote-key="2"] [data-quote-gutter]',
    );
    const endRow = container.querySelector('[data-quote-key="4"]');
    expect(gutter).not.toBeNull();
    expect(endRow).not.toBeNull();

    fireEvent.mouseDown(gutter!, { button: 0 });
    vi.spyOn(document, "elementFromPoint").mockReturnValue(endRow);
    fireEvent.mouseMove(window, { buttons: 1, clientX: 10, clientY: 10 });
    fireEvent.mouseUp(window);

    expect(addComposerFileSelections).toHaveBeenCalledWith([
      {
        path: "README.md",
        startLine: 2,
        endLine: 4,
      },
    ]);
  });

  it("starts a multi-line drag from the line number while the + control is present", async () => {
    const { container } = render(
      <WorkspaceFileViewer
        content={"one\ntwo\nthree\nfour"}
        path="README.md"
        target={null}
      />,
    );

    await waitFor(() =>
      expect(
        screen.getByRole("button", {
          name: appI18n.t("files.quoteLineToChat", { line: 2 }),
        }),
      ).toBeInTheDocument(),
    );
    const lineNumber = screen.getByRole("button", {
      name: appI18n.t("files.selectLine", { line: 2 }),
    });
    const endRow = container.querySelector('[data-quote-key="4"]');
    expect(endRow).not.toBeNull();

    // Drag must start on the number too; hover/visibility of + cannot block it.
    fireEvent.mouseDown(lineNumber, { button: 0 });
    vi.spyOn(document, "elementFromPoint").mockReturnValue(endRow);
    fireEvent.mouseMove(window, { buttons: 1, clientX: 10, clientY: 10 });
    fireEvent.mouseUp(window);

    expect(addComposerFileSelections).toHaveBeenCalledWith([
      {
        path: "README.md",
        startLine: 2,
        endLine: 4,
      },
    ]);
  });

  it("pins the focused line via keyboard instead of quoting", async () => {
    const { container } = render(
      <WorkspaceFileViewer
        content={"one\ntwo"}
        path="README.md"
        target={null}
      />,
    );

    await waitFor(() =>
      expect(
        screen.getByRole("button", {
          name: appI18n.t("files.selectLine", { line: 2 }),
        }),
      ).toBeInTheDocument(),
    );
    fireEvent.keyDown(
      screen.getByRole("button", {
        name: appI18n.t("files.selectLine", { line: 2 }),
      }),
      { key: "Enter" },
    );

    expect(addComposerFileSelections).not.toHaveBeenCalled();
    expect(
      container
        .querySelector('[data-quote-key="2"]')
        ?.getAttribute("data-quote-pinned"),
    ).toBe("true");
  });

  it("quotes the pinned range from the keyboard with Ctrl+Enter", async () => {
    render(
      <WorkspaceFileViewer
        content={"one\ntwo\nthree\nfour"}
        path="README.md"
        target={null}
      />,
    );

    await waitFor(() =>
      expect(
        screen.getByRole("button", {
          name: appI18n.t("files.selectLine", { line: 2 }),
        }),
      ).toBeInTheDocument(),
    );
    const line = (n: number) =>
      screen.getByRole("button", {
        name: appI18n.t("files.selectLine", { line: n }),
      });

    // Pin 2-3 with Enter + shift-Enter, then quote the pinned range.
    fireEvent.keyDown(line(2), { key: "Enter" });
    fireEvent.keyDown(line(3), { key: "Enter", shiftKey: true });
    expect(addComposerFileSelections).not.toHaveBeenCalled();

    fireEvent.keyDown(line(3), { key: "Enter", ctrlKey: true });
    expect(addComposerFileSelections).toHaveBeenCalledWith([
      { path: "README.md", startLine: 2, endLine: 3 },
    ]);
  });

  it("keeps the quote + control as a sibling of the line-number button without an SVG", async () => {
    const { container } = render(
      <WorkspaceFileViewer
        content={"one\ntwo"}
        path="README.md"
        target={null}
      />,
    );

    await waitFor(() =>
      expect(
        screen.getByRole("button", {
          name: appI18n.t("files.selectLine", { line: 1 }),
        }),
      ).toBeInTheDocument(),
    );
    const lineButton = screen.getByRole("button", {
      name: appI18n.t("files.selectLine", { line: 1 }),
    });
    const quoteButton = screen.getByRole("button", {
      name: appI18n.t("files.quoteLineToChat", { line: 1 }),
    });
    expect(lineButton.contains(quoteButton)).toBe(false);
    expect(quoteButton.parentElement).toBe(lineButton.parentElement);
    expect(container.querySelector("[data-quote-button] svg")).toBeNull();
  });
});
