import { Fragment, memo, useState } from "react";
import {
  Handle,
  NodeResizer,
  Position,
  useReactFlow,
  type Node,
  type NodeProps,
} from "@xyflow/react";
import { useTranslation } from "react-i18next";
import { IconTrash } from "@tabler/icons-react";
import { cn } from "@ora/ui";
import {
  createMockWorkflowCapabilities,
  createMockWorkflowNodeType,
  isWorkflowConditionComparisonComplete,
  resolveConditionCases,
  WORKFLOW_LOOP_NODE_HEIGHT,
  WORKFLOW_LOOP_NODE_WIDTH,
  WORKFLOW_NODE_ANCHOR_Y,
  WORKFLOW_NODE_WIDTH,
  type WorkflowNodeData,
} from "@ora/workflow-mock";
import {
  AgentExecutionModeMark,
  WorkflowNodeCardShell,
  useWorkflowNodeUnused,
  WorkflowUnusedBadge,
} from "../../workflow-node-chrome";
import { useWorkflowConnectionState } from "./use-connection-state";
import { WorkflowNodeParameterSummary } from "./node-parameter-summary";
import { IterationInsertMenu } from "./iteration-actions";
import { useWorkflowIterationActions } from "./iteration-actions-context";
import { IterationNodeFrame } from "./iteration-node";
import type { IterationInsertion } from "../workflow-iteration-graph";

const CONDITION_NODE_WIDTH = 320;
const CONDITION_FIRST_HANDLE_Y = 82;
const CONDITION_EMPTY_CASE_HEIGHT = 28;
const CONDITION_RULE_HEIGHT = 44;
const CONDITION_UNSET_RULE_HEIGHT = 28;
const CONDITION_LOGIC_CONNECTOR_HEIGHT = 14;

