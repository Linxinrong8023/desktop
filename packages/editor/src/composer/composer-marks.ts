import {
  InputRule,
  markInputRule,
  markPasteRule,
  type InputRuleMatch,
} from "@tiptap/core";
import type { MarkType } from "@tiptap/pm/model";
import { Bold } from "@tiptap/extension-bold";
import { Code } from "@tiptap/extension-code";
import { Italic } from "@tiptap/extension-italic";
import { Strike } from "@tiptap/extension-strike";
import { Underline } from "@tiptap/extension-underline";

/**
 * Kit Bold/Italic/Strike wrap input rules with `(?:^|\\s)`, which is
 * Latin-centric: `你好**等等**` never converts. Keep the kit marks and
 * markInputRule helper; only the flanking pattern changes so CJK and
 * other non-space characters can sit against the delimiters. Opening
 * delimiters still reject a following space, closing ones a preceding
 * space (so `**d **` and `2 * 3 * 4` stay literal). `***both***` is a
 * stacked bold+italic rule so it does not steal a line that is only `***`.
 */
export const ComposerBold = Bold.extend({
  inclusive: false,

  addInputRules() {
    const italic = this.editor.schema.marks.italic;
    return [
      ...(italic === undefined
        ? []
        : [
            stackedMarkInputRule({
              find: boldItalicInputMatch,
              types: [this.type, italic],
            }),
          ]),
      markInputRule({
        find: /(?<!\*)(\*\*(?!\s)((?:[^*]+))(?<!\s)\*\*)(?!\*)$/,
        type: this.type,
      }),
      markInputRule({
        find: /(?<!_)(__(?!\s)((?:[^_]+))(?<!\s)__)(?!_)$/,
        type: this.type,
      }),
    ];
  },

  addPasteRules() {
    return [
      markPasteRule({
        find: /(?<!\*)\*\*(?!\s)((?:[^*]+))(?<!\s)\*\*(?!\*)/g,
        type: this.type,
      }),
      markPasteRule({
        find: /(?<!_)__(?!\s)((?:[^_]+))(?<!\s)__(?!_)/g,
        type: this.type,
      }),
    ];
  },
});

export const ComposerItalic = Italic.extend({
  inclusive: false,

  addInputRules() {
    return [
      markInputRule({
        find: /(?<!\*)(\*(?![*\s])((?:[^*]+))(?<!\s)\*)(?!\*)$/,
        type: this.type,
      }),
      markInputRule({
        find: /(?<![A-Za-z0-9_])(_(?![_\s])((?:[^_]+))(?<!\s)_)(?![A-Za-z0-9_])$/,
        type: this.type,
      }),
    ];
  },

  addPasteRules() {
    return [
      markPasteRule({
        find: /(?<!\*)\*(?![*\s])((?:[^*]+))(?<!\s)\*(?!\*)/g,
        type: this.type,
      }),
      markPasteRule({
        find: /(?<![A-Za-z0-9_])_(?![_\s])((?:[^_]+))(?<!\s)_(?![A-Za-z0-9_])/g,
        type: this.type,
      }),
    ];
  },
});

export const ComposerStrike = Strike.extend({
  inclusive: false,

  addInputRules() {
    return [
      markInputRule({
        find: /(~~(?!\s)((?:[^~]+))(?<!\s)~~)$/,
        type: this.type,
      }),
    ];
  },

  addPasteRules() {
    return [
      markPasteRule({
        find: /~~(?!\s)((?:[^~]+))(?<!\s)~~/g,
        type: this.type,
      }),
    ];
  },
});

export const ComposerCode = Code.extend({
  inclusive: false,
});

/**
 * Markdown has no underline delimiters; the kit shortcut is Mod-u.
 * Exclusive so typing after the mark is body text.
 */
export const ComposerUnderline = Underline.extend({
  inclusive: false,
});

/**
 * Closed `***bold-italic***` run. markInputRule only applies one mark, so
 * this is a small stacked variant used before the `**` / `*` rules.
 */
export function boldItalicInputMatch(text: string): InputRuleMatch | null {
  const match = /(?<!\*)(\*\*\*(?!\s)((?:[^*]+))(?<!\s)\*\*\*)(?!\*)$/.exec(
    text,
  );
  const inner = match?.[2];
  if (match === null || inner === undefined) {
    return null;
  }
  return {
    index: match.index,
    text: match[0],
    replaceWith: inner,
  };
}

function stackedMarkInputRule(config: {
  find: (text: string) => InputRuleMatch | null;
  types: MarkType[];
}): InputRule {
  return new InputRule({
    find: config.find,
    handler: ({ state, range, match }) => {
      const captureGroup = match[match.length - 1];
      const fullMatch = match[0];
      if (captureGroup === undefined || captureGroup.length === 0) {
        return null;
      }
      const { tr } = state;
      const startSpaces = fullMatch.search(/\S/);
      const textStart = range.from + fullMatch.indexOf(captureGroup);
      const textEnd = textStart + captureGroup.length;
      if (textEnd < range.to) {
        tr.delete(textEnd, range.to);
      }
      if (textStart > range.from) {
        tr.delete(range.from + startSpaces, textStart);
      }
      const from = range.from + startSpaces;
      const to = from + captureGroup.length;
      for (const type of config.types) {
        tr.addMark(from, to, type.create());
        tr.removeStoredMark(type);
      }
    },
  });
}
