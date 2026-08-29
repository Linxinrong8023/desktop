import { Extension, type Editor } from "@tiptap/core";
import type { Node as PmNode } from "@tiptap/pm/model";
import {
  NodeSelection,
  Plugin,
  PluginKey,
  TextSelection,
  type EditorState,
} from "@tiptap/pm/state";
import type { EditorView } from "@tiptap/pm/view";

const CHIP_NODE_TYPES = new Set(["composerFile", "promptToken"]);
/**
 * Chip types whose plain click is owned by a host-shell node view
 * (`ComposerFileChipView` navigates, or selects when there is nothing to
 * navigate to). The plugin still swallows ProseMirror's default atom click
 * for them; it just does not pin, so the host handler decides.
 */
const HOST_OWNED_PLAIN_CLICKS = new Set(["composerFile"]);
export const composerChipSelectionKey = new PluginKey("composerChipSelection");
const CHIP_SELECTED_ATTR = "data-chip-selected";

/**
 * Builds a TextSelection for a pointer drag. Atom chips are `user-select: none`
 * and as wide as a filename, so caret mapping lands on whichever half of the
 * chip the pointer is over — native selection would wait until the midpoint.
 * Once the pointer is inside a chip, snap the head (and the anchor, if the
 * press started on that same chip) so the whole atom is in the range.
 */
export function textSelectionForChipDrag(
  doc: PmNode,
  anchorPos: number,
  headPos: number,
  pointerInside: number,
): TextSelection {
  let anchor = anchorPos;
  let head = headPos;
  const chip = chipRangeAt(doc, pointerInside);
  if (chip !== null) {
    if (headPos >= anchorPos) {
      head = Math.max(headPos, chip.end);
      if (anchorPos > chip.start && anchorPos < chip.end) anchor = chip.start;
    } else {
      head = Math.min(headPos, chip.start);
      if (anchorPos > chip.start && anchorPos < chip.end) anchor = chip.end;
    }
  }
  return TextSelection.create(doc, anchor, head);
}

/** Inclusive chip span when `pos` is the atom's own position. */
export function chipRangeAt(
  doc: PmNode,
  pos: number,
): { start: number; end: number } | null {
  if (pos < 0 || pos > doc.content.size) return null;
  const node = doc.nodeAt(pos);
  if (node === null || !CHIP_NODE_TYPES.has(node.type.name)) return null;
  return { start: pos, end: pos + node.nodeSize };
}

/**
 * Caret target for an arrow press that sits against a chip atom, or null when
 * the default handling applies.
 *
 * ProseMirror turns ArrowLeft/ArrowRight next to a selectable inline atom into
 * a NodeSelection. A NodeSelection has no text caret, so the press reads as
 * "the caret vanished" instead of "the chip is selected", and a second press is
 * needed to get past a chip. Stepping the caret across the whole atom keeps a
 * caret on screen and makes one press cross one chip. Shift-extension is left
 * to ProseMirror: it already builds a TextSelection over the atom, which the
 * `data-chip-selected` painting picks up.
 */
export function chipCaretStep(
  state: Pick<EditorState, "doc" | "selection">,
  direction: 1 | -1,
): TextSelection | null {
  const { selection, doc } = state;
  if (!(selection instanceof TextSelection) || !selection.empty) return null;
  const $head = selection.$head;
  // A caret inside a text node has text on that side, never an atom.
  if ($head.textOffset !== 0) return null;
  const node = direction < 0 ? $head.nodeBefore : $head.nodeAfter;
  if (node === null || !CHIP_NODE_TYPES.has(node.type.name)) return null;
  return TextSelection.create(doc, $head.pos + node.nodeSize * direction);
}

/** Applies `chipCaretStep`, reporting whether the arrow press was consumed. */
function moveCaretPastChip(editor: Editor, direction: 1 | -1): boolean {
  const next = chipCaretStep(editor.state, direction);
  if (next === null) return false;
  editor.view.dispatch(editor.state.tr.setSelection(next).scrollIntoView());
  return true;
}

/**
 * Selects exactly one chip as a TextSelection over the atom.
 *
 * NodeSelection puts `ProseMirror-hideselection` on the editor (caret-color:
 * transparent) and paints `ProseMirror-selectednode` on the React node-view
 * wrapper, not the visible chip — so the caret vanishes and the chip does not
 * highlight. A range over the atom keeps a caret at the head and lets
 * `data-chip-selected` paint the same wash as a drag.
 */
export function pinComposerChipSelection(
  view: Pick<EditorView, "dispatch" | "state">,
  nodePos: number,
  event: Pick<MouseEvent, "preventDefault">,
): boolean {
  const node = view.state.doc.nodeAt(nodePos);
  const end = nodePos + (node?.nodeSize ?? 1);
  view.dispatch(
    view.state.tr.setSelection(
      TextSelection.create(view.state.doc, nodePos, end),
    ),
  );
  event.preventDefault();
  return true;
}

/**
 * File and prompt chips are atoms: native `::selection` skips them, and a
 * React node-view decoration class would re-render on every mousemove. This
 * plugin snaps the dragged TextSelection onto any chip under the pointer and
 * paints `data-chip-selected` imperatively so highlight tracks the mouse.
 */
