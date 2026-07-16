import { Avatar, AvatarFallback, cn } from "@ora/ui";
import { IconMessageCircle } from "@tabler/icons-react";

type AvatarSize = "sm" | "default" | "lg";

const ICON_SIZE: Record<AvatarSize, string> = {
  sm: "size-3.5",
  default: "size-4",
  lg: "size-5",
};

interface OraMarkProps {
  size?: AvatarSize;
  className?: string;
}

/** The Ora brand mark: a primary-colored circle with a chat glyph. */
export function OraMark({ size = "default", className }: OraMarkProps) {
  return (
    <Avatar size={size} className={cn("rounded-lg", className)}>
      <AvatarFallback className="rounded-lg bg-primary text-primary-foreground">
        <IconMessageCircle className={ICON_SIZE[size]} />
      </AvatarFallback>
    </Avatar>
  );
}
