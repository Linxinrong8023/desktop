import { useEffect, useRef, useState } from "react";
import { IconDotsVertical, IconMessage, IconPencil, IconTrash } from "@tabler/icons-react";
import {
  Button,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
  Input,
  cn,
} from "@ora/ui";
import type { Conversation } from "../../lib/types";

interface ConversationItemProps {
  conversation: Conversation;
  active: boolean;
  onSelect: () => void;
  onRename: (title: string) => void;
  onRemove: () => void;
}

/**
 * A single sidebar conversation row: title, active highlight, and a hover
 * affordance to rename or delete. Double-clicking the title enters rename mode.
 */
export function ConversationItem({
  conversation,
  active,
  onSelect,
  onRename,
  onRemove,
}: ConversationItemProps) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(conversation.title);
  const inputRef = useRef<HTMLInputElement>(null);

  // Focus and select the title text when entering rename mode.
  useEffect(() => {
    if (editing) inputRef.current?.select();
  }, [editing]);

  const startEdit = () => {
    setDraft(conversation.title);
    setEditing(true);
  };

  const commit = () => {
    const next = draft.trim();
    if (next && next !== conversation.title) onRename(next);
    else setDraft(conversation.title);
    setEditing(false);
  };

  const cancel = () => {
    setDraft(conversation.title);
    setEditing(false);
  };

  if (editing) {
    return (
      <Input
        ref={inputRef}
        aria-label="Rename conversation"
        value={draft}
        onChange={(event) => setDraft(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            commit();
          } else if (event.key === "Escape") {
            event.preventDefault();
            cancel();
          }
        }}
        onBlur={commit}
      />
    );
  }

  return (
    <div className="group relative">
      <button
        type="button"
        onClick={onSelect}
        onDoubleClick={startEdit}
        aria-current={active ? "true" : undefined}
        className={cn(
          "flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-left text-sm font-medium transition duration-100 ease-linear",
          active
            ? "bg-accent text-accent-foreground"
            : "text-muted-foreground hover:bg-accent/60 hover:text-foreground",
        )}
      >
        <IconMessage className="size-[18px] shrink-0 stroke-[1.75px] text-muted-foreground" />
        <span className="flex-1 truncate">{conversation.title}</span>
      </button>

      <div className={cn("absolute right-1 top-1/2 -translate-y-1/2", !active && "opacity-0 group-hover:opacity-100")}>
        <DropdownMenu>
          <DropdownMenuTrigger
            render={
              <Button variant="ghost" size="icon-xs" aria-label="Conversation options">
                <IconDotsVertical className="size-4 text-muted-foreground" />
              </Button>
            }
          />
          <DropdownMenuContent className="w-48" align="end">
            <DropdownMenuItem onClick={startEdit}>
              <IconPencil />
              Rename
            </DropdownMenuItem>
            <DropdownMenuItem variant="destructive" onClick={onRemove}>
              <IconTrash />
              Delete
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </div>
  );
}
