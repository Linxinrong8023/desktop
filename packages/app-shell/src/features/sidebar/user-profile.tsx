import { IconChevronDown, IconLanguage, IconLogout } from "@tabler/icons-react";
import {
  Button,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@ora/ui";
import { useTranslation } from "react-i18next";
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
 * menu (language / log out).
 */
export function UserProfile({ user, compact = false, onSignOut }: UserProfileProps) {
  const { i18n, t } = useTranslation();
  const locale = i18n.resolvedLanguage === "en-US" ? "en-US" : "zh-CN";
  const accountLabel = t("account.label", { name: user.name });
  const trigger = compact ? (
    <Button variant="ghost" size="icon" aria-label={accountLabel} className="rounded-full">
      <ColoredAvatar name={user.name} size="sm" />
    </Button>
  ) : (
    <Button
      variant="ghost"
      size="sm"
      aria-label={accountLabel}
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
        <DropdownMenuItem onClick={() => void i18n.changeLanguage(locale === "zh-CN" ? "en-US" : "zh-CN")}>
          <IconLanguage />
          {t("account.language")}: {locale === "zh-CN" ? t("account.switchEnglish") : t("account.switchChinese")}
        </DropdownMenuItem>
        <DropdownMenuItem onClick={onSignOut}>
          <IconLogout />
          {t("account.logout")}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
