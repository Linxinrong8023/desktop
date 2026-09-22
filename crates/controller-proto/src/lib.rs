//! Generated client of the Cloud internal control contract (`ora.cloud.internal.v1`). The `.proto`
//! files live in the Cloud repository and are pinned by the `third_party/cloud` submodule;
//! regenerate with `task proto:generate` and never edit `src/gen` by hand.
#![allow(clippy::all, clippy::pedantic, clippy::nursery)]

/// `ora.cloud.internal.v1`: lease, execution coordination and control signals as a Controller
/// consumes them. The prost output pulls in the tonic client module generated beside it.
pub mod v1 {
    include!("gen/ora/cloud/internal/v1/ora.cloud.internal.v1.rs");
}
