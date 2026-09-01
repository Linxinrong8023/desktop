# Mission: Master Ora's Workflow Module

## Why

Build a source-grounded mental model of Ora's workflow module so the learner can explain it clearly to another engineer, locate failures across UI, transport, application, database, and runtime boundaries, and withstand deep interview follow-up questions without relying on memorized slogans.

## Success looks like

- Explain one workflow from authoring and publishing through run creation, DAG scheduling, node execution, and persisted output
- Read the four workflow tables and reconstruct what happened during a failed run
- Distinguish current executable behavior from editor prototypes, mock-runtime behavior, and planned capability
- Diagnose whether a symptom belongs to graph data, lifecycle state, persistence, scheduling, Agent session execution, or frontend projection
- Answer design and trade-off questions about snapshots, DAG constraints, node state, concurrency, HITL, idempotency, and failure recovery

## Constraints

- Teach in Chinese, beginning with one concrete end-to-end example before introducing terminology
- Stay grounded in the current `C:\Users\mds\ora_desktop\desktop` source tree and mark uncertain or drifting claims
- Keep each lesson short, interactive, and interview-oriented; require retrieval or teach-back before recording mastery
- Separate definition-time, deploy-time, run-time, and presentation-time responsibilities explicitly

## Out of scope

- Implementing a new workflow feature before the current module can be explained and debugged confidently
- Treating prototype-only node metadata or memory-adapter behavior as production capability
- A general survey of every workflow engine or distributed-systems orchestration product
