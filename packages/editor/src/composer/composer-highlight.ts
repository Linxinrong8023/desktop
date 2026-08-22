import {
  markInputRule,
  markPasteRule,
  type InputRuleMatch,
  type PasteRuleMatch,
} from "@tiptap/core";
import Highlight from "@tiptap/extension-highlight";

/**
 * Closed `==highlight==` run at the caret. Kit Highlight wraps the whole
 * match in a capturing group, so markInputRule's last-group heuristic kept
 * the delimiters in the document (a chip around `==高亮==`). `replaceWith`
 * is the inner text, same pattern as ComposerCode.
 */
export function highlightInputMatch(text: string): InputRuleMatch | null {
  const match = /==(?!\s)([^=]+)(?<!\s)==$/.exec(text);
  const inner = match?.[1];
  if (match === null || inner === undefined) {
    return null;
  }
  return {
    index: match.index,
    text: match[0],
    replaceWith: inner,
  };
}

/**
 * Same inner-text replacement as typing, for a pasted `==highlight==` run.
 */
export function highlightPasteMatch(text: string): PasteRuleMatch[] {
  return [...text.matchAll(/==(?!\s)([^=]+)(?<!\s)==/g)].flatMap((match) => {
    const inner = match[1];
    if (inner === undefined) {
      return [];
    }
    return [
      {
        index: match.index,
        text: match[0],
        replaceWith: inner,
      },
    ];
  });
}

/**
 * Kit Highlight already understands `==`, but it requires a leading space and
 * would stay inclusive. Match adjacent text and exit the mark when typing on.
 */
export const ComposerHighlight = Highlight.extend({
  inclusive: false,
  exitable: true,

  addInputRules() {
    return [
      markInputRule({
        find: highlightInputMatch,
        type: this.type,
      }),
    ];
  },

  addPasteRules() {
    return [
      markPasteRule({
        find: highlightPasteMatch,
        type: this.type,
      }),
    ];
  },
}).configure({
  HTMLAttributes: { class: "composer-highlight" },
});
