import type { ReactNode } from "react";
import { UnusedNodes } from "./execution-membership-context";
import { useTranslation } from "react-i18next";

/** Carries backend-derived participation without mutating persisted node data. */
export function WorkflowMembershipProvider({
  unusedNodeIds,
  children,
}: {
  unusedNodeIds: readonly string[];
  children: ReactNode;
}) {
  return (
    <UnusedNodes.Provider value={unusedNodeIds}>
      {children}
    </UnusedNodes.Provider>
  );
}

/** Distinguishes static exclusion from pending execution and unselected branches. */
export function WorkflowUnusedBadge() {
  const { t } = useTranslation();
  return (
    <span
      className="rounded bg-muted px-1.5 py-0.5 text-[10px] text-muted-foreground"
      title={t("workflowNode.unusedHint")}
    >
      {t("workflowNode.unused")}
    </span>
  );
}
