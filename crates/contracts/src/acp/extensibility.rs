use serde_json::Value;
use std::collections::BTreeMap;

/// Arbitrary extension data carried by the `_meta` field.
pub type Metadata = BTreeMap<String, Value>;
