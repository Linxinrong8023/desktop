import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { IconDownload, IconLoader2, IconRefresh } from "@tabler/icons-react";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  Button,
  cn,
} from "@ora/ui";
import { appVersion } from "../../lib/app-version";
import { useOptionalPlatform } from "../../platform/use-platform";
import type { DesktopUpdateStatus } from "../../platform/types";

/** Formats a byte count the way the download progress reads in both locales. */
function formatMegabytes(bytes: number): string {
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/** Renders the plain version stamp used by hosts that cannot update themselves. */
function VersionStamp() {
  const { t } = useTranslation();
  return (
    <p className="px-2 pb-1 text-xs leading-5 text-muted-foreground">
      {t("update.currentVersion", { version: appVersion })}
    </p>
  );
}

/** Renders the version stamp and turns it into the install action once a package is ready. */
export function DesktopUpdateControl() {
  const { t } = useTranslation();
  const platform = useOptionalPlatform();
  const updates = platform?.updates;
  const [status, setStatus] = useState<DesktopUpdateStatus>({
    kind: "current",
  });
  const [confirming, setConfirming] = useState(false);

  useEffect(() => {
    if (updates === undefined) return;
    let active = true;
    void updates.getStatus().then((next) => {
      if (active) setStatus(next);
    });
    let unsubscribe: (() => void) | undefined;
    void updates
      .onStatus((next) => {
        if (active) setStatus(next);
      })
      .then((stop) => {
        unsubscribe = stop;
        if (!active) stop();
      });
    return () => {
      active = false;
      unsubscribe?.();
    };
  }, [updates]);

  // The web host has no updater, but the settings sidebar still stamps the running version.
  if (updates === undefined) return <VersionStamp />;

  // Only a downloaded package is installable; a manual update still deserves the badge so the
  // tooltip can explain which channel to use instead.
  const installableVersion = status.kind === "ready" ? status.version : null;
  const notified =
    installableVersion !== null || status.kind === "manual_update";
  const busy =
    status.kind === "checking" ||
    status.kind === "downloading" ||
    status.kind === "installing";

  return (
    <>
      <div className="flex items-center gap-1">
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="relative h-7 flex-1 justify-start px-2 text-xs text-muted-foreground"
          disabled={busy || installableVersion === null}
          title={describeStatus(status, t)}
          aria-label={describeStatus(status, t)}
          onClick={() => setConfirming(true)}
        >
          {busy ? (
            <IconLoader2 className="mr-1.5 size-3.5 animate-spin" />
          ) : (
            <IconDownload className="mr-1.5 size-3.5" />
          )}
          <span className="truncate">
            {t("update.currentVersion", { version: appVersion })}
          </span>
          {notified && (
            <span
              data-testid="desktop-update-badge"
              className="absolute right-2 top-1/2 size-2 -translate-y-1/2 rounded-full bg-destructive"
            />
          )}
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="size-7 shrink-0 text-muted-foreground"
          disabled={busy}
          title={t("update.checkNow")}
          aria-label={t("update.checkNow")}
          onClick={() => void updates.check()}
        >
          <IconRefresh
            className={cn(
              "size-3.5",
              status.kind === "checking" && "animate-spin",
            )}
          />
        </Button>
      </div>
      <AlertDialog open={confirming} onOpenChange={setConfirming}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              {t("update.confirmTitle", { version: installableVersion ?? "" })}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {t("update.confirmDescription", {
                currentVersion: appVersion,
                version: installableVersion ?? "",
              })}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t("common.cancel")}</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => {
                setConfirming(false);
                void updates.install();
              }}
            >
              {t("update.confirmAction")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}

/** Renders the status as the one line shown in the tooltip and to assistive technology. */
function describeStatus(
  status: DesktopUpdateStatus,
  t: (key: string, options?: Record<string, unknown>) => string,
): string {
  switch (status.kind) {
    case "current":
      return t("update.currentVersion", { version: appVersion });
    case "checking":
      return t("update.checking");
    case "downloading":
      return status.total === null
        ? t("update.downloadingUnknownSize", { version: status.version })
        : t("update.downloading", {
            version: status.version,
            progress: `${formatMegabytes(status.downloaded)} / ${formatMegabytes(status.total)}`,
          });
    case "ready":
      return t("update.ready", { version: status.version });
    case "manual_update":
      return t(`update.manual.${status.reason}`, { version: status.version });
    case "installing":
      return t("update.installing", { version: status.version });
    case "failed":
      return t("update.failed", { message: status.message });
  }
}
