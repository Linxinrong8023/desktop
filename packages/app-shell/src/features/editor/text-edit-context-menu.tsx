/* eslint-disable react-refresh/only-export-components */
import {
  cloneElement,
  useState,
  type MouseEvent,
  type ReactElement,
  type ReactNode,
} from "react";
import { useTranslation } from "react-i18next";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
  cn,
} from "@ora/ui";

interface TextEditContextMenuProps {
  children: ReactNode;
  /** Host element merged onto the trigger so WebKit/Tauri still deliver `contextmenu`. */
  trigger: ReactElement<{
    className?: string;
    onContextMenu?: (event: MouseEvent<HTMLDivElement>) => void;
  }>;
  /** When false, Cut and Paste stay visible but disabled (read-only transcript). */
  editable: boolean;
  hasSelection: boolean;
  onOpenChange?: (open: boolean) => void;
  onCut: () => void;
  onCopy: () => void;
  onPaste: () => void;
  onSelectAll: () => void;
}

/**
 * Synchronously publishes plain text through `document.execCommand("copy")`.
 *
 * WebView2 selections that are mostly path link `<button>`s can leave the OS
 * clipboard without usable `CF_UNICODETEXT`, so external apps paste empty while
 * the same WebView can still read an internal/HTML payload. A temporary
 * textarea write flushes real system text and restores the prior selection.
 */
export function writeClipboardTextSync(text: string): boolean {
  const selection = window.getSelection();
  const previous: Range[] = [];
  if (selection !== null) {
    for (let index = 0; index < selection.rangeCount; index += 1) {
      previous.push(selection.getRangeAt(index));
    }
  }
  const textarea = document.createElement("textarea");
  textarea.value = text;
  textarea.setAttribute("readonly", "");
  textarea.style.cssText = "position:fixed;left:-9999px;top:0";
  document.body.append(textarea);
  textarea.focus();
  textarea.select();
  textarea.setSelectionRange(0, text.length);
  let ok: boolean;
  try {
    ok = document.execCommand("copy");
  } catch {
    ok = false;
  } finally {
    textarea.remove();
    if (selection !== null) {
      selection.removeAllRanges();
      for (const range of previous) {
        selection.addRange(range);
      }
    }
  }
  return ok;
}

const BLOCK_TAGS = new Set([
  "P",
  "DIV",
  "LI",
  "PRE",
  "TR",
  "H1",
  "H2",
  "H3",
  "H4",
  "H5",
  "H6",
  "BLOCKQUOTE",
  "SECTION",
  "ARTICLE",
]);

/**
 * Serializes the live selection to plain text, including `user-select: none`
 * controls such as chat path-link `<button>`s that `Selection#toString` skips.
 */
export function serializeSelectionPlainText(
  selection: Selection | null = window.getSelection(),
): string {
  if (selection === null || selection.rangeCount === 0) {
    return "";
  }
  const chunks: string[] = [];
  for (let index = 0; index < selection.rangeCount; index += 1) {
    chunks.push(serializeRangePlainText(selection.getRangeAt(index)));
  }
  return chunks.join("");
}

/** Walks a range clone so path buttons contribute text the way they render. */
function serializeRangePlainText(range: Range): string {
  if (range.collapsed) {
    return "";
  }
  const parts: string[] = [];
  const walk = (node: Node) => {
    if (node.nodeType === Node.TEXT_NODE) {
      parts.push(node.nodeValue ?? "");
      return;
    }
    if (
      node.nodeType !== Node.ELEMENT_NODE &&
      node.nodeType !== Node.DOCUMENT_FRAGMENT_NODE
    ) {
      return;
    }
    if (node.nodeType === Node.ELEMENT_NODE) {
      const tag = (node as Element).tagName;
      if (tag === "BR") {
        parts.push("\n");
        return;
      }
      if (tag === "SCRIPT" || tag === "STYLE") {
        return;
      }
    }
    for (const child of node.childNodes) {
      walk(child);
    }
    if (
      node.nodeType === Node.ELEMENT_NODE &&
      BLOCK_TAGS.has((node as Element).tagName)
    ) {
      parts.push("\n");
    }
  };
  walk(range.cloneContents());
  return parts.join("").replace(/\n+$/u, "");
}

