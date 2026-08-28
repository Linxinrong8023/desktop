import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ComponentProps } from "react";
import { AppI18nProvider } from "../../../i18n/i18n";
import { appI18n } from "../../../i18n/i18n-instance";
import type { WorkflowHistoryStep } from "../workflow-history";
import { WorkflowHistoryControls } from "./history-controls";

const historyStep: WorkflowHistoryStep = {
  id: "step-1",
  event: "node.add",
  meta: { nodeTitle: "Agent" },
  snapshot: {
    name: "Demo",
    description: "",
    nodes: [],
    edges: [],
    annotations: [],
  },
  fingerprint: "snapshot",
};

/** Builds a compact history entry for ordering assertions. */
function createHistoryStep(
  id: string,
  event: WorkflowHistoryStep["event"],
  subject: string,
): WorkflowHistoryStep {
  return {
    ...historyStep,
    id,
    event,
    meta: { subject },
  };
}

/** Wraps the history controls with the same translation provider as the editor. */
function renderHistory(
  overrides: Partial<ComponentProps<typeof WorkflowHistoryControls>> = {},
) {
  return render(
    <AppI18nProvider>
      <WorkflowHistoryControls
        canUndo={true}
        canRedo={false}
        past={[historyStep]}
        future={[]}
        currentEvent="node.add"
        readOnly={false}
        onUndo={vi.fn()}
        onRedo={vi.fn()}
        onJump={vi.fn()}
        onClear={vi.fn()}
        {...overrides}
      />
    </AppI18nProvider>,
  );
}

describe("WorkflowHistoryControls", () => {
  beforeEach(() => {
    void appI18n.changeLanguage("zh-CN");
  });

  it("uses the requested undo, redo, divider, and history controls", () => {
    renderHistory();

    expect(screen.getByRole("button", { name: "撤销" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "重做" })).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "变更历史" }),
    ).toBeInTheDocument();
  });

  it("opens the history list and jumps to a selected row", async () => {
    const user = userEvent.setup();
    const onJump = vi.fn();
    renderHistory({
      onJump,
      currentEvent: "node.move",
      currentMeta: { nodeTitle: "审核节点" },
    });

    await user.click(screen.getByRole("button", { name: "变更历史" }));
    expect(screen.getByText("会话开始（1 步后退）")).toBeInTheDocument();
    expect(
      screen.getByText("移动节点：审核节点（当前状态）"),
    ).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /会话开始/ }));
    expect(onJump).toHaveBeenCalledWith("past", 1);
  });

  it("closes the history list when the user clicks elsewhere", async () => {
    const user = userEvent.setup();
    renderHistory();

    await user.click(screen.getByRole("button", { name: "变更历史" }));
    expect(
      screen.getByRole("heading", { name: "变更历史" }),
    ).toBeInTheDocument();

    await user.click(document.body);

    expect(
      screen.queryByRole("heading", { name: "变更历史" }),
    ).not.toBeInTheDocument();
  });

  it("keeps annotation edits concise in the history list", async () => {
    const user = userEvent.setup();
    renderHistory({ currentEvent: "annotation.edit" });

    await user.click(screen.getByRole("button", { name: "变更历史" }));

    expect(screen.getByText("编辑注释（当前状态）")).toBeInTheDocument();
    expect(screen.queryByText(/：注释内容/)).not.toBeInTheDocument();
  });

  it("uses a generic annotation name for annotation moves", async () => {
    const user = userEvent.setup();
    renderHistory({
      currentEvent: "node.move",
      currentMeta: { subject: "注释", nodeKind: "annotation" },
    });

    await user.click(screen.getByRole("button", { name: "变更历史" }));

    expect(screen.getByText("移动节点：注释（当前状态）")).toBeInTheDocument();
  });

  it("keeps future, current, and past entries in newest-to-oldest order", async () => {
    const user = userEvent.setup();
    renderHistory({
      currentEvent: "node.edit",
      currentMeta: { subject: "输出" },
      future: [
        createHistoryStep("newest", "edge.connect", "输出 → 汇总"),
        createHistoryStep("nearest", "node.move", "模板转换"),
      ],
      past: [
        createHistoryStep("first", "node.add", "Agent"),
        createHistoryStep("second", "node.move", "模板转换"),
      ],
    });

    await user.click(screen.getByRole("button", { name: "变更历史" }));
    const newest = screen.getByText("连接节点：输出 → 汇总（2 步前进）");
    const nearest = screen.getByText("移动节点：模板转换（1 步前进）");
    const current = screen.getByText("编辑节点：输出（当前状态）");
    const previous = screen.getByText("添加节点：Agent（1 步后退）");
    const oldest = screen.getByText("会话开始（2 步后退）");

    expect(newest.compareDocumentPosition(nearest)).toBe(
      Node.DOCUMENT_POSITION_FOLLOWING,
    );
    expect(nearest.compareDocumentPosition(current)).toBe(
      Node.DOCUMENT_POSITION_FOLLOWING,
    );
    expect(current.compareDocumentPosition(previous)).toBe(
      Node.DOCUMENT_POSITION_FOLLOWING,
    );
    expect(previous.compareDocumentPosition(oldest)).toBe(
      Node.DOCUMENT_POSITION_FOLLOWING,
    );
  });
});
