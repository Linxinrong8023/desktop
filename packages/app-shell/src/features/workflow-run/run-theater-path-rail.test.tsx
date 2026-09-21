import { createRef } from "react";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createMockWorkflow } from "@ora/workflow-mock";
import {
  normalizeWorkflowDefinition,
  type GraphWorkflowRun,
  type HitlRequest,
} from "@ora/workflow-runtime";
import { AppI18nProvider } from "../../i18n/i18n";
import { appI18n } from "../../i18n/i18n-instance";
import { RunTheaterPathRail } from "./run-theater-path-rail";

beforeEach(async () => {
  await appI18n.changeLanguage("zh-CN");
});

/** Builds a finished mock run for path-rail Result chip coverage. */
function terminalRun(
  status: Extract<
    GraphWorkflowRun["status"],
    "succeeded" | "failed" | "cancelled"
  >,
): GraphWorkflowRun {
  const definition = normalizeWorkflowDefinition(createMockWorkflow("zh-CN"));
  return {
    id: "run-1",
    projectId: "p1",
    definitionId: definition.id,
    definitionSnapshot: definition,
    name: definition.name,
    status,
    nodeStates: Object.fromEntries(
      definition.nodes.map((node) => [
        node.id,
        {
          status: "succeeded" as const,
          finishedAt: "2026-08-04T12:00:00+08:00",
        },
      ]),
    ),
    openHitls: [],
    createdAt: "2026-08-04T12:00:00+08:00",
    updatedAt: "2026-08-04T12:00:00+08:00",
  };
}

/** Waiting run with one open gate on understand. */
function waitingRun(): { run: GraphWorkflowRun; request: HitlRequest } {
  const definition = normalizeWorkflowDefinition(createMockWorkflow("zh-CN"));
  const request: HitlRequest = {
    id: "hitl-1",
    runId: "run-1",
    nodeId: "understand",
    schema: {
      kind: "clarify",
      title: "Clarify",
      fields: [
        { name: "answer", type: "text", label: "Answer", required: true },
      ],
    },
    blocking: true,
    policy: "wait",
    status: "open",
    createdAt: "2026-08-04T12:00:00+08:00",
  };
  return {
    request,
    run: {
      id: "run-1",
      projectId: "p1",
      definitionId: definition.id,
      definitionSnapshot: definition,
      name: definition.name,
      status: "awaiting_input",
      nodeStates: Object.fromEntries(
        definition.nodes.map((node) => [
          node.id,
          {
            status:
              node.id === "understand"
                ? ("awaiting_input" as const)
                : ("idle" as const),
          },
        ]),
      ),
      openHitls: [request],
      createdAt: "2026-08-04T12:00:00+08:00",
      updatedAt: "2026-08-04T12:00:00+08:00",
    },
  };
}

