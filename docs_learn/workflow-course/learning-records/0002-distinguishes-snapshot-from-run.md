# Distinguishes a published Snapshot from a Run

The learner correctly explained that repeatedly starting the same published Snapshot does not mutate or recreate that frozen version; each start creates a separate Run execution record. This establishes the definition/version versus execution-entity boundary needed before learning run and node state machines.

## Evidence

When asked what three starts create, the learner answered that the Snapshot remains unchanged and the Run is the entity that represents each execution.

## Implications

Future lessons can assume the learner distinguishes frozen configuration from an execution instance, while continuing to sharpen the broader meaning of Snapshot because the editable Draft is also stored as a snapshot row.
