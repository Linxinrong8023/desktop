import type {
  GraphWorkflowNodeState,
  GraphWorkflowRound,
  GraphWorkflowRun,
} from "@ora/workflow-runtime";

export type LoopRoundSelection = Readonly<Record<string, string>>;

/** Resolves one Loop's explicit round selection, falling back to its latest round. */
export function selectedLoopRound(
  rounds: GraphWorkflowRound[],
  loopNodeId: string,
  selection: LoopRoundSelection,
): GraphWorkflowRound | undefined {
  const loopRounds = rounds.filter(
    (round) => round.parentLoopNodeId === loopNodeId,
  );
  const selectedRoundId = selection[loopNodeId];
  return (
    loopRounds.find((round) => round.id === selectedRoundId) ??
    loopRounds.reduce<GraphWorkflowRound | undefined>(
      (latest, round) =>
        latest === undefined || round.roundIndex > latest.roundIndex
          ? round
          : latest,
      undefined,
    )
  );
}

/**
 * Projects one selected round per Loop onto the Theater's flat node-state view.
 * Persisted child runs stay isolated in round history; this projection only decides
 * which occurrence is visible in cards, path status, focus, and session details.
 */
export function projectLoopRoundNodeStates(
  run: GraphWorkflowRun,
  selection: LoopRoundSelection,
): Record<string, GraphWorkflowNodeState> {
  const rounds = run.rounds ?? [];
  if (rounds.length === 0) {
    return run.nodeStates;
  }

  const loopNodeIds = new Set(rounds.map((round) => round.parentLoopNodeId));
  const nodeStates = { ...run.nodeStates };
  for (const loopNodeId of loopNodeIds) {
    const round = selectedLoopRound(rounds, loopNodeId, selection);
    if (round !== undefined) {
      Object.assign(nodeStates, round.nodeStates);
    }
  }
  return nodeStates;
}
