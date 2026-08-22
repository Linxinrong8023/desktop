import { Editor } from "@tiptap/core";
import { TextSelection } from "@tiptap/pm/state";
import { afterEach, describe, expect, it } from "vitest";
import {
  createComposerExtensions,
  handleComposerMarkdownBackspace,
} from "@ora/editor/composer";

const editors: Editor[] = [];

function leftoverEditor(text: string): Editor {
  const editor = new Editor({
    extensions: createComposerExtensions(),
    content: {
      type: "doc",
      content: [
        {
          type: "paragraph",
          content: [{ type: "text", text }],
        },
      ],
    },
  });
  editors.push(editor);
  return editor;
}

function multiParagraphEditor(lines: string[]): Editor {
  const editor = new Editor({
    extensions: createComposerExtensions(),
    content: {
      type: "doc",
      content: lines.map((text) =>
        text.length === 0
          ? { type: "paragraph" }
          : {
              type: "paragraph",
              content: [{ type: "text", text }],
            },
      ),
    },
  });
  editors.push(editor);
  return editor;
}

/** Places the caret at the start or end of a top-level paragraph. */
function setCaretInParagraph(
  editor: Editor,
  paragraphIndex: number,
  where: "start" | "end",
): void {
  let current = 0;
  let target = 1;
  editor.state.doc.forEach((node, pos) => {
    if (current === paragraphIndex) {
      target = where === "start" ? pos + 1 : pos + 1 + node.content.size;
    }
    current += 1;
  });
  editor.chain().setTextSelection(target).focus().run();
}

/** Trailing space is the confirm keystroke for leftover Markdown source. */
function confirmWithSpace(editor: Editor): void {
  editor.commands.focus("end");
  editor.view.dispatch(editor.state.tr.insertText(" "));
}

afterEach(() => {
  for (const editor of editors.splice(0)) {
    editor.destroy();
  }
});

const LEFTOVER_CASES: Array<{
  source: string;
  html: RegExp;
  forbidden: string;
}> = [
  {
    source: "**bold**",
    html: /<(strong|b)>bold<\/(strong|b)>/,
    forbidden: "**",
  },
  { source: "==a==", html: /<mark[^>]*>a<\/mark>/, forbidden: "==" },
  { source: "~~out~~", html: /<(s|del)>out<\/(s|del)>/, forbidden: "~~" },
  { source: "*em*", html: /<(em|i)>em<\/(em|i)>/, forbidden: "*" },
  { source: "`code`", html: /<code>code<\/code>/, forbidden: "`" },
  {
    source: "***both***",
    html: /<(strong|b)><(em|i)>both<\/(em|i)><\/(strong|b)>/,
    forbidden: "*",
  },
  {
    source: "__bold__",
    html: /<(strong|b)>bold<\/(strong|b)>/,
    forbidden: "__",
  },
  { source: "_em_", html: /<(em|i)>em<\/(em|i)>/, forbidden: "_" },
  {
    source: "[Docs](https://example.com)",
    html: /<a [^>]*href="https:\/\/example.com"/,
    forbidden: "](",
  },
  { source: "# Title", html: /<h1>Title\s*<\/h1>/, forbidden: "#" },
  { source: "> quote", html: /<blockquote>/, forbidden: "&gt;" },
];

const PREFIX_CASES: Array<{
  leftover: string;
  opener: string;
  html: RegExp;
  forbidden: string;
}> = [
  {
    leftover: "bold**",
    opener: "**",
    html: /<(strong|b)>bold<\/(strong|b)>/,
    forbidden: "**",
  },
  {
    leftover: "a==",
    opener: "==",
    html: /<mark[^>]*>a<\/mark>/,
    forbidden: "==",
  },
  {
    leftover: "out~~",
    opener: "~~",
    html: /<(s|del)>out<\/(s|del)>/,
    forbidden: "~~",
  },
  {
    leftover: "em*",
    opener: "*",
    html: /<(em|i)>em<\/(em|i)>/,
    forbidden: "*",
  },
  {
    leftover: "code`",
    opener: "`",
    html: /<code>code<\/code>/,
    forbidden: "`",
  },
  {
    leftover: "both***",
    opener: "***",
    html: /<(strong|b)><(em|i)>both<\/(em|i)><\/(strong|b)>/,
    forbidden: "*",
  },
  {
    leftover: "bold__",
    opener: "__",
    html: /<(strong|b)>bold<\/(strong|b)>/,
    forbidden: "__",
  },
  {
    leftover: "em_",
    opener: "_",
    html: /<(em|i)>em<\/(em|i)>/,
    forbidden: "_",
  },
  {
    leftover: "Docs](https://example.com)",
    opener: "[",
    html: /<a [^>]*href="https:\/\/example.com"/,
    forbidden: "](",
  },
];