/**
 * Forces `text/plain` on a native Copy so path-link-heavy transcript selections
 * do not leave external editors with an HTML-only clipboard.
 */
export function forcePlainTextClipboard(
  event: {
    clipboardData: DataTransfer | null;
    preventDefault: () => void;
  },
  text: string = serializeSelectionPlainText(),
): boolean {
  if (text.length === 0 || event.clipboardData === null) {
    return false;
  }
  event.clipboardData.setData("text/plain", text);
  event.preventDefault();
  return true;
}

/**
 * Writes plain text for Copy and Cut. Callers serialize their own selection
 * because a composer chip and a Markdown transcript are not the same payload.
 * Prefer the synchronous OS write first so external pastes see real text.
 * Returns whether either write path succeeded.
 */
export async function writeClipboardText(text: string): Promise<boolean> {
  if (writeClipboardTextSync(text)) {
    return true;
  }
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    // WebView may deny clipboard write; Copy/Cut already closed the menu.
    return false;
  }
}

/**
 * Reads clipboard text for Paste. A denied or empty clipboard is an empty
 * string so the editor does not insert a failed-read diagnostic.
 */
export async function readClipboardText(): Promise<string> {
  try {
    return await navigator.clipboard.readText();
  } catch {
    return "";
  }
}

/**
 * Reads image files from the clipboard. ClipboardItem has `getType`, not the
 * DataTransfer `getAsFiles` helper — that mismatch is why menu Paste saw no
 * images while Ctrl+V (which reads `clipboardData.files`) still worked.
 */
export async function readClipboardFiles(): Promise<File[]> {
  if (typeof navigator.clipboard.read !== "function") {
    return [];
  }
  try {
    const items = await navigator.clipboard.read();
    const files: File[] = [];
    for (const item of items) {
      for (const type of item.types) {
        if (!type.startsWith("image/")) {
          continue;
        }
        const blob = await item.getType(type);
        const subtype = type.slice("image/".length).split(";")[0] ?? "png";
        files.push(new File([blob], `clipboard.${subtype}`, { type }));
      }
    }
    return files;
  } catch {
    return [];
  }
}

/** Selects every node inside `element` so a later Copy uses the visible transcript. */
export function selectElementContents(element: HTMLElement): void {
  const selection = window.getSelection();
  if (selection === null) {
    return;
  }
  const range = document.createRange();
  range.selectNodeContents(element);
  selection.removeAllRanges();
  selection.addRange(range);
}

/**
 * Resolves a right-clicked image, including overlay buttons and preview canvases
 * whose `<img>` has `pointer-events-none`.
 */
function copyableImageFromEvent(
  event: MouseEvent<HTMLElement>,
): HTMLImageElement | null {
  const node = event.target;
  if (!(node instanceof Element)) {
    return null;
  }
  if (node instanceof HTMLImageElement && node.src !== "") {
    return node;
  }
  const host = node.closest("[data-copyable-image]");
  if (host === null) {
    return null;
  }
  if (host instanceof HTMLImageElement && host.src !== "") {
    return host;
  }
  const nested = host.querySelector("img");
  return nested instanceof HTMLImageElement && nested.src !== ""
    ? nested
    : null;
}

/** Decodes a data URL so ACP/composer images can be written without a network fetch. */
function blobFromDataUrl(src: string): Blob | null {
  const match = /^data:([^;,]+);base64,(.+)$/i.exec(src);
  if (match === null) {
    return null;
  }
  let binary: string;
  try {
    binary = atob(match[2] ?? "");
  } catch {
    return null;
  }
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return new Blob([bytes], { type: match[1] ?? "image/png" });
}

/** Rasterizes `img` to PNG. WebView2 rejects ClipboardItem JPEG payloads. */
function blobFromCanvas(img: HTMLImageElement): Promise<Blob | null> {
  const canvas = document.createElement("canvas");
  canvas.width = img.naturalWidth;
  canvas.height = img.naturalHeight;
  const context = canvas.getContext("2d");
  if (context === null || canvas.width === 0 || canvas.height === 0) {
    return Promise.resolve(null);
  }
  context.drawImage(img, 0, 0);
  return new Promise((resolve) => {
    // canvas.toBlob may pass null if the canvas is tainted or allocation fails.
    canvas.toBlob((blob) => resolve(blob ?? null), "image/png");
  });
}

