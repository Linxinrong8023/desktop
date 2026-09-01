# Localizes Agent failures by the Session binding checkpoint

The learner now uses `WorkflowNodeRun.status` and `session_id` as durable checkpoints instead of treating every Agent failure as one undifferentiated Session problem.

## Evidence

Given a Running Agent NodeRun with no `session_id`, the learner correctly recognized that the node had entered the persisted scheduling wave while `session/new`, warm, or attach could already have occurred before the database binding. Given a Running or Failed NodeRun with a bound `session_id`, the learner correctly moved the investigation to the post-binding ACP execution chain. For a model-not-advertised failure with no binding, the learner correctly identified the frozen model configuration and the warm Session's configuration options as the relevant evidence, rather than Session conversation history.

## Implications

Future debugging exercises can assume the learner distinguishes warm/attach from database binding, treats `session_id` as proof that the owning prompt crossed admission, and uses the last durable checkpoint to narrow the next inspection surface.
