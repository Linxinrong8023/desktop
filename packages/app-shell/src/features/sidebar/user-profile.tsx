import { IconChevronDown, IconLogout, IconSettings } from "@tabler/icons-react";
import {
  Button,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@ora/ui";
import { ColoredAvatar } from "../../components/colored-avatar";
import type { CurrentUser } from "../../lib/types";

interface UserProfileProps {
  user: CurrentUser;
  /** Renders only the avatar - used when the sidebar is collapsed. */
  compact?: boolean;
  onSignOut?: () => void;
}

/**
 * The sidebar footer user chip. Expanded it shows the colored avatar, name,
 * and email; collapsed it shows just the avatar. Both open a small account
 * menu (Settings / Log out).
 */
export function UserProfile({ user, compact = false, onSignOut }: UserProfileProps) {
  const trigger = compact ? (
    <Button variant="ghost" size="icon" aria-label={`${user.name} account`} className="rounded-full">
      <ColoredAvatar name={user.name} size="sm" />
    </Button>
  ) : (
    <Button
      variant="ghost"
      size="sm"
      aria-label={`${user.name} account`}
      className="h-auto w-full justify-start gap-2 px-1.5 py-1.5"
    >
      <ColoredAvatar name={user.name} size="sm" />
      <span className="flex min-w-0 flex-1 flex-col text-left">
        <span className="truncate text-sm font-semibold text-foreground">{user.name}</span>
        <span className="truncate text-xs text-muted-foreground">{user.email}</span>
      </span>
      <IconChevronDown className="size-4 shrink-0 text-muted-foreground" />
    </Button>
  );

  return (
    <DropdownMenu>
      <DropdownMenuTrigger render={trigger} />
      <DropdownMenuContent className="w-60" align="start" side="top">
        <DropdownMenuItem>
          <IconSettings />
          Settings
        </DropdownMenuItem>
        <DropdownMenuItem onClick={onSignOut}>
          <IconLogout />
          Log out
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