/** Renders one workflow card with left/right handles styled for the definition editor. */
export const WorkflowFlowNodeView = memo(function WorkflowFlowNodeView({
  id,
  data,
  deletable,
  selected,
  draggable,
  width,
  height,
  parentId,
  positionAbsoluteX,
  positionAbsoluteY,
}: NodeProps<Node<WorkflowNodeData, "workflow">>) {
  const { i18n, t } = useTranslation();
  const unused = useWorkflowNodeUnused(id);
  const { deleteElements } = useReactFlow<Node<WorkflowNodeData, "workflow">>();
  const { connectionCandidateEndpoint, connectionCandidateNodeId } =
    useWorkflowConnectionState();
  const locale =
    i18n.resolvedLanguage === "en-US" ? ("en-US" as const) : ("zh-CN" as const);
  const nodeKindLabel = createMockWorkflowNodeType(data.kind, locale).label;
  const isConnectionCandidate = connectionCandidateNodeId === id;
  const isInputCandidate =
    isConnectionCandidate && connectionCandidateEndpoint === "target";
  const isOutputCandidate =
    isConnectionCandidate && connectionCandidateEndpoint === "source";
  const conditionCases =
    data.kind === "condition" ? resolveConditionCases(data) : [];
  const iterationActions = useWorkflowIterationActions();

  if (data.kind === "iteration") {
    return (
      <IterationNodeFrame
        id={id}
        data={data}
        selected={selected}
        deletable={deletable}
        isInputCandidate={isInputCandidate}
        isOutputCandidate={isOutputCandidate}
        positionAbsoluteX={positionAbsoluteX}
        positionAbsoluteY={positionAbsoluteY}
        nodeKindLabel={nodeKindLabel}
      />
    );
  }

  return (
    <>
      {data.kind === "loop" && (
        <NodeResizer
          isVisible={selected && draggable}
          minWidth={480}
          minHeight={260}
          handleClassName="!size-2.5 !border !border-ring !bg-background"
          lineClassName="!border-ring/60"
        />
      )}
      <WorkflowNodeCardShell
        data-workflow-node=""
        data-workflow-node-id={id}
        data-x={String(Math.round(positionAbsoluteX))}
        data-y={String(Math.round(positionAbsoluteY))}
        kind={data.kind}
        title={data.title}
        description={data.description}
        kindLabel={id}
        density="editor"
        headerAccessory={unused ? <WorkflowUnusedBadge /> : undefined}
        selected={selected}
        width={
          data.kind === "loop"
            ? (width ?? WORKFLOW_LOOP_NODE_WIDTH)
            : data.kind === "condition"
              ? CONDITION_NODE_WIDTH
              : WORKFLOW_NODE_WIDTH
        }
        style={
          data.kind === "loop"
            ? { height: height ?? WORKFLOW_LOOP_NODE_HEIGHT }
            : undefined
        }
        titleAccessory={
          data.kind === "agent" ? (
            <AgentExecutionModeMark
              interactive={data.agentConfig?.interactive === true}
            />
          ) : undefined
        }
        ariaLabel={`${t("settings.workflow.nodeSuffix", { type: nodeKindLabel })}: ${data.title}`}
        frameClassName={cn(
          unused && "opacity-60",
          parentId !== undefined && "group/iteration-member",
          isConnectionCandidate &&
            "border-ring/60 shadow-md ring-2 ring-ring/10",
        )}
        details={
          data.kind === "condition" ? (
            <ConditionNodeDetails data={data} locale={locale} />
          ) : (
            <WorkflowNodeParameterSummary data={data} />
          )
        }
        detailsClassName={data.kind === "condition" ? "space-y-2" : undefined}
        headerEnd={
          selected && deletable ? (
            <button
              type="button"
              className="nodrag nopan flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground outline-none hover:bg-destructive/10 hover:text-destructive focus-visible:ring-2 focus-visible:ring-ring"
              aria-label={t("settings.workflow.deleteNamed", {
                name: data.title,
              })}
              onClick={() => {
                void deleteElements({ nodes: [{ id }] });
              }}
            >
              <IconTrash className="size-3.5" />
            </button>
          ) : undefined
        }
        targetHandle={
          <Handle
            type="target"
            position={Position.Left}
            data-workflow-input={id}
            aria-label={t("settings.workflow.connectTo", { name: data.title })}
            className={cn(
              "workflow-port workflow-port-input !size-2.5 !border-0 !bg-transparent",
              isInputCandidate && "workflow-port-candidate",
            )}
            style={{ top: WORKFLOW_NODE_ANCHOR_Y }}
          />
        }
        sourceHandle={
          data.kind === "condition" ? (
            <>
              {conditionCases.map((conditionCase, index) => (
                <IterationOutputPort
                  key={conditionCase.id}
                  nodeId={id}
                  handleId={conditionCase.id}
                  top={conditionHandleTop(conditionCases, index)}
                  connectLabel={`${t("settings.workflow.connectFrom", { name: data.title })} · ${conditionCase.id}`}
                  plusLabel={t("settings.workflow.iteration.appendBranch", {
                    branch: conditionCase.id,
                  })}
                  insertion={iterationActions.outputInsertion(
                    id,
                    conditionCase.id,
                  )}
                  isOutputCandidate={isOutputCandidate}
                />
              ))}
              <IterationOutputPort
                nodeId={id}
                handleId="else"
                top={conditionHandleTop(conditionCases, conditionCases.length)}
                connectLabel={`${t("settings.workflow.connectFrom", { name: data.title })} · else`}
                plusLabel={t("settings.workflow.iteration.appendBranch", {
                  branch: "ELSE",
                })}
                insertion={iterationActions.outputInsertion(id, "else")}
                isOutputCandidate={isOutputCandidate}
              />
            </>
          ) : (
            <IterationOutputPort
              nodeId={id}
              top={WORKFLOW_NODE_ANCHOR_Y}
              connectLabel={t("settings.workflow.connectFrom", {
                name: data.title,
              })}
              plusLabel={t("settings.workflow.iteration.appendOutput", {
                name: data.title,
              })}
              insertion={
                parentId !== undefined
                  ? iterationActions.outputInsertion(id)
                  : null
              }
              isOutputCandidate={isOutputCandidate}
            />
          )
        }
      />
    </>
  );
});

