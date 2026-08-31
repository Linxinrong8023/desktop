# Understands lazy WorkflowNodeRun records

The learner correctly explained that a Pending Run is a valid persisted execution instance while the absence of WorkflowNodeRun rows means no node has started, not that data was lost. The learner also distinguished the Run entity from its `pending` status.

## Evidence

Given a persisted Pending Run with zero node-run rows, the learner reasoned that node records should not exist before node execution begins and identified `pending` as one valid Run status.

## Implications

Future lessons can introduce lazy node-run creation, graph-versus-record projection, and parallel scheduling without reteaching the entity-versus-status distinction.