describe("RunTheaterPathRail", () => {
  it("nests completed iteration members in a persistent parallel round navigator", async () => {
    const onFocusNode = vi.fn();
    const onRoundChange = vi.fn();
    const user = userEvent.setup();
    const run: GraphWorkflowRun = {
      id: "run-iteration",
      projectId: "project",
      definitionId: "definition",
      definitionSnapshot: {
        id: "snapshot",
        name: "Iteration run",
        description: "",
        updatedAt: "2026-09-17T12:00:00+08:00",
        viewport: { x: 0, y: 0, zoom: 1 },
        nodes: [
          {
            id: "body-b",
            type: "workflow",
            parentId: "iter",
            position: { x: 120, y: 240 },
            data: {
              kind: "agent",
              title: "生成处理结果",
              description: "",
            },
          },
          {
            id: "out",
            type: "workflow",
            position: { x: 1_200, y: 160 },
            data: { kind: "output", title: "汇总输出", description: "" },
          },
          {
            id: "iter",
            type: "workflow",
            position: { x: 300, y: 160 },
            data: { kind: "iteration", title: "逐项处理", description: "" },
          },
          {
            id: "body-a",
            type: "workflow",
            parentId: "iter",
            position: { x: 120, y: 80 },
            data: { kind: "agent", title: "Agent 1", description: "" },
          },
          {
            id: "start",
            type: "workflow",
            position: { x: 0, y: 160 },
            data: { kind: "start", title: "输入待处理项", description: "" },
          },
        ],
        edges: [
          { id: "start-iter", source: "start", target: "iter" },
          {
            id: "entry-a",
            source: "iter",
            sourceHandle: "iteration-entry",
            target: "body-a",
          },
          {
            id: "entry-b",
            source: "iter",
            sourceHandle: "iteration-entry",
            target: "body-b",
          },
          { id: "iter-out", source: "iter", target: "out" },
        ],
      },
      name: "Iteration run",
      status: "succeeded",
      nodeStates: {
        start: { status: "succeeded" },
        iter: { status: "succeeded" },
        "body-a": { status: "succeeded", iteration: 2 },
        "body-b": { status: "succeeded", iteration: 2 },
        out: { status: "succeeded" },
      },
      roundStates: {
        "body-a": [0, 1, 2].map((iteration) => ({
          status: "succeeded" as const,
          iteration,
          ...(iteration === 1
            ? {
                startedAt: "2026-09-17T12:00:00+08:00",
                finishedAt: "2026-09-17T12:00:05+08:00",
              }
            : {}),
        })),
        "body-b": [0, 2].map((iteration) => ({
          status: "succeeded" as const,
          iteration,
        })),
      },
      openHitls: [],
      createdAt: "2026-09-17T12:00:00+08:00",
      updatedAt: "2026-09-17T12:00:10+08:00",
      finishedAt: "2026-09-17T12:00:10+08:00",
    };

    const view = render(
      <AppI18nProvider>
        <RunTheaterPathRail
          run={run}
          primaryId="body-a"
          activeIds={[]}
          openHitls={[]}
          artifactCountByNode={{}}
          showResultAct={false}
          selectedRound={1}
          onRoundChange={onRoundChange}
          pathRailRef={createRef()}
          onFocusNode={onFocusNode}
          onExpandHitl={vi.fn()}
          onShowResultAct={vi.fn()}
        />
      </AppI18nProvider>,
    );

    const topLevelPath = screen.getByRole("list", {
      name: "顶层执行路径",
    });
    expect(
      within(topLevelPath)
        .getAllByRole("button")
        .map((button) => button.getAttribute("data-path-node")),
    ).toEqual(["start", "iter", "out", null]);

    const navigator = screen.getByRole("region", {
      name: "逐项处理，第 2/3 轮",
    });
    expect(within(navigator).getByText("并行 2")).toBeInTheDocument();
    expect(within(navigator).getByText("1/2 完成")).toBeInTheDocument();
    expect(within(navigator).getByText("5s")).toBeInTheDocument();
    expect(
      within(navigator).getAllByRole("button", { name: /成功/ }),
    ).toHaveLength(1);
    expect(
      within(navigator).getByRole("button", {
        name: "生成处理结果: 本轮未执行",
      }),
    ).toBeInTheDocument();

    await user.click(
      within(navigator).getByRole("button", { name: "选择迭代轮次" }),
    );
    const roundList = screen.getByRole("listbox", { name: "迭代轮次" });
    await user.click(
      within(roundList).getByRole("option", { name: "第 1 轮" }),
    );
    expect(onRoundChange).toHaveBeenCalledWith(0);

    await user.click(
      within(navigator).getByRole("button", { name: /生成处理结果/ }),
    );
    expect(onFocusNode).toHaveBeenCalledWith("body-b");

    await user.click(within(navigator).getByRole("button", { name: "下一轮" }));
    expect(onRoundChange).toHaveBeenCalledWith(2);

    view.rerender(
      <AppI18nProvider>
        <RunTheaterPathRail
          run={{ ...run, roundStates: undefined }}
          primaryId="body-a"
          activeIds={[]}
          openHitls={[]}
          artifactCountByNode={{}}
          showResultAct={false}
          selectedRound={null}
          onRoundChange={onRoundChange}
          pathRailRef={createRef()}
          onFocusNode={onFocusNode}
          onExpandHitl={vi.fn()}
          onShowResultAct={vi.fn()}
        />
      </AppI18nProvider>,
    );

    const legacyNavigator = screen.getByRole("region", { name: "逐项处理" });
    expect(within(legacyNavigator).getByText("2 个成员")).toBeInTheDocument();
    expect(
      within(legacyNavigator).queryByRole("button", { name: "选择迭代轮次" }),
    ).not.toBeInTheDocument();
  });

  it("removes inactive branch nodes from the live path and progress total", () => {
    const definition = normalizeWorkflowDefinition(createMockWorkflow("zh-CN"));
    const inactiveIds = new Set(["quality", "tests", "review"]);
    const run: GraphWorkflowRun = {
      id: "run-1",
      projectId: "p1",
      definitionId: definition.id,
      definitionSnapshot: definition,
      name: definition.name,
      status: "running",
      nodeStates: Object.fromEntries(
        definition.nodes.map((node) => [
          node.id,
          {
            status: inactiveIds.has(node.id)
              ? ("inactive" as const)
              : node.id === "output"
                ? ("running" as const)
                : ("succeeded" as const),
          },
        ]),
      ),
      openHitls: [],
      createdAt: "2026-08-04T12:00:00+08:00",
      updatedAt: "2026-08-04T12:00:00+08:00",
    };
    const undecidedRun: GraphWorkflowRun = {
      ...run,
      nodeStates: Object.fromEntries(
        definition.nodes.map((node) => [node.id, { status: "idle" as const }]),
      ),
    };
    const rail = (currentRun: GraphWorkflowRun) => (
      <AppI18nProvider>
        <RunTheaterPathRail
          run={currentRun}
          primaryId="output"
          activeIds={["output"]}
          openHitls={[]}
          artifactCountByNode={{}}
          showResultAct={false}
          pathRailRef={createRef()}
          onFocusNode={vi.fn()}
          onExpandHitl={vi.fn()}
        />
      </AppI18nProvider>
    );

    const view = render(rail(undecidedRun));
    expect(
      within(screen.getByRole("list")).getAllByRole("button"),
    ).toHaveLength(definition.nodes.length);

    view.rerender(rail(run));

    const chips = within(screen.getByRole("list"))
      .getAllByRole("button")
      .map((chip) => chip.getAttribute("data-path-node"));
    expect(chips).toEqual(["start", "understand", "output"]);
    expect(screen.getByText("2 / 3")).toBeInTheDocument();
  });

  it("renders chips in path order when the snapshot array is reversed", () => {
    const definition = normalizeWorkflowDefinition(createMockWorkflow("zh-CN"));
    const reversed = {
      ...definition,
      nodes: [...definition.nodes].reverse(),
    };
    const run: GraphWorkflowRun = {
      id: "run-1",
      projectId: "p1",
      definitionId: reversed.id,
      definitionSnapshot: reversed,
      name: reversed.name,
      status: "running",
      nodeStates: Object.fromEntries(
        reversed.nodes.map((node) => [node.id, { status: "idle" as const }]),
      ),
      openHitls: [],
      createdAt: "2026-08-04T12:00:00+08:00",
      updatedAt: "2026-08-04T12:00:00+08:00",
    };

    render(
      <AppI18nProvider>
        <RunTheaterPathRail
          run={run}
          primaryId={null}
          activeIds={[]}
          openHitls={[]}
          artifactCountByNode={{}}
          showResultAct={false}
          pathRailRef={createRef()}
          onFocusNode={vi.fn()}
          onExpandHitl={vi.fn()}
        />
      </AppI18nProvider>,
    );

    const chips = within(screen.getByRole("list")).getAllByRole("button");
    expect(chips.map((chip) => chip.getAttribute("data-path-node"))).toEqual([
      "start",
      "understand",
      "quality",
      "tests",
      "review",
      "output",
    ]);
    expect(reversed.nodes.map((node) => node.id)[0]).toBe("output");
  });

  it("appends a status-toned Result chip after path nodes on terminal runs", async () => {
    const onShowResultAct = vi.fn();
    const onFocusNode = vi.fn();
    const user = userEvent.setup();
    const run = terminalRun("succeeded");
    const nodeCount = run.definitionSnapshot.nodes.length;

    render(
      <AppI18nProvider>
        <RunTheaterPathRail
          run={run}
          primaryId={null}
          activeIds={[]}
          openHitls={[]}
          artifactCountByNode={{}}
          showResultAct
          pathRailRef={createRef()}
          onFocusNode={onFocusNode}
          onExpandHitl={vi.fn()}
          onShowResultAct={onShowResultAct}
        />
      </AppI18nProvider>,
    );

    const chips = within(screen.getByRole("list")).getAllByRole("button");
    expect(chips).toHaveLength(nodeCount + 1);
    const resultChip = chips[chips.length - 1]!;
    expect(resultChip).toHaveAttribute("data-path-result");
    expect(resultChip).toHaveAccessibleName("结果: 成功");
    expect(resultChip.className).toContain("border-emerald-500");

    await user.click(resultChip);
    expect(onShowResultAct).toHaveBeenCalledTimes(1);
    expect(onFocusNode).not.toHaveBeenCalled();
  });

  it("tones a failed Result chip and omits Result while the run is live", () => {
    const failed = terminalRun("failed");
    const { rerender } = render(
      <AppI18nProvider>
        <RunTheaterPathRail
          run={failed}
          primaryId="understand"
          activeIds={[]}
          openHitls={[]}
          artifactCountByNode={{}}
          showResultAct={false}
          pathRailRef={createRef()}
          onFocusNode={vi.fn()}
          onExpandHitl={vi.fn()}
          onShowResultAct={vi.fn()}
        />
      </AppI18nProvider>,
    );

    const resultChip = screen.getByRole("button", { name: "结果: 失败" });
    expect(resultChip.className).toContain("border-rose-500");

    const live = waitingRun().run;
    rerender(
      <AppI18nProvider>
        <RunTheaterPathRail
          run={live}
          primaryId="understand"
          activeIds={["understand"]}
          openHitls={live.openHitls}
          artifactCountByNode={{}}
          showResultAct={false}
          pathRailRef={createRef()}
          onFocusNode={vi.fn()}
          onExpandHitl={vi.fn()}
        />
      </AppI18nProvider>,
    );

    expect(
      screen.queryByRole("button", { name: /结果/ }),
    ).not.toBeInTheDocument();
  });

  it("expands HITL from a waiting path chip and focuses other chips", async () => {
    const { run, request } = waitingRun();
    const onFocusNode = vi.fn();
    const onExpandHitl = vi.fn();
    const user = userEvent.setup();

    render(
      <AppI18nProvider>
        <RunTheaterPathRail
          run={run}
          primaryId="start"
          activeIds={["understand"]}
          openHitls={[request]}
          artifactCountByNode={{ understand: 2 }}
          showResultAct={false}
          pathRailRef={createRef()}
          onFocusNode={onFocusNode}
          onExpandHitl={onExpandHitl}
        />
      </AppI18nProvider>,
    );

    await user.click(screen.getByRole("button", { name: /理解改动/ }));
    expect(onExpandHitl).toHaveBeenCalledWith("hitl-1");
    expect(onFocusNode).not.toHaveBeenCalled();

    await user.click(screen.getByRole("button", { name: /开始:/ }));
    expect(onFocusNode).toHaveBeenCalledWith("start");
  });
});
