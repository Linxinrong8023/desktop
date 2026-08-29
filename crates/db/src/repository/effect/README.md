# SQLite Effect repository

This module implements the deep `ora-effect::EffectRepository` persistence seam plus Source and
Consumer declaration transactions.

The files are split by invariant boundary:

- `source` publishes immutable Source Revisions and advances each changed Scope once.
- `declaration` persists Consumer Revisions and converges `(Scope, Consumer)` Targets and shared
  Resource bindings.
- `queue` owns level-triggered Target scheduling, Target/Resource claims, blocking, and no-mutation
  commits.
- `journal` owns immutable Attempt preparation, monotonic progress, and atomic finalization.
- `claims` centralizes fenced authority checks and ownership-ledger snapshot loading.
- `projection_persistence` stores digest-addressed snapshots with exact replay validation;
  `ledger_validation` ties ownership transitions back to those snapshots and Operation evidence.
- `persistence` stores operation journals, ledgers, receipts, and status/request transitions;
  `validation` enforces Scope and Resource authority at each aggregate write boundary.
- `recovery` turns unfinished journals into explicit `RecoveryRequired` state and owns exact
  Artifact cleanup transitions.
- `mapping` validates rows at the database/domain boundary; `store` implements Desired CAS and the
  public repository trait.

Immediate write transactions keep every compare-and-swap, claim, publication, blocking transition,
retry, recovery quarantine, and finalization check atomic with its writes. No database transaction
is held across filesystem or plugin calls. Observed and Preserved state remain live adapter facts and
are never persisted as ownership.
