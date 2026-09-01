# Understands that NodeRun means an execution attempt

The learner correctly explained that a failed node must have started and therefore must have a WorkflowNodeRun record. NodeRun existence is now understood as evidence of an execution attempt, not evidence of successful completion.

## Evidence

After initially omitting a failed node from the row count, the learner corrected the model with: a node can fail only after it starts, so it necessarily has a record.

## Implications

The course can now move from row-existence semantics to the distinction between NodeRun's summarized execution fields and Session's complete Agent conversation.
