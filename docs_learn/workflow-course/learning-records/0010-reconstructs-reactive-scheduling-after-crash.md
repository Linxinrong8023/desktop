# Reconstructs reactive scheduling after a crash

The learner correctly reconstructed the scheduler state after a crash between predecessor completion and successor scheduling.

## Evidence

Given a persisted run where Start, Agent A, and Agent B were all `Succeeded`, no Output NodeRun existed, the Run remained `Running`, and `current_nodes` was empty, the learner identified `completed = {Start, A, B}`, `in_flight = {}`, and `ready = {Output}`. They also correctly stated that `resume` would not re-execute the completed nodes and would create only the Output NodeRun.

Before this retrieval, the learner also distinguished a late completion from a duplicate callback: Cancel wins the state transition first, and the later completion observes a non-Running node and becomes a no-op.

## Implications

The learner can now move from ready-set calculation into the Agent node execution lifecycle, Session binding, prompt assembly, output persistence, and failure localization without conflating scheduler resume with workflow restart.
