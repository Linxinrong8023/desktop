import { Extension, textblockTypeInputRule } from "@tiptap/core";
import { TextSelection } from "@tiptap/pm/state";
import type { EditorView } from "@tiptap/pm/view";

/**
 * GFM info string up to the first space: `C++`, `c#`, `objective-c`.
 * Conversion waits for Shift+Enter or a trailing space so ```C++ is typeable.
 */
const FENCE_OPENER = /^```([^\s`]*)$/;
const FENCE_WITH_SPACE = /^```([^\s`]*) $/;

export function parseFenceOpener(
  text: string,
): { language: string | null } | null {
  const match = FENCE_OPENER.exec(text);
  if (match === null) {
    return null;
  }
  return { language: match[1] === "" ? null : match[1] };
}

/**
 * Turns a paragraph that is only a Markdown fence opener into a code block.
 * Called from Shift+Enter (newline) and Enter (so send does not swallow it).
 */
export function convertMarkdownFenceOpener(view: EditorView): boolean {
  const { state } = view;
  const { $from } = state.selection;
  if ($from.parent.type.name !== "paragraph") {
    return false;
  }
  const parsed = parseFenceOpener($from.parent.textContent);
  if (parsed === null) {
    return false;
  }
  const codeBlock = state.schema.nodes.codeBlock;
  if (codeBlock === undefined) {
    return false;
  }
  const blockPos = $from.before($from.depth);
  const tr = state.tr.delete($from.start(), $from.end());
  const mapped = tr.mapping.map(blockPos);
  tr.setBlockType(mapped, mapped + 2, codeBlock, {
    language: parsed.language,
  });
  view.dispatch(tr.scrollIntoView());
  return true;
}

/**
 * Enter inside a fence leaves code. Shift+Enter is the newline everywhere
 * else in the composer, so it keeps that meaning inside the fence too.
 */
export function handleComposerCodeEnter(view: EditorView): boolean {
  const { state } = view;
  const { $from } = state.selection;
  if ($from.parent.type.name !== "codeBlock") {
    return false;
  }
  const text = $from.parent.textContent;
  if (text.length === 0) {
    return collapseEmptyCodeBlock(view);
  }
  const offset = $from.parentOffset;
  const before = text.slice(0, offset);
  const after = text.slice(offset);
  const lineStart = before.lastIndexOf("\n") + 1;
  const nextBreak = after.indexOf("\n");
  const lineEnd = nextBreak === -1 ? text.length : offset + nextBreak;
  const currentLine = text.slice(lineStart, lineEnd);
  if (currentLine === "```" && lineEnd === text.length) {
    return closeFenceAndExit(view, lineStart);
  }
  return exitComposerCodeBlock(view, { trimTrailingNewline: true });
}

/**
 * Leaves a code fence: empty fences become a paragraph; otherwise a paragraph
 * is inserted after the block so the user can keep typing body text.
 */
export function exitComposerCodeBlock(
  view: EditorView,
  options: { trimTrailingNewline?: boolean } = {},
): boolean {
  const { state } = view;
  const { $from } = state.selection;
  if ($from.parent.type.name !== "codeBlock") {
    return false;
  }
  if ($from.parent.textContent.length === 0) {
    return collapseEmptyCodeBlock(view);
  }
  const paragraph = state.schema.nodes.paragraph;
  if (paragraph === undefined) {
    return false;
  }
  const tr = state.tr;
  if (
    options.trimTrailingNewline === true &&
    $from.parent.textContent.endsWith("\n")
  ) {
    const newlinePos = $from.start() + $from.parent.textContent.length - 1;
    tr.delete(newlinePos, newlinePos + 1);
  }
  const mapped = tr.mapping.map($from.pos);
  const $mapped = tr.doc.resolve(mapped);
  const after = $mapped.after($mapped.depth);
  tr.insert(after, paragraph.create());
  tr.setSelection(TextSelection.create(tr.doc, after + 1));
  view.dispatch(tr.scrollIntoView());
  return true;
}

/**
 * Backspace in an empty fence restores a normal paragraph instead of deleting
 * the whole composer block. Wired through `ComposerMarkdownRevert` so the
 * shared Backspace shortcut can chain fence collapse before mark revert.
 */
export function handleComposerCodeBackspace(view: EditorView): boolean {
  const { $from } = view.state.selection;
  if ($from.parent.type.name !== "codeBlock") {
    return false;
  }
  if ($from.parentOffset !== 0 || $from.parent.content.size !== 0) {
    return false;
  }
  return collapseEmptyCodeBlock(view);
}

function collapseEmptyCodeBlock(view: EditorView): boolean {
  const { state } = view;
  const { $from } = state.selection;
  const paragraph = state.schema.nodes.paragraph;
  if (paragraph === undefined || $from.parent.type.name !== "codeBlock") {
    return false;
  }
  const from = $from.before($from.depth);
  view.dispatch(
    state.tr
      .setBlockType(from, from + $from.parent.nodeSize, paragraph)
      .scrollIntoView(),
  );
  return true;
}

function closeFenceAndExit(view: EditorView, lineStartOffset: number): boolean {
  const { state } = view;
  const { $from } = state.selection;
  const paragraph = state.schema.nodes.paragraph;
  if (paragraph === undefined) {
    return false;
  }
  const start = $from.start();
  const textLen = $from.parent.textContent.length;
  const deleteFrom =
    lineStartOffset === 0 ? start : start + lineStartOffset - 1;
  const tr = state.tr.delete(deleteFrom, start + textLen);
  const mappedStart = tr.mapping.map(start, -1);
  const safePos = Math.min(Math.max(mappedStart, 1), tr.doc.content.size);
  const $pos = tr.doc.resolve(safePos);
  let depth = $pos.depth;
  while (depth > 0 && $pos.node(depth).type.name !== "codeBlock") {
    depth -= 1;
  }
  if (depth === 0) {
    view.dispatch(tr.scrollIntoView());
    return true;
  }
  const codeNode = $pos.node(depth);
  if (codeNode.textContent.length === 0) {
    const blockFrom = $pos.before(depth);
    tr.setBlockType(blockFrom, blockFrom + codeNode.nodeSize, paragraph);
    view.dispatch(tr.scrollIntoView());
    return true;
  }
  const after = $pos.after(depth);
  tr.insert(after, paragraph.create());
  tr.setSelection(TextSelection.create(tr.doc, after + 1));
  view.dispatch(tr.scrollIntoView());
  return true;
}

/**
 * Converts ```lang␠ into a fenced block. Bare ``` waits for Shift+Enter or a
 * trailing space so the user can still type a language such as C++.
 */
export const ComposerCodeFence = Extension.create({
  name: "composerCodeFence",
  priority: 1100,

  addKeyboardShortcuts() {
    return {
      Enter: () => handleComposerCodeEnter(this.editor.view),
    };
  },

  addInputRules() {
    const type = this.editor.schema.nodes.codeBlock;
    if (type === undefined) {
      return [];
    }
    return [
      textblockTypeInputRule({
        find: FENCE_WITH_SPACE,
        type,
        getAttributes: (match) => ({
          language: match[1] === "" ? null : match[1],
        }),
      }),
    ];
  },
});
