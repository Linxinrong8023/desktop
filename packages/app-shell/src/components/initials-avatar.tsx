import { Avatar, AvatarFallback, cn } from "@ora/ui";
import { getInitials } from "../lib/avatar";

type AvatarSize = "sm" | "default" | "lg";

interface InitialsAvatarProps {
  name: string;
  size?: AvatarSize;
  className?: string;
}

/**
 * An initials avatar (e.g. "Eric Wang" -> "EW") rendered on top of the @ora/ui
 * Avatar shell. It uses the app's mid neutral `muted-foreground` surface - darker
 * than the muted chip but softer than near-black `primary` - and inverts with the
 * light/dark theme.
 */
export function InitialsAvatar({
  name,
  size = "default",
  className,
}: InitialsAvatarProps) {
  return (
    <Avatar size={size} className={className}>
      <AvatarFallback
        className={cn("bg-muted-foreground font-semibold text-muted")}
      >
        {getInitials(name)}
      </AvatarFallback>
    </Avatar>
  );
}