export const ComposerChipSelection = Extension.create({
  name: "composerChipSelection",

  addKeyboardShortcuts() {
    return {
      ArrowRight: ({ editor }) => moveCaretPastChip(editor, 1),
      ArrowLeft: ({ editor }) => moveCaretPastChip(editor, -1),
    };
  },

  addProseMirrorPlugins() {
    /** Document position where the current button-1 drag started. */
    let dragAnchor: number | null = null;
    let lastPointer: { left: number; top: number } | null = null;

    const clearDrag = () => {
      dragAnchor = null;
      lastPointer = null;
    };

    return [
      new Plugin({
        key: composerChipSelectionKey,
        view(editorView) {
          paintChipSelection(editorView);
          const onUp = () => clearDrag();
          window.addEventListener("mouseup", onUp, true);
          window.addEventListener("blur", onUp);
          return {
            update(nextView) {
              paintChipSelection(nextView);
            },
            destroy() {
              window.removeEventListener("mouseup", onUp, true);
              window.removeEventListener("blur", onUp);
              clearDrag();
            },
          };
        },
        props: {
          createSelectionBetween(view) {
            if (dragAnchor === null || lastPointer === null) return null;
            const hit = view.posAtCoords(lastPointer);
            if (
              hit === null ||
              chipRangeAt(view.state.doc, hit.inside) === null
            ) {
              return null;
            }
            return textSelectionForChipDrag(
              view.state.doc,
              dragAnchor,
              hit.pos,
              hit.inside,
            );
          },
          handleClickOn(view, _pos, node, nodePos, event) {
            if (!CHIP_NODE_TYPES.has(node.type.name)) return false;
            // Mentions render as bare renderHTML spans nobody listens to, so
            // their plain click pins the same range as Ctrl-click — otherwise
            // the consumed click shows nothing at all. File chips keep their
            // plain click for the host node view. Shift-click extends the
            // existing caret range and must keep what the drag/anchor logic
            // already built, so it never re-pins.
            const pinsOnPlainClick =
              !HOST_OWNED_PLAIN_CLICKS.has(node.type.name) && !event.shiftKey;
            if (event.ctrlKey || event.metaKey || pinsOnPlainClick) {
              return pinComposerChipSelection(view, nodePos, event);
            }
            // Consume the default atom click so ProseMirror cannot NodeSelect
            // (that hides the caret). Ctrl/double-click still pin a range;
            // a drag that ends on a chip keeps its TextSelection.
            return true;
          },
          handleDoubleClickOn(view, _pos, node, nodePos, event) {
            if (!CHIP_NODE_TYPES.has(node.type.name)) return false;
            return pinComposerChipSelection(view, nodePos, event);
          },
          handleDOMEvents: {
            mousedown(view, event) {
              if (event.button !== 0) return false;
              lastPointer = { left: event.clientX, top: event.clientY };
              if (event.shiftKey) {
                dragAnchor = view.state.selection.anchor;
                return false;
              }
              const hit = view.posAtCoords(lastPointer);
              dragAnchor = hit?.pos ?? null;
              return false;
            },
            mousemove(view, event) {
              if (event.buttons !== 1 || dragAnchor === null) return false;
              lastPointer = { left: event.clientX, top: event.clientY };
              const hit = view.posAtCoords(lastPointer);
              if (hit === null) return false;
              const overChip = chipRangeAt(view.state.doc, hit.inside) !== null;
              if (!overChip) return false;
              const next = textSelectionForChipDrag(
                view.state.doc,
                dragAnchor,
                hit.pos,
                hit.inside,
              );
              if (!next.eq(view.state.selection)) {
                view.dispatch(view.state.tr.setSelection(next));
              }
              // Claim the event so native selection cannot jump over the atom.
              return true;
            },
            /** Browser dblclick on adjacent user-select:none atoms selects them all. */
            dblclick(view, event) {
              const coords = view.posAtCoords({
                left: event.clientX,
                top: event.clientY,
              });
              if (coords === null) return false;
              const directPos = coords.inside >= 0 ? coords.inside : coords.pos;
              const direct = view.state.doc.nodeAt(directPos);
              if (direct !== null && CHIP_NODE_TYPES.has(direct.type.name)) {
                return pinComposerChipSelection(view, directPos, event);
              }
              const $pos = view.state.doc.resolve(coords.pos);
              if (
                $pos.nodeAfter !== null &&
                CHIP_NODE_TYPES.has($pos.nodeAfter.type.name)
              ) {
                return pinComposerChipSelection(view, $pos.pos, event);
              }
              if (
                $pos.nodeBefore !== null &&
                CHIP_NODE_TYPES.has($pos.nodeBefore.type.name)
              ) {
                return pinComposerChipSelection(
                  view,
                  $pos.pos - $pos.nodeBefore.nodeSize,
                  event,
                );
              }
              return false;
            },
          },
        },
      }),
    ];
  },
});

/**
 * Paints selected chips on the live DOM. Covers a TextSelection range and a
 * leftover NodeSelection so the wash still lands when some other path pins
 * the atom the old way.
 */
function paintChipSelection(view: EditorView): void {
  const root = view.dom;
  const { selection, doc } = view.state;
  const keep = new Set<Element>();
  const paintAt = (pos: number) => {
    const dom = view.nodeDOM(pos);
    const el = elementForChipDom(dom);
    if (el === null) return;
    el.setAttribute(CHIP_SELECTED_ATTR, "true");
    keep.add(el);
  };
  if (selection instanceof NodeSelection) {
    if (CHIP_NODE_TYPES.has(selection.node.type.name)) {
      paintAt(selection.from);
    }
  } else if (!selection.empty) {
    doc.nodesBetween(selection.from, selection.to, (node, pos) => {
      if (!CHIP_NODE_TYPES.has(node.type.name)) return;
      paintAt(pos);
    });
  }
  root.querySelectorAll(`[${CHIP_SELECTED_ATTR}]`).forEach((el) => {
    if (!keep.has(el)) el.removeAttribute(CHIP_SELECTED_ATTR);
  });
}

function elementForChipDom(dom: Node | null | undefined): HTMLElement | null {
  if (dom instanceof HTMLElement) return dom;
  if (dom?.parentElement instanceof HTMLElement) return dom.parentElement;
  return null;
}
