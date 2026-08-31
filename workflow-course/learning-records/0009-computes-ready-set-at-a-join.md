# Computes the ready set at a join

The learner correctly computed the scheduler state for a parallel join: `Start` and `Agent A` were completed, `Agent B` was in flight while pending for human input, and the ready set was empty.

## Evidence

Given a graph where `Output` depends on both Agent A and Agent B, the learner explained that Output was not ready because Agent B had not completed. This demonstrates use of the all-direct-predecessors rule rather than treating any successful predecessor as sufficient.

## Implications

The course can now deepen reactive scheduling into persistence transactions, `current_nodes`, idempotent callbacks, cancellation, and crash recovery while continuing to test ready-set calculations in more complex graphs.
