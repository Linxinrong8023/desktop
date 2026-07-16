import { useMemo, useState } from "react";
import { IconMenu2, IconSearch, IconEdit } from "@tabler/icons-react";
import { Input } from "@ora/ui";
import { IconButton } from "../../components/icon-button";
import { ConversationItem } from "./conversation-item";
import { UserProfile } from "./user-profile";
import { groupConversationsByDate } from "../../lib/grouping";
import type { Conversation, CurrentUser } from "../../lib/types";

interface SidebarProps {
  user: CurrentUser;
  conversations: Conversation[];
  activeId: string | null;
  collapsed: boolean;
  onToggleCollapsed: () => void;
  onNewChat: () => void;
  onSelectConversation: (id: string) => void;
  onRenameConversation: (id: string, title: string) => void;
  onRemoveConversation: (id: string) => void;
  onSignOut: () => void;
}

/** The collapsible left rail: new chat, IconSearch, date-grouped history, user footer. */
export function Sidebar({
  user,
  conversations,
  activeId,
  collapsed,
  onToggleCollapsed,
  onNewChat,
  onSelectConversation,
  onRenameConversation,
  onRemoveConversation,
  onSignOut,
}: SidebarProps) {
  const [query, setQuery] = useState("");

  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return conversations;
    return conversations.filter((c) => c.title.toLowerCase().includes(needle));
  }, [conversations, query]);

  const groups = useMemo(() => groupConversationsByDate(filtered, Date.now()), [filtered]);

  if (collapsed) {
    return (
      <aside className="flex h-dvh w-16 shrink-0 flex-col items-center gap-2 border-r border-border bg-muted/40 py-2">
        <IconButton icon={IconEdit} label="New chat" onClick={onNewChat} />
        <IconButton icon={IconMenu2} label="Open sidebar" onClick={onToggleCollapsed} />
        <div className="flex-1" />
        <UserProfile user={user} compact onSignOut={onSignOut} />
      </aside>
    );
  }

  return (
    <aside className="flex h-dvh w-72 shrink-0 flex-col border-r border-border bg-muted/40">
      <div className="flex items-center justify-between px-2 pt-2">
        <IconButton icon={IconEdit} label="New chat" onClick={onNewChat} />
        <IconButton icon={IconMenu2} label="Collapse sidebar" onClick={onToggleCollapsed} />
      </div>

      <div className="px-2 pt-2">
        <div className="relative">
          <IconSearch className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="Search conversations"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            className="pl-8"
          />
        </div>
      </div>

      <nav className="scrollbar-hide mt-1 flex-1 overflow-y-auto px-2 py-1">
        {groups.length === 0 ? (
          <p className="px-2 py-6 text-center text-sm text-muted-foreground">No conversations found.</p>
        ) : (
          groups.map((group) => (
            <section key={group.label} className="mb-1">
              <p className="px-2 py-1 text-xs font-semibold text-muted-foreground">{group.label}</p>
              <div className="space-y-px">
                {group.conversations.map((conversation) => (
                  <ConversationItem
                    key={conversation.id}
                    conversation={conversation}
                    active={conversation.id === activeId}
                    onSelect={() => onSelectConversation(conversation.id)}
                    onRename={(title) => onRenameConversation(conversation.id, title)}
                    onRemove={() => onRemoveConversation(conversation.id)}
                  />
                ))}
              </div>
            </section>
          ))
        )}
      </nav>

      <div className="border-t border-border p-2">
        <UserProfile user={user} onSignOut={onSignOut} />
      </div>
    </aside>
  );
}
