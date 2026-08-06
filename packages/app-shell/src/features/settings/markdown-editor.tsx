import { Markdown } from "@tiptap/markdown";
import { EditorContent, useEditor } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import { cn } from "@ora/ui";

interface MarkdownEditorProps {
  ariaLabel: string;
  value: string;
  onChange: (value: string) => void;
  disabled?: boolean;
  className?: string;
}

/** Edits Markdown as rich text while keeping Markdown as the persisted value. */
export function MarkdownEditor({ ariaLabel, value, onChange, disabled = false, className }: MarkdownEditorProps) {
  const editor = useEditor({
    extensions: [StarterKit, Markdown],
    content: value,
    contentType: "markdown",
    editable: !disabled,
    immediatelyRender: false,
    editorProps: {
      attributes: {
        "aria-label": ariaLabel,
        class: cn(
          "min-h-56 px-3 py-2 text-sm outline-none",
          "[&_h1]:mb-4 [&_h1]:mt-2 [&_h1]:text-3xl [&_h1]:font-bold",
          "[&_h2]:mb-3 [&_h2]:mt-2 [&_h2]:text-2xl [&_h2]:font-semibold",
          "[&_h3]:mb-2 [&_h3]:mt-2 [&_h3]:text-xl [&_h3]:font-semibold",
          "[&_p]:my-2 [&_p:first-child]:mt-0 [&_p:last-child]:mb-0",
          "[&_ul]:my-2 [&_ul]:list-disc [&_ul]:pl-6 [&_ol]:my-2 [&_ol]:list-decimal [&_ol]:pl-6",
          "[&_blockquote]:my-2 [&_blockquote]:border-l-2 [&_blockquote]:border-border [&_blockquote]:pl-4 [&_blockquote]:text-muted-foreground",
          "[&_pre]:my-2 [&_pre]:overflow-x-auto [&_pre]:rounded-md [&_pre]:bg-muted [&_pre]:p-3 [&_code]:rounded [&_code]:bg-muted [&_code]:px-1",
        ),
      },
    },
    onUpdate: ({ editor: updatedEditor }) => onChange(updatedEditor.getMarkdown().trimEnd()),
  });

  return (
    <div
      className={cn(
        "overflow-y-auto rounded-md border border-input bg-transparent shadow-xs transition-[color,box-shadow]",
        "focus-within:border-ring focus-within:ring-[3px] focus-within:ring-ring/50",
        disabled && "cursor-not-allowed opacity-50",
        className,
      )}
    >
      <EditorContent editor={editor} />
    </div>
  );
}
