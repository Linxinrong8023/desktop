import { describe, expect, it } from "vitest";
import {
  createMockWorkflow,
  createMockWorkflowLoopGroup,
} from "@ora/workflow-mock";
import {
  normalizeWorkflowDefinition,
  validateWorkflowDefinition,
  WorkflowDefinitionValidationError,
} from "../src/index";

describe("workflow definition validation", () => {
  it("accepts a normalized executable DAG", () => {
    const definition = normalizeWorkflowDefinition(createMockWorkflow("en-US"));

    expect(() => validateWorkflowDefinition(definition)).not.toThrow();
  });

  it("migrates a legacy Start instruction to the Start input", () => {
    const workflow = createMockWorkflow("en-US");
    const start = workflow.nodes.find((node) => node.data.kind === "start");
    if (start === undefined) {
      throw new Error("Mock workflow must contain a Start node");
    }
    const legacyStartData = { ...start.data };
    delete legacyStartData.input;
    start.data = { ...legacyStartData, instruction: "Legacy kickoff" };

    const definition = normalizeWorkflowDefinition(workflow);
    const normalizedStart = definition.nodes.find(
      (node) => node.data.kind === "start",
    );

    expect(normalizedStart?.data.input).toBe("Legacy kickoff");
    expect(normalizedStart?.data.instruction).toBeUndefined();
  });

  it("preserves Loop parentage through import normalization", () => {
    const workflow = createMockWorkflow("en-US");
    const group = createMockWorkflowLoopGroup({
      sequence: 1,
      position: { x: 360, y: 120 },
      locale: "en-US",
    });
    workflow.nodes.push(...group.nodes);
    workflow.edges.push(
      {
        id: "e-start-loop-1",
        source: "start",
        target: "loop-1",
        type: "workflow",
      },
      ...group.edges,
    );

    const definition = normalizeWorkflowDefinition(workflow);

    expect(
      definition.nodes
        .filter((node) => node.data.containerId === "loop-1")
        .map((node) => ({ id: node.id, parentId: node.parentId })),
    ).toEqual([
      { id: "loop-1-start", parentId: "loop-1" },
      { id: "loop-1-agent", parentId: "loop-1" },
    ]);
  });

  it("canonicalizes container parentage and persists resized dimensions", () => {
    const workflow = createMockWorkflow("en-US");
    const group = createMockWorkflowLoopGroup({
      sequence: 1,
      position: { x: 360, y: 120 },
      locale: "en-US",
    });
    const loop = group.nodes[0]!;
    const child = group.nodes[1]!;
    child.parentId = undefined;
    loop.width = 840;
    loop.height = 460;
    workflow.nodes.push(child, loop, group.nodes[2]!);

    const definition = normalizeWorkflowDefinition(workflow);

    expect(
      definition.nodes.map((node) => ({
        id: node.id,
        parentId: node.parentId,
        initialWidth: node.initialWidth,
        initialHeight: node.initialHeight,
      })),
    ).toEqual([
      ...workflow.nodes.slice(0, -3).map((node) => ({
        id: node.id,
        parentId: node.parentId,
        initialWidth: node.initialWidth,
        initialHeight: node.initialHeight,
      })),
      {
        id: loop.id,
        parentId: undefined,
        initialWidth: 840,
        initialHeight: 460,
      },
      {
        id: child.id,
        parentId: loop.id,
        initialWidth: undefined,
        initialHeight: undefined,
      },
      {
        id: group.nodes[2]!.id,
        parentId: loop.id,
        initialWidth: undefined,
        initialHeight: undefined,
      },
    ]);
  });

  it("preserves condition source handles and executable cases through normalization", () => {
    const definition = normalizeWorkflowDefinition(createMockWorkflow("en-US"));
    definition.edges[0] = {
      ...definition.edges[0]!,
      sourceHandle: "else",
      targetHandle: undefined,
    };
    const condition = definition.nodes.find(
      (node) => node.data.kind === "condition",
    );
    if (condition !== undefined) {
      condition.data = {
        ...condition.data,
        cases: [
          {
            id: "approved",
            logic: "and",
            conditions: [
              {
                variableSelector: ["review", "structured_output", "approved"],
                operator: "is",
                value: true,
              },
            ],
          },
        ],
      };
    }

    const normalized = normalizeWorkflowDefinition(definition);
    expect(normalized.edges[0]!.sourceHandle).toBe("else");
    const normalizedCondition = normalized.nodes.find(
      (node) => node.data.kind === "condition",
    );
    expect(normalizedCondition?.data.cases).toEqual([
      {
        id: "approved",
        logic: "and",
        conditions: [
          {
            variableSelector: ["review", "structured_output", "approved"],
            operator: "is",
            value: true,
          },
        ],
      },
    ]);
  });

  it("persists iteration geometry and parentId without render-only constraints", () => {
    const workflow = createMockWorkflow("en-US");
    const iteration = {
      id: "iter",
      type: "workflow" as const,
      position: { x: 100, y: 200 },
      initialWidth: 760,
      initialHeight: 420,
      data: { kind: "iteration" as const, title: "Iteration", description: "" },
    };
    const member = {
      id: "member",
      type: "workflow" as const,
      parentId: "iter",
      extent: "parent" as const,
      expandParent: true,
      position: { x: 96, y: 160 },
      data: { kind: "agent" as const, title: "Agent", description: "" },
    };
    workflow.nodes.push(iteration, member);

    const normalized = normalizeWorkflowDefinition(workflow);

    expect(normalized.nodes.at(-2)).toMatchObject({
      id: "iter",
      initialWidth: 760,
      initialHeight: 420,
    });
    expect(normalized.nodes.at(-1)).toMatchObject({
      id: "member",
      parentId: "iter",
    });
    expect(normalized.nodes.at(-1)).not.toHaveProperty("extent");
    expect(normalized.nodes.at(-1)).not.toHaveProperty("expandParent");
  });

  it("rejects cycles before they can leave a run permanently running", () => {
    const definition = normalizeWorkflowDefinition(createMockWorkflow("en-US"));
    const firstNode = definition.nodes[0]!;
    const lastNode = definition.nodes.at(-1)!;
    definition.edges.push({
      id: "cycle",
      source: lastNode.id,
      target: firstNode.id,
    });

    expect(() => validateWorkflowDefinition(definition)).toThrowError(
      expect.objectContaining<Partial<WorkflowDefinitionValidationError>>({
        name: "WorkflowDefinitionValidationError",
        issues: expect.arrayContaining(["graph must be acyclic"]),
      }),
    );
  });

  it("reports duplicate ids and dangling edges at the deploy boundary", () => {
    const definition = normalizeWorkflowDefinition(createMockWorkflow("en-US"));
    definition.nodes.push(structuredClone(definition.nodes[0]!));
    definition.edges.push({
      id: "dangling",
      source: definition.nodes[0]!.id,
      target: "missing-node",
    });

    expect(() => validateWorkflowDefinition(definition)).toThrowError(
      expect.objectContaining<Partial<WorkflowDefinitionValidationError>>({
        issues: expect.arrayContaining([
          expect.stringContaining("duplicate node id"),
          expect.stringContaining("references an unknown node"),
        ]),
      }),
    );
  });
});
