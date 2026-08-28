import {
  composerFileAttrsFromUnknown,
  composerFileChipTitle,
  composerFileLabel,
} from "@ora/editor/composer";
import { IconX } from "@tabler/icons-react";
import { NodeViewWrapper, type NodeViewProps } from "@tiptap/react";
import { useRef } from "react";
import type {
  DragEvent as ReactDragEvent,
  MouseEvent as ReactMouseEvent,
} from "react";
import { useTranslation } from "react-i18next";
import { useTaskChangesNavigation } from "../diff/task-changes-navigation-context";
import { navigateToFileRef } from "../file-ref-chip-navigation";
import { FileRefChipContent } from "../file-ref-chip";

/**
 * Inline path mention chip: type/folder icon + basename, Cursor-style (no pill).
 * Wired through `AppComposerFile` so the explorer and @ picker share one visual.
 *
 * A plain single click jumps to the reference's Files/Changes location, same
 * as the read-only history chip. Double-click and Ctrl/Cmd-click keep their
 * existing select behaviour (for delete/drag) instead of navigating: they
 * pin a TextSelection over the atom so the caret stays visible and the chip
 * paints `data-chip-selected`. Without a navigation context, a plain click
 * selects the same way.
 */
export function ComposerFileChipView({ node, editor, getPos }: NodeViewProps) {
  const { t } = useTranslation();
  const attrs = composerFileAttrsFromUnknown(node.attrs);
  const kind = attrs.kind === "directory" ? "directory" : "file";
  const title = composerFileChipTitle(attrs);
  const navigation = useTaskChangesNavigation();
  // Armed by the select-only mousedown branch so the click completing that
  // same press does not also navigate.
  const suppressNextClickRef = useRef(false);

  const selectOnlyThisChip = (event: ReactMouseEvent<HTMLElement>): void => {
    const pos = getPos();
    // Bail before consuming the press: preventing the default on a stale
    // (destroyed) node would swallow the mousedown without focusing the
    // editor, so the caret would silently disappear.
    if (typeof pos !== "number") return;
    event.preventDefault();
    event.stopPropagation();
    editor
      .chain()
      .focus()
      .setTextSelection({ from: pos, to: pos + node.nodeSize })
      .run();
  };

  /** Drops this reference from the prompt; hover swaps the type icon for it. */
  const removeThisChip = (event: ReactMouseEvent<HTMLElement>): void => {
    event.preventDefault();
    event.stopPropagation();
    const pos = getPos();
    if (typeof pos !== "number") return;
    editor
      .chain()
      .focus()
      .deleteRange({ from: pos, to: pos + node.nodeSize })
      .run();
  };

  return (
    <NodeViewWrapper
      as="span"
      className="composer-file-ref"
      data-composer-file={attrs.path}
      data-kind={kind}
      {...(navigation === null ? {} : { "data-navigable": "true" })}
      {...(attrs.startLine === undefined
        ? {}
        : { "data-start-line": String(attrs.startLine) })}
      {...(attrs.endLine === undefined
        ? {}
        : { "data-end-line": String(attrs.endLine) })}
      contentEditable={false}
      title={title}
      draggable={false}
      onDragStart={(event: ReactDragEvent<HTMLElement>) => {
        event.preventDefault();
      }}
      onMouseDown={(event: ReactMouseEvent<HTMLElement>) => {
        if (event.button !== 0) return;
        if (event.detail >= 2 || event.ctrlKey || event.metaKey) {
          // Armed here rather than inside the shared handler: only a mousedown
          // is followed by a click that must not also navigate. `dblclick`
          // fires after its own click was already consumed, so arming there
          // would strand the flag and swallow the chip next genuine click.
          suppressNextClickRef.current = true;
          selectOnlyThisChip(event);
        }
      }}
      onDoubleClick={selectOnlyThisChip}
      onClick={(event: ReactMouseEvent<HTMLElement>) => {
        if (suppressNextClickRef.current) {
          suppressNextClickRef.current = false;
          return;
        }
        if (event.button !== 0) return;
        if (navigation === null) {
          // No Files/Changes jump: treat the click as a select so the chip
          // highlights and the caret is not swallowed by a NodeSelection.
          selectOnlyThisChip(event);
          return;
        }
        event.preventDefault();
        event.stopPropagation();
        navigateToFileRef(attrs, navigation);
      }}
    >
      {editor.isEditable && (
        <button
          type="button"
          // Chips sit in a contenteditable; a tab stop per chip would bury the
          // send button. Keyboard removal stays select-the-chip plus Backspace.
          tabIndex={-1}
          className="composer-file-ref-remove"
          aria-label={t("chat.removeFileReference", {
            name: composerFileLabel(attrs),
          })}
          onMouseDown={(event: ReactMouseEvent<HTMLElement>) => {
            // Claim the press so the editor cannot node-select the chip first.
            event.preventDefault();
            event.stopPropagation();
          }}
          onClick={removeThisChip}
        >
          <IconX className="composer-file-ref-remove-glyph" />
        </button>
      )}
      {/* Renders after the button so hover can swap them in the same slot. */}
      <FileRefChipContent attrs={attrs} />
    </NodeViewWrapper>
  );
}