/**
 * Copies the displayed bitmap. Prefer selecting the `<img>` and using the
 * same native copy path as Ctrl+C; ClipboardItem write is a fallback and must
 * be PNG or WebView2 silently drops it.
 */
export async function copyImageElement(img: HTMLImageElement): Promise<void> {
  if (copyImageViaNativeSelection(img)) {
    return;
  }
  if (img.decode !== undefined) {
    try {
      await img.decode();
    } catch {
      // Decode failure still allows a canvas attempt from the current frame.
    }
  }
  const fromDataUrl = blobFromDataUrl(img.src);
  const png =
    fromDataUrl?.type === "image/png" ? fromDataUrl : await blobFromCanvas(img);
  if (png === null || typeof ClipboardItem === "undefined") {
    return;
  }
  try {
    await navigator.clipboard.write([new ClipboardItem({ "image/png": png })]);
  } catch {
    try {
      await navigator.clipboard.write([
        new ClipboardItem({ "image/png": Promise.resolve(png) }),
      ]);
    } catch {
      // Neither ClipboardItem construction path is available in this WebView.
    }
  }
}

/**
 * User-gesture copy of an already-rendered image. Chromium still honors this
 * when `clipboard.write` of a ClipboardItem is denied to the WebView.
 */
function copyImageViaNativeSelection(img: HTMLImageElement): boolean {
  const selection = window.getSelection();
  if (selection === null) {
    return false;
  }
  const previous: Range[] = [];
  for (let index = 0; index < selection.rangeCount; index += 1) {
    previous.push(selection.getRangeAt(index));
  }
  const range = document.createRange();
  range.selectNode(img);
  selection.removeAllRanges();
  selection.addRange(range);
  try {
    return document.execCommand("copy");
  } finally {
    selection.removeAllRanges();
    for (const restored of previous) {
      selection.addRange(restored);
    }
  }
}

/**
 * Cursor-style Cut / Copy / Paste / Select All. The trigger is a `div` via
 * `render` so the default button trigger cannot swallow `contextmenu` in the
 * desktop WebView; `select-text` overrides the shared trigger's `select-none`.
 * Right-clicking a `[data-copyable-image]` host copies the bitmap instead of text.
 */
export function TextEditContextMenu({
  children,
  trigger,
  editable,
  hasSelection,
  onOpenChange,
  onCut,
  onCopy,
  onPaste,
  onSelectAll,
}: TextEditContextMenuProps) {
  const { t } = useTranslation();
  const [parkedImage, setParkedImage] = useState<HTMLImageElement | null>(null);
  const cutDisabled = !editable || !hasSelection;
  const copyDisabled = !hasSelection && parkedImage === null;
  const pasteDisabled = !editable;

  return (
    <ContextMenu onOpenChange={onOpenChange}>
      <ContextMenuTrigger
        className="select-text"
        render={cloneElement(trigger, {
          className: cn(trigger.props.className, "select-text"),
          onContextMenu: (event) => {
            event.preventDefault();
            const image = copyableImageFromEvent(event);
            setParkedImage(image);
            trigger.props.onContextMenu?.(event);
          },
        })}
      >
        {children}
      </ContextMenuTrigger>
      <ContextMenuContent className="min-w-36">
        <ContextMenuItem disabled={cutDisabled} onClick={onCut}>
          {t("chat.cut")}
        </ContextMenuItem>
        <ContextMenuItem
          disabled={copyDisabled}
          onClick={() => {
            if (parkedImage !== null) {
              void copyImageElement(parkedImage);
              return;
            }
            onCopy();
          }}
        >
          {t("chat.copy")}
        </ContextMenuItem>
        <ContextMenuItem disabled={pasteDisabled} onClick={onPaste}>
          {t("chat.paste")}
        </ContextMenuItem>
        <ContextMenuSeparator />
        <ContextMenuItem onClick={onSelectAll}>
          {t("chat.selectAll")}
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  );
}