/** Renders Dify-style IF / ELIF / ELSE rows whose labels align with branch handles. */
function ConditionNodeDetails({
  data,
  locale,
}: {
  data: WorkflowNodeData;
  locale: "zh-CN" | "en-US";
}) {
  const { t } = useTranslation();
  const cases = resolveConditionCases(data);
  const operators = createMockWorkflowCapabilities(locale).conditionOperators;
  return (
    <div className="space-y-2">
      {cases.map((conditionCase, caseIndex) =>
        conditionCase.conditions.length === 0 ? (
          <div
            key={conditionCase.id}
            className="flex h-5 items-center justify-end text-[10px] font-semibold"
          >
            {caseIndex === 0 ? "IF" : "ELIF"}
          </div>
        ) : (
          <div key={conditionCase.id} className="space-y-1">
            <div className="flex items-center justify-between text-[10px] font-semibold text-muted-foreground">
              <span>CASE {caseIndex + 1}</span>
              <span className="text-foreground">
                {caseIndex === 0 ? "IF" : "ELIF"}
              </span>
            </div>
            <div>
              {conditionCase.conditions.map((comparison, comparisonIndex) => (
                <Fragment key={comparisonIndex}>
                  {comparisonIndex > 0 && (
                    <div className="flex h-3.5 items-center justify-end pr-2 text-[10px] font-semibold text-blue-600 dark:text-blue-400">
                      {(conditionCase.logic ?? "and").toUpperCase()}
                    </div>
                  )}
                  <div className="min-w-0 rounded-lg bg-muted/70 px-2 py-1.5 text-[10px] leading-4">
                    {isWorkflowConditionComparisonComplete(comparison) ? (
                      <>
                        <div className="flex min-w-0 items-center gap-1 font-medium">
                          <span className="truncate text-muted-foreground">
                            {comparison.variableSelector[0]}
                          </span>
                          <span className="text-blue-600 dark:text-blue-400">
                            /
                          </span>
                          <span className="truncate text-blue-600 dark:text-blue-400">
                            {comparison.variableSelector.slice(1).join(".")}
                          </span>
                        </div>
                        <div className="flex min-w-0 items-center gap-2">
                          <span className="font-semibold">
                            {operators.find(
                              (operator) =>
                                operator.value === comparison.operator,
                            )?.label ?? comparison.operator}
                          </span>
                          <span className="truncate text-muted-foreground">
                            {conditionValueLabel(comparison.value)}
                          </span>
                        </div>
                      </>
                    ) : (
                      <span className="font-medium text-muted-foreground">
                        {t("settings.workflow.condition.unset")}
                      </span>
                    )}
                  </div>
                </Fragment>
              ))}
            </div>
          </div>
        ),
      )}
      <div className="flex justify-end pt-0.5 text-[10px] font-semibold">
        ELSE
      </div>
    </div>
  );
}

/** One source port paired with its Dify-style append affordance. The plus badge is
 * decorative (pointer-events-none) and centered on the port; clicking the port opens
 * the node picker, while dragging from the port still starts a connection. */
function IterationOutputPort({
  nodeId,
  handleId,
  top,
  connectLabel,
  plusLabel,
  insertion,
  isOutputCandidate,
}: {
  nodeId: string;
  handleId?: string;
  top: number;
  connectLabel: string;
  plusLabel: string;
  insertion: IterationInsertion | null;
  isOutputCandidate: boolean;
}) {
  const { readOnly } = useWorkflowIterationActions();
  const [open, setOpen] = useState(false);
  const offersInsert = insertion !== null && !readOnly;
  return (
    <>
      <Handle
        id={handleId}
        type="source"
        position={Position.Right}
        data-workflow-output={nodeId}
        aria-label={connectLabel}
        className={cn(
          "workflow-port workflow-port-output !size-2.5 !border-0 !bg-transparent",
          isOutputCandidate && "workflow-port-candidate",
        )}
        style={{ top }}
        onClick={offersInsert ? () => setOpen((open) => !open) : undefined}
      />
      {insertion !== null && (
        <IterationInsertMenu
          insertion={insertion}
          label={plusLabel}
          side="right"
          open={open}
          onOpenChange={setOpen}
          style={{ top }}
          className="pointer-events-none absolute -right-3 z-10 -translate-y-1/2 opacity-0 transition-opacity duration-150 group-hover/iteration-member:opacity-100 focus-visible:opacity-100 data-popup-open:opacity-100"
        />
      )}
    </>
  );
}

function conditionHandleTop(
  cases: ReturnType<typeof resolveConditionCases>,
  branchIndex: number,
): number {
  return cases
    .slice(0, branchIndex)
    .reduce(
      (top, conditionCase) =>
        top +
        (conditionCase.conditions.length === 0
          ? CONDITION_EMPTY_CASE_HEIGHT
          : CONDITION_EMPTY_CASE_HEIGHT +
            conditionCase.conditions.reduce(
              (height, comparison) =>
                height +
                (isWorkflowConditionComparisonComplete(comparison)
                  ? CONDITION_RULE_HEIGHT
                  : CONDITION_UNSET_RULE_HEIGHT),
              0,
            ) +
            Math.max(0, conditionCase.conditions.length - 1) *
              CONDITION_LOGIC_CONNECTOR_HEIGHT),
      CONDITION_FIRST_HANDLE_Y,
    );
}

/** Formats a condition comparison value for the compact canvas card. */
function conditionValueLabel(value: unknown): string {
  if (value === undefined || value === null) {
    return "";
  }
  return typeof value === "string" ? value : JSON.stringify(value);
}
