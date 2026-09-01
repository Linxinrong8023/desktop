# Distinguishes known node types from executable node types

The learner now separates graph parsing capability from workflow execution capability.

## Evidence

Given `Start -> Condition -> Output`, the learner correctly identified that `Condition` is recognized by the backend parser but rejected because the current execution engine does not support it. Given `Start -> Junction -> Output`, the learner correctly identified that parsing fails because `Junction` has no backend `NodeType` variant. The learner also explained that these cases should report different errors: one says the engine does not yet support a known node, while the other says the graph contains no recognized node type.

## Implications

Future lessons can assume the learner understands that enum membership, successful parsing, UI configuration, and end-to-end executability are separate levels of implementation maturity.
