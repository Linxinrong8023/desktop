# Uses the Run's frozen Snapshot during debugging

The learner correctly selected the model configuration stored in a failed Run's frozen Snapshot rather than the model currently visible in the editable Draft. This demonstrates that the learner can apply snapshot pinning to a concrete debugging decision instead of treating it as only a versioning definition.

## Evidence

When the Draft used model B but the failed Run's Snapshot used model A, the learner immediately chose model A because execution follows the frozen Snapshot and explicitly rejected using the Draft as evidence.

## Implications

The first lesson's core definition-versus-execution boundary is established. The course can advance to reconstructing a run through the four SQLite tables.
