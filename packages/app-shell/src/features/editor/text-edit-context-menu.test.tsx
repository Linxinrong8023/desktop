import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AppI18nProvider } from "../../i18n/i18n";
import { appI18n } from "../../i18n/i18n-instance";
import {
  copyImageElement,
  forcePlainTextClipboard,
  serializeSelectionPlainText,
  TextEditContextMenu,
  writeClipboardText,
  writeClipboardTextSync,
} from "./text-edit-context-menu";

afterEach(async () => {
  cleanup();
  await appI18n.changeLanguage("zh-CN");
});

const ONE_PIXEL_PNG =
  "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

describe("TextEditContextMenu", () => {
  it.each([
    { locale: "zh-CN", labels: ["剪切", "复制", "粘贴", "全选"] },
    { locale: "en-US", labels: ["Cut", "Copy", "Paste", "Select All"] },
  ] as const)(
    "renders editing actions in $locale and disables read-only actions",
    async ({ locale, labels }) => {
      await act(async () => {
        await appI18n.changeLanguage(locale);
      });
      const [cut, copy, paste, selectAll] = labels;
      const user = userEvent.setup();
      render(
        <AppI18nProvider>
          <TextEditContextMenu
            trigger={<div data-testid="host">body</div>}
            editable={false}
            hasSelection={false}
            onCut={() => undefined}
            onCopy={() => undefined}
            onPaste={() => undefined}
            onSelectAll={() => undefined}
          >
            visible
          </TextEditContextMenu>
        </AppI18nProvider>,
      );

      fireEvent.contextMenu(screen.getByTestId("host"));

      expect(
        await screen.findByRole("menuitem", { name: cut }),
      ).toHaveAttribute("data-disabled");
      expect(screen.getByRole("menuitem", { name: copy })).toHaveAttribute(
        "data-disabled",
      );
      expect(screen.getByRole("menuitem", { name: paste })).toHaveAttribute(
        "data-disabled",
      );
      expect(
        screen.getByRole("menuitem", { name: selectAll }),
      ).not.toHaveAttribute("data-disabled");
      await user.click(screen.getByRole("menuitem", { name: selectAll }));
    },
  );

  it("copies the bitmap when the right-click target is a copyable image", async () => {
    const user = userEvent.setup();
    Object.defineProperty(document, "execCommand", {
      configurable: true,
      value: vi.fn().mockReturnValue(true),
    });

    render(
      <AppI18nProvider>
        <TextEditContextMenu
          trigger={<div data-testid="host" />}
          editable={false}
          hasSelection={false}
          onCut={() => undefined}
          onCopy={() => undefined}
          onPaste={() => undefined}
          onSelectAll={() => undefined}
        >
          <figure data-copyable-image>
            <img alt="shot" src={ONE_PIXEL_PNG} />
          </figure>
        </TextEditContextMenu>
      </AppI18nProvider>,
    );

    fireEvent.contextMenu(screen.getByAltText("shot"));
    const copy = await screen.findByRole("menuitem", { name: "复制" });
    await waitFor(() => expect(copy).not.toHaveAttribute("data-disabled"));
    await user.click(copy);
    await waitFor(() =>
      expect(document.execCommand).toHaveBeenCalledWith("copy"),
    );
  });

  it("writes a PNG ClipboardItem when native image copy is unavailable", async () => {
    const user = userEvent.setup();
    Object.defineProperty(document, "execCommand", {
      configurable: true,
      value: vi.fn().mockReturnValue(false),
    });
    const write = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal(
      "ClipboardItem",
      class {
        constructor(public items: Record<string, Blob>) {}
      },
    );
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { write, writeText: vi.fn(), readText: vi.fn() },
    });

    render(
      <AppI18nProvider>
        <TextEditContextMenu
          trigger={<div data-testid="host" />}
          editable={false}
          hasSelection={false}
          onCut={() => undefined}
          onCopy={() => undefined}
          onPaste={() => undefined}
          onSelectAll={() => undefined}
        >
          <figure data-copyable-image>
            <img alt="shot" src={ONE_PIXEL_PNG} />
          </figure>
        </TextEditContextMenu>
      </AppI18nProvider>,
    );

    fireEvent.contextMenu(screen.getByAltText("shot"));
    const copy = await screen.findByRole("menuitem", { name: "复制" });
    await waitFor(() => expect(copy).not.toHaveAttribute("data-disabled"));
    await user.click(copy);
    await waitFor(() => expect(write).toHaveBeenCalled());
  });

  it("swallows clipboard write denials so Copy does not reject", async () => {
    Object.defineProperty(document, "execCommand", {
      configurable: true,
      value: vi.fn().mockReturnValue(false),
    });
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText: vi.fn().mockRejectedValue(new Error("denied")) },
    });
    await expect(writeClipboardText("hello")).resolves.toBe(false);
  });

  it("prefers a synchronous execCommand write so external pastes get plain text", async () => {
    const execCommand = vi.fn().mockReturnValue(true);
    Object.defineProperty(document, "execCommand", {
      configurable: true,
      value: execCommand,
    });
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });
    await expect(writeClipboardText("AGENTS.md\nCargo.toml")).resolves.toBe(
      true,
    );
    expect(execCommand).toHaveBeenCalledWith("copy");
    expect(writeText).not.toHaveBeenCalled();
  });

  it("falls back to clipboard.writeText when execCommand fails", async () => {
    Object.defineProperty(document, "execCommand", {
      configurable: true,
      value: vi.fn().mockReturnValue(false),
    });
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });
    await expect(writeClipboardText("README.md")).resolves.toBe(true);
    expect(writeText).toHaveBeenCalledWith("README.md");
  });

  it("forces text/plain onto a native Copy event", () => {
    const setData = vi.fn();
    const preventDefault = vi.fn();
    expect(
      forcePlainTextClipboard(
        {
          clipboardData: { setData } as unknown as DataTransfer,
          preventDefault,
        },
        "crates/\npackages/\nAGENTS.md",
      ),
    ).toBe(true);
    expect(setData).toHaveBeenCalledWith(
      "text/plain",
      "crates/\npackages/\nAGENTS.md",
    );
    expect(preventDefault).toHaveBeenCalled();
  });

  it("leaves native Copy alone when the selection is empty", () => {
    const setData = vi.fn();
    const preventDefault = vi.fn();
    expect(
      forcePlainTextClipboard(
        {
          clipboardData: { setData } as unknown as DataTransfer,
          preventDefault,
        },
        "",
      ),
    ).toBe(false);
    expect(setData).not.toHaveBeenCalled();
    expect(preventDefault).not.toHaveBeenCalled();
  });

  it("serializes path-link buttons that Selection#toString skips", () => {
    const host = document.createElement("div");
    host.innerHTML =
      '<p>文件:</p><p><button style="user-select:none">AGENTS.md</button> - Agent文档</p>' +
      '<p><button style="user-select:none">Cargo.lock</button> / ' +
      '<button style="user-select:none">Cargo.toml</button> - Rust项目配置</p>';
    document.body.append(host);
    const selection = window.getSelection();
    const range = document.createRange();
    range.selectNodeContents(host);
    selection?.removeAllRanges();
    selection?.addRange(range);

    const serialized = serializeSelectionPlainText(selection);
    expect(serialized).toContain("AGENTS.md");
    expect(serialized).toContain("Cargo.lock");
    expect(serialized).toContain("Cargo.toml");
    expect(serialized).toContain("Agent文档");
    // Native toString drops user-select:none button text in WebView/Chromium.
    expect(serialized.length).toBeGreaterThan(
      (selection?.toString() ?? "").length,
    );

    host.remove();
  });

  it("restores the live selection after a sync clipboard write", () => {
    Object.defineProperty(document, "execCommand", {
      configurable: true,
      value: vi.fn().mockReturnValue(true),
    });
    const host = document.createElement("div");
    host.textContent = "path/to/file.rs";
    document.body.append(host);
    const selection = window.getSelection();
    const range = document.createRange();
    range.selectNodeContents(host);
    selection?.removeAllRanges();
    selection?.addRange(range);

    expect(writeClipboardTextSync("path/to/file.rs")).toBe(true);
    expect(selection?.toString()).toBe("path/to/file.rs");
    host.remove();
  });

  it("does not reject when both ClipboardItem construction paths throw", async () => {
    Object.defineProperty(document, "execCommand", {
      configurable: true,
      value: vi.fn().mockReturnValue(false),
    });
    vi.stubGlobal(
      "ClipboardItem",
      class {
        constructor() {
          throw new Error("unsupported");
        }
      },
    );
    const img = document.createElement("img");
    img.src = ONE_PIXEL_PNG;
    document.body.append(img);
    await expect(copyImageElement(img)).resolves.toBeUndefined();
    img.remove();
  });
});
