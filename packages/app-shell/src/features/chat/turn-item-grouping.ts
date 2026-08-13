import type {
  ChatPlan,
  ChatToolCall,
  ChatTurnItem,
  ChatTurnStatus,
} from "@ora/chat";
import type { ActivityItem } from "./activity-group";
import {
  toolCallGroupKind,
  type ToolCallGroupKind,
} from "./tool-call-group-kind";

/** One run of consecutive tool calls sharing a group kind, collapsed into a single disclosure. */
export interface ToolGroup {
  kind: "toolGroup";
  id: string;
  groupKind: ToolCallGroupKind;
  tools: ChatToolCall[];
}

/** One run of interleaved thoughts and exploratory tool calls, condensed into a secondary timeline. */
export interface ActivityGroupItem {
  kind: "activityGroup";
  id: string;
  items: ActivityItem[];
}

/** Non-text turn content: reasoning and tool activity, as opposed to the final answer. */
export type NonTextDisplayItem =
  ActivityGroupItem | ChatPlan | ChatToolCall | ToolGroup;

/** One contiguous run of non-text activity: rendered live while streaming, or as one collapsed summary once it ends. */
export interface ActivityPhaseItem {
  kind: "activityPhase";
  id: string;
  items: NonTextDisplayItem[];
  live: boolean;
}

type TextDisplayItem = Extract<ChatTurnItem, { kind: "message" | "content" }>;

/** The final sequence of blocks rendered for one response turn. */
export type DisplayTurnItem = ActivityPhaseItem | TextDisplayItem;

type ToolGroupedTurnItem = ChatTurnItem | ToolGroup;
// Every thought is absorbed into an ActivityGroupItem by groupExplorationActivity, so it never
// reaches this stage on its own.
type ExplorationGroupedItem =
  ActivityGroupItem | Exclude<ToolGroupedTurnItem, { kind: "thought" }>;

/** Builds the display sequence for one turn: adjacent tools, exploration activity, then activity phases. */
export function buildTurnDisplayItems(
  items: ChatTurnItem[],
  turnStatus: ChatTurnStatus,
): DisplayTurnItem[] {
  return groupActivityPhases(
    groupExplorationActivity(groupAdjacentTools(items)),
    turnStatus,
  );
}

/** Groups adjacent tools by intent while preserving boundaries created by messages and plans. */
function groupAdjacentTools(items: ChatTurnItem[]): ToolGroupedTurnItem[] {
  const grouped: ToolGroupedTurnItem[] = [];
  let tools: ChatToolCall[] = [];
  let groupKind: ToolCallGroupKind | null = null;

  const flushTools = () => {
    if (tools.length === 1) grouped.push(tools[0]);
    if (tools.length > 1 && groupKind !== null) {
      grouped.push({
        kind: "toolGroup",
        id: `${groupKind}-group-${tools[0].id}`,
        groupKind,
        tools,
      });
    }
    tools = [];
    groupKind = null;
  };

  for (const item of items) {
    const nextGroupKind =
      item.kind === "toolCall" ? toolCallGroupKind(item) : null;
    if (item.kind === "toolCall" && nextGroupKind !== null) {
      if (groupKind !== null && groupKind !== nextGroupKind) flushTools();
      groupKind = nextGroupKind;
      tools.push(item);
      continue;
    }
    flushTools();
    grouped.push(item);
  }
  flushTools();
  return grouped;
}

/** Merges interleaved thoughts and exploratory calls into one compact progress group. */
function groupExplorationActivity(
  items: ToolGroupedTurnItem[],
): ExplorationGroupedItem[] {
  const grouped: ExplorationGroupedItem[] = [];
  let activity: ActivityItem[] = [];

  const flushActivity = () => {
    if (activity.length > 0) {
      grouped.push({
        kind: "activityGroup",
        id: `activity-group-${activity[0].id}`,
        items: activity,
      });
    }
    activity = [];
  };

  for (const item of items) {
    if (item.kind === "thought") {
      activity.push(item);
      continue;
    }
    if (item.kind === "toolCall" && toolCallGroupKind(item) === "exploration") {
      activity.push(item);
      continue;
    }
    if (item.kind === "toolGroup" && item.groupKind === "exploration") {
      activity.push(...item.tools);
      continue;
    }
    flushActivity();
    grouped.push(item);
  }
  flushActivity();
  return grouped;
}

/**
 * Bundles every non-text item between text output into one activity phase. A phase stays live
 * (each item rendered and disclosed as today) only while it is the turn's trailing run and the
 * turn is still streaming; it becomes one collapsed summary as soon as text follows it or the
 * turn stops (completed, cancelled, or failed), matching how the agent's "thinking" visually
 * wraps up once the answer starts or the round ends.
 */
function groupActivityPhases(
  items: ExplorationGroupedItem[],
  turnStatus: ChatTurnStatus,
): DisplayTurnItem[] {
  const grouped: DisplayTurnItem[] = [];
  let phase: NonTextDisplayItem[] = [];

  const flushPhase = (live: boolean) => {
    if (phase.length > 0) {
      grouped.push({
        kind: "activityPhase",
        id: `activity-phase-${phase[0].id}`,
        items: phase,
        live,
      });
    }
    phase = [];
  };

  for (const item of items) {
    if (item.kind === "message" || item.kind === "content") {
      flushPhase(false);
      grouped.push(item);
      continue;
    }
    phase.push(item);
  }
  flushPhase(turnStatus === "streaming");
  return grouped;
}
