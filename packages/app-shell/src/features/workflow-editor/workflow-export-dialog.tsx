import { useState } from "react";
import { useTranslation } from "react-i18next";
import { IconInfoCircle } from "@tabler/icons-react";
import {
  Button,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
  cn,
} from "@ora/ui";
import {
  workflowExportFileName,
  type WorkflowDependency,
} from "./workflow-transfer";
import {
  WorkflowTransferDependencies,
  WorkflowTransferPill,
} from "./workflow-transfer-dependencies";

/** One published version the user may export instead of the editable draft. */
export interface WorkflowExportVersionOption {
  version: string;
  createdAt: string;
  active: boolean;
}

interface WorkflowExportDialogProps {
  workflowName: string;
  versions: readonly WorkflowExportVersionOption[];
  /** Formatted time the draft was last saved, when known. */
  draftSavedAt: string | undefined;
  /** `null` selects the current draft, including unpublished edits. */
  selectedVersion: string | null;
  dependencies: readonly WorkflowDependency[];
  /** Pretty-printed document that export will write, or `null` while it loads. */
  previewJson: string | null;
  busy: boolean;
  error: string | null;
  onSelectVersion: (version: string | null) => void;
  onCancel: () => void;
  onExport: (fileName: string) => void;
}

/**
 * Lets users choose the draft or a published version and review the plugin references the
 * file will carry. Plugins are referenced by identity only; no package or secret is bundled.
 */
export function WorkflowExportDialog({
  workflowName,
  versions,
  draftSavedAt,
  selectedVersion,
  dependencies,
  previewJson,
  busy,
  error,
  onSelectVersion,
  onCancel,
  onExport,
}: WorkflowExportDialogProps) {
  const { t } = useTranslation();
  const [fileName, setFileName] = useState(() =>
    workflowExportFileName(workflowName, selectedVersion),
  );
  const loading = previewJson === null;
  const options = [
    {
      version: null,
      label: t("settings.workflow.transfer.currentDraft"),
      detail:
        draftSavedAt === undefined
          ? t("settings.workflow.transfer.currentDraftHint")
          : t("settings.workflow.transfer.currentDraftSavedHint", {
              time: draftSavedAt,
            }),
      active: false,
    },
    ...versions.map((option) => ({
      version: option.version,
      label: option.version,
      detail: option.active
        ? t("settings.workflow.transfer.activeVersionDetail", {
            time: option.createdAt,
          })
        : option.createdAt,
      active: option.active,
    })),
  ];

  /** Switches the source and resets the proposed file name to match it. */
  function select(version: string | null): void {
    onSelectVersion(version);
    setFileName(workflowExportFileName(workflowName, version));
  }

  const trimmedFileName = fileName.trim();

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !busy) {
          onCancel();
        }
      }}
    >
      <DialogContent className="gap-0 p-0 sm:max-w-[620px]">
        <DialogHeader className="gap-1 px-5 pt-[18px] pb-2.5">
          <DialogTitle className="text-[15px] font-semibold tracking-[-0.01em]">
            {t("settings.workflow.transfer.exportTitle", {
              name: workflowName,
            })}
          </DialogTitle>
          <DialogDescription className="text-[12.5px] text-muted-foreground">
            {t("settings.workflow.transfer.exportDescription")}
          </DialogDescription>
        </DialogHeader>
        <div className="flex max-h-[60vh] min-w-0 flex-col gap-3.5 overflow-y-auto px-5 pt-1.5 pb-3.5">
          <div>
            <span
              id="workflow-export-source-label"
              className="mb-1.5 block text-[12px] font-medium"
            >
              {t("settings.workflow.transfer.exportSource")}
            </span>
            <div
              role="radiogroup"
              aria-labelledby="workflow-export-source-label"
              className="grid gap-1.5"
            >
              {options.map((option) => {
                const checked = option.version === selectedVersion;
                return (
                  <button
                    key={option.version ?? "draft"}
                    type="button"
                    role="radio"
                    aria-checked={checked}
                    disabled={busy}
                    onClick={() => select(option.version)}
                    className={cn(
                      "grid grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-2.5 rounded-[9px] border px-3 py-[9px] text-left outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-60",
                      checked ? "border-foreground" : "border-border",
                    )}
                  >
                    <span
                      aria-hidden
                      className={cn(
                        "flex size-[13px] items-center justify-center rounded-full border",
                        checked ? "border-blue-600" : "border-muted-foreground",
                      )}
                    >
                      {checked && (
                        <span className="size-[7px] rounded-full bg-blue-600" />
                      )}
                    </span>
                    <span className="min-w-0">
                      <span
                        className={cn(
                          "block truncate text-[13px] font-medium",
                          option.version !== null && "font-mono",
                        )}
                      >
                        {option.label}
                      </span>
                      <span className="block truncate text-[11.5px] text-muted-foreground">
                        {option.detail}
                      </span>
                    </span>
                    {option.active ? (
                      <WorkflowTransferPill tone="ok">
                        {t("settings.workflow.transfer.activeVersion")}
                      </WorkflowTransferPill>
                    ) : (
                      <span />
                    )}
                  </button>
                );
              })}
            </div>
          </div>
          <label className="block">
            <span className="mb-1.5 block text-[12px] font-medium">
              {t("settings.workflow.transfer.fileName")}
            </span>
            <Input
              value={fileName}
              onChange={(event) => setFileName(event.target.value)}
              disabled={busy}
              className="h-[34px] font-mono"
            />
          </label>
          {loading ? (
            <p className="text-[12px] text-muted-foreground">
              {t("settings.workflow.transfer.loadingVersion")}
            </p>
          ) : (
            <WorkflowTransferDependencies
              title={t("settings.workflow.transfer.recordedDependencies")}
              dependencies={dependencies}
              variant="export"
            />
          )}
          <div className="flex gap-2.5 rounded-[9px] bg-blue-500/10 px-3 py-2.5 text-[12.5px] leading-normal text-blue-700 dark:text-blue-300">
            <IconInfoCircle className="mt-0.5 size-[15px] shrink-0" />
            <div>{t("settings.workflow.transfer.referenceOnlyNotice")}</div>
          </div>
          {previewJson !== null && (
            <details>
              <summary className="cursor-pointer text-[12.5px] font-medium">
                {t("settings.workflow.transfer.previewStructure")}
              </summary>
              <pre className="mt-2 max-h-[170px] overflow-auto rounded-[9px] border border-border bg-muted/40 px-3 py-2.5 font-mono text-[11.5px] whitespace-pre">
                {previewJson}
              </pre>
            </details>
          )}
          {error !== null && (
            <p role="alert" className="text-[12px] text-destructive">
              {error}
            </p>
          )}
        </div>
        <DialogFooter className="mx-0 mb-0 flex-row flex-wrap items-center gap-2 rounded-b-xl border-t border-border bg-transparent px-5 pt-3 pb-4">
          <span className="flex-1" />
          <Button variant="outline" disabled={busy} onClick={onCancel}>
            {t("common.cancel")}
          </Button>
          <Button
            disabled={busy || loading || trimmedFileName === ""}
            onClick={() => onExport(trimmedFileName)}
          >
            {t("settings.workflow.transfer.confirmExport")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
