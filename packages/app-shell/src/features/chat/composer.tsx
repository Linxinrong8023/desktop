import { useEffect, useRef, useState } from "react";
import type { KeyboardEvent } from "react";
import { IconArrowUp } from "@tabler/icons-react";
import { Button, Textarea } from "@ora/ui";

interface ComposerProps {
  onSend: (text: string) => void;
  isResponding: boolean;
  placeholder?: string;
  autoFocus?: boolean;
}

/**
 * The chat composer: a rounded input shell wrapping the @ora/ui Textarea with
 * an inline send button. Enter sends, Shift+Enter inserts a newline, and the
 * textarea auto-grows up to a max height.
 */
export function Composer({
  onSend,
  isResponding,
  placeholder = "Message Ora...",
  autoFocus = false,
}: ComposerProps) {
  const [value, setValue] = useState("");
  const textAreaRef = useRef<HTMLTextAreaElement>(null);

  const canSend = value.trim().length > 0 && !isResponding;

  const submit = () => {
    const text = value.trim();
    if (!text || isResponding) return;
    onSend(text);
    setValue("");
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
      event.preventDefault();
      submit();
    }
  };

  // Auto-grow the textarea to fit its content, capped at a comfortable max.
  useEffect(() => {
    const el = textAreaRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${Math.min(el.scrollHeight, 200)}px`;
  }, [value]);

  return (
    <div className="flex flex-col rounded-2xl border border-border bg-background p-2 shadow-xs transition focus-within:ring-2 focus-within:ring-ring">
      <Textarea
        ref={textAreaRef}
        autoFocus={autoFocus}
        placeholder={placeholder}
        value={value}
        onChange={(event) => setValue(event.target.value)}
        onKeyDown={handleKeyDown}
        className="min-h-[28px] max-h-[200px] resize-none rounded-none border-0 bg-transparent px-2 py-1.5 shadow-none focus-visible:ring-0"
      />
      <div className="flex items-center justify-between pt-1">
        <p className="px-2 text-xs text-muted-foreground">
          Enter to send / <span className="text-foreground/70">Shift+Enter for newline</span>
        </p>
        <Button
          size="icon-sm"
          aria-label="Send message"
          disabled={!canSend}
          onClick={submit}
          className="rounded-full"
        >
          <IconArrowUp className="size-[18px]" />
        </Button>
      </div>
    </div>
  );
}