describe("composer markdown live conversion", () => {
  it.each(LEFTOVER_CASES)(
    "converts leftover $source when a space is typed after it",
    ({ source, html, forbidden }) => {
      const editor = leftoverEditor(source);
      confirmWithSpace(editor);
      expect(editor.getHTML()).toMatch(html);
      expect(editor.getHTML()).not.toContain(forbidden);
    },
  );

  it.each(PREFIX_CASES)(
    "waits for a trailing space after wrapping leftover $leftover with $opener",
    ({ leftover, opener, html, forbidden }) => {
      const editor = leftoverEditor(leftover);
      editor.commands.focus("start");
      editor.view.dispatch(editor.state.tr.insertText(opener));
      expect(editor.getHTML()).not.toMatch(html);
      confirmWithSpace(editor);
      expect(editor.getHTML()).toMatch(html);
      expect(editor.getHTML()).not.toContain(forbidden);
    },
  );

  it("converts leftover adjacent bold/italic when a space is typed after them", () => {
    const editor = leftoverEditor("**加粗***倾斜*");
    confirmWithSpace(editor);
    expect(editor.getHTML()).toMatch(/<(strong|b)>加粗<\/(strong|b)>/);
    expect(editor.getHTML()).toMatch(/<(em|i)>倾斜<\/(em|i)>/);
    expect(editor.getHTML()).not.toContain("*");
  });

  it("converts leftover highlight when the line is split", () => {
    const editor = leftoverEditor("==a==");
    editor.commands.focus("end");
    editor.commands.splitBlock();
    expect(editor.getHTML()).toMatch(/<mark[^>]*>a<\/mark>/);
    expect(editor.getHTML()).not.toContain("==");
  });

  it("converts leftover marks when the line is split", () => {
    const editor = leftoverEditor("**bold**");
    editor.commands.focus("end");
    editor.commands.splitBlock();
    expect(editor.getHTML()).toMatch(/<(strong|b)>bold<\/(strong|b)>/);
    expect(editor.getHTML()).not.toContain("**");
  });

  it("does not convert a pending leftover when another line is confirmed", () => {
    const editor = multiParagraphEditor(["==pending==", "**done**"]);
    setCaretInParagraph(editor, 1, "end");
    editor.view.dispatch(editor.state.tr.insertText(" "));
    expect(editor.getHTML()).toContain("==pending==");
    expect(editor.getHTML()).toMatch(/<(strong|b)>done<\/(strong|b)>/);
    expect(editor.getHTML()).not.toMatch(/<mark[^>]*>pending<\/mark>/);
  });

  it("does not convert leftover when an empty line is inserted above it", () => {
    const editor = leftoverEditor("==a==");
    setCaretInParagraph(editor, 0, "start");
    const { state } = editor;
    const paragraph = state.schema.nodes.paragraph;
    if (paragraph === undefined) {
      throw new Error("expected paragraph node");
    }
    const insertPos = state.selection.$from.before();
    const tr = state.tr.insert(insertPos, paragraph.create());
    tr.setSelection(TextSelection.create(tr.doc, insertPos + 1));
    editor.view.dispatch(tr);
    expect(editor.getHTML()).toContain("==a==");
    expect(editor.getHTML()).not.toMatch(/<mark[^>]*>a<\/mark>/);
  });

  it.each([
    { source: "**bold**", restored: "**bold**", rendered: /<(strong|b)>bold/ },
    { source: "==a==", restored: "==a==", rendered: /<mark[^>]*>a<\/mark>/ },
    { source: "~~out~~", restored: "~~out~~", rendered: /<(s|del)>out/ },
    { source: "*em*", restored: "*em*", rendered: /<(em|i)>em/ },
    { source: "`code`", restored: "`code`", rendered: /<code>code<\/code>/ },
  ])(
    "Backspace after confirming $source restores Markdown source",
    ({ source, restored, rendered }) => {
      const editor = leftoverEditor(source);
      confirmWithSpace(editor);
      expect(editor.getHTML()).toMatch(rendered);
      editor.commands.focus("end");
      expect(handleComposerMarkdownBackspace(editor.view)).toBe(true);
      expect(editor.getText()).toBe(restored);
      expect(editor.getHTML()).not.toMatch(rendered);
    },
  );

  it("does not intercept Backspace on plain text", () => {
    const editor = leftoverEditor("hello");
    editor.commands.focus("end");
    expect(handleComposerMarkdownBackspace(editor.view)).toBe(false);
    expect(editor.getText()).toBe("hello");
  });

  it("does not revert when plain text was typed after a converted mark", () => {
    const editor = leftoverEditor("**bold**");
    confirmWithSpace(editor);
    editor.commands.focus("end");
    editor.view.dispatch(editor.state.tr.insertText(" more"));
    expect(handleComposerMarkdownBackspace(editor.view)).toBe(false);
    expect(editor.getHTML()).toMatch(/<(strong|b)>bold<\/(strong|b)>/);
    expect(editor.getText()).toMatch(/more$/);
  });

  it("does not revert a heading back to # source on Backspace", () => {
    const editor = new Editor({
      extensions: createComposerExtensions(),
      content: {
        type: "doc",
        content: [
          {
            type: "heading",
            attrs: { level: 1 },
            content: [{ type: "text", text: "Title" }],
          },
        ],
      },
    });
    editors.push(editor);
    editor.commands.focus("end");
    expect(handleComposerMarkdownBackspace(editor.view)).toBe(false);
    expect(editor.getHTML()).toMatch(/<h1>Title<\/h1>/);
  });

  it("does not revert a mark run that includes underline", () => {
    const editor = new Editor({
      extensions: createComposerExtensions(),
      content: {
        type: "doc",
        content: [
          {
            type: "paragraph",
            content: [
              {
                type: "text",
                text: "hi",
                marks: [{ type: "bold" }, { type: "underline" }],
              },
            ],
          },
        ],
      },
    });
    editors.push(editor);
    editor.commands.focus("end");
    expect(handleComposerMarkdownBackspace(editor.view)).toBe(false);
    expect(editor.getHTML()).toMatch(/<(strong|b)>/);
    expect(editor.getHTML()).toMatch(/<u>/);
  });

  it("Backspace restores adjacent bold/italic runs together", () => {
    const editor = leftoverEditor("**加粗***倾斜*");
    confirmWithSpace(editor);
    expect(editor.getHTML()).toMatch(/<(strong|b)>加粗<\/(strong|b)>/);
    expect(editor.getHTML()).toMatch(/<(em|i)>倾斜<\/(em|i)>/);
    editor.commands.focus("end");
    expect(handleComposerMarkdownBackspace(editor.view)).toBe(true);
    expect(editor.getText()).toBe("**加粗***倾斜*");
    expect(editor.getHTML()).not.toMatch(/<(strong|b)>/);
    expect(editor.getHTML()).not.toMatch(/<(em|i)>/);
  });

  it("does not wake a stuck leftover on an unfocused line while typing elsewhere", () => {
    const editor = multiParagraphEditor(["# Title ", "hello"]);
    setCaretInParagraph(editor, 1, "end");
    editor.view.dispatch(editor.state.tr.insertText("!"));
    expect(editor.getHTML()).toContain("# Title");
    expect(editor.getHTML()).not.toMatch(/<h1>/);
    expect(editor.getText()).toContain("hello!");
  });
});
