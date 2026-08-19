import type { RuntimeLogLevel } from "@ora/contracts";
import { Select, SelectContent, SelectItem, SelectTrigger } from "@ora/ui";
import { IconBug } from "@tabler/icons-react";
import { useTranslation } from "react-i18next";
import { useRuntimeLogLevel } from "../../state/hooks/use-runtime-log-level";

const LOG_LEVELS: RuntimeLogLevel[] = [
  "trace",
  "debug",
  "info",
  "warn",
  "error",
];

/** Presents the server-authoritative process-wide log filter in shared settings. */
export function RuntimeLogLevelSettings() {
  const { t } = useTranslation();
  const { state, isLoading, loadError, isSaving, updateError, submitLevel } =
    useRuntimeLogLevel();
  const error =
    updateError === null
      ? loadError === null
        ? null
        : t("settings.developer.logLevelLoadError")
      : t("settings.developer.logLevelUpdateError");
  const highVolume =
    state?.effectiveLevel === "trace" || state?.effectiveLevel === "debug";

  return (
    <section
      className="border-b border-border py-4"
      aria-labelledby="runtime-log-level-title"
    >
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center">
        <IconBug className="hidden size-4 shrink-0 text-muted-foreground sm:block" />
        <div className="min-w-0 flex-1">
          <p id="runtime-log-level-title" className="text-sm font-medium">
            {t("settings.developer.logLevel")}
          </p>
          <p className="mt-1 text-xs leading-5 text-muted-foreground">
            {t("settings.developer.logLevelDescription")}
          </p>
        </div>
        <div className="shrink-0">
          <Select
            value={state?.effectiveLevel ?? ""}
            disabled={isLoading || loadError !== null || isSaving}
            onValueChange={(value) => submitLevel(value as RuntimeLogLevel)}
          >
            <SelectTrigger
              className="w-40"
              aria-label={t("settings.developer.logLevel")}
            >
              <span className="flex-1 text-left">
                {state === undefined
                  ? isLoading
                    ? t("settings.developer.logLevelLoading")
                    : t("settings.developer.logLevelUnavailable")
                  : logLevelLabel(state.effectiveLevel, t)}
              </span>
            </SelectTrigger>
            <SelectContent>
              {LOG_LEVELS.map((level) => (
                <SelectItem key={level} value={level}>
                  {logLevelLabel(level, t)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </div>
      {isSaving && (
        <p role="status" className="mt-2 text-xs text-muted-foreground">
          {t("settings.developer.logLevelSaving")}
        </p>
      )}
      {error !== null && (
        <p role="alert" className="mt-2 text-xs text-destructive">
          {error}
        </p>
      )}
      {highVolume && (
        <p className="mt-2 text-xs leading-5 text-amber-700 dark:text-amber-400">
          {t("settings.developer.logLevelVolumeWarning")}
        </p>
      )}
    </section>
  );
}

/** Localizes one closed contract log-level value without maintaining a second label map. */
function logLevelLabel(
  level: RuntimeLogLevel,
  t: ReturnType<typeof useTranslation>["t"],
): string {
  return t(`settings.developer.logLevel.${level}`);
}
