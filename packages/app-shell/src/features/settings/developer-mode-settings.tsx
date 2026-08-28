import { Button, Switch } from "@ora/ui";
import { useTranslation } from "react-i18next";
import type { DeveloperModeController } from "../../state/hooks/use-developer-mode";

/** Presents the developer-mode switch inside the always-reachable Developer options page. */
export function DeveloperModeSettings({
  controller,
}: {
  controller: DeveloperModeController;
}) {
  const { t } = useTranslation();
  const {
    state,
    isLoading,
    loadError,
    isSaving,
    updateError,
    submitEnabled,
    retry,
  } = controller;

  return (
    <section className="border-y border-border py-4">
      <div className="flex items-center gap-4">
        <div className="min-w-0 flex-1">
          <p className="text-sm font-medium">
            {t("settings.developer.developerMode")}
          </p>
          <p className="mt-1 text-xs leading-5 text-muted-foreground">
            {t("settings.developer.developerModeDescription")}
          </p>
        </div>
        <Switch
          aria-label={t("settings.developer.developerMode")}
          checked={state?.enabled ?? false}
          disabled={isLoading || loadError !== null || isSaving}
          onCheckedChange={submitEnabled}
        />
      </div>
      {isLoading && (
        <p role="status" className="mt-2 text-xs text-muted-foreground">
          {t("settings.developer.developerModeLoading")}
        </p>
      )}
      {isSaving && (
        <p role="status" className="mt-2 text-xs text-muted-foreground">
          {t("settings.developer.developerModeSaving")}
        </p>
      )}
      {loadError !== null && (
        <div className="mt-2 flex items-center gap-2">
          <p role="alert" className="text-xs text-destructive">
            {t("settings.developer.developerModeLoadError")}
          </p>
          <Button size="sm" variant="outline" onClick={() => void retry()}>
            {t("common.retry")}
          </Button>
        </div>
      )}
      {updateError !== null && (
        <p role="alert" className="mt-2 text-xs text-destructive">
          {t("settings.developer.developerModeUpdateError")}
        </p>
      )}
    </section>
  );
}
