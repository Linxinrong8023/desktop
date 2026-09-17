# Separates graph storage from DAG validation

The learner correctly explained that SQLite may successfully store a graph containing a cycle because the graph column is opaque JSON, while the workflow-run application layer rejects that graph when parsing and validating it as a DAG. The learner now distinguishes persistence validity from workflow executability.

## Evidence

Given `A -> B -> A`, the learner stated that storage success does not indicate database corruption and that the cycle must be rejected when the graph is parsed for execution.

## Implications

Future lessons can assume the learner understands the storage-versus-domain-validation boundary. Continue to qualify that SQLite still enforces relational schema constraints even though it does not interpret graph semantics.
