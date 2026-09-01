# Distinguishes HITL state, node output, and DAG scheduling

The learner now separates interactive-session state from node completion, and separates predecessor status from predecessor data availability.

## Evidence

Given an interactive Agent that ended its first ACP turn, the learner correctly stated that its NodeRun becomes `Pending` and its successor is not scheduled. Given a succeeded predecessor with `outputPolicy = none`, the learner correctly stated that the successor still becomes ready but cannot receive the predecessor's response. The learner also identified that Start and Output nodes have no Session, while Output becomes ready only after its predecessors succeed and can consume only their exposed NodeRun outputs.

## Implications

Future lessons can assume the learner understands that HITL delays the `Succeeded` transition rather than replacing DAG scheduling, that Session history is not duplicated into `WorkflowNodeRun.output`, and that scheduling readiness is independent from downstream handoff availability.
