//! Common-trait module retained for compatibility.
//!
//! After the serde migration the hand-rolled `IntoJson`/`ApiOptions` traits are
//! no longer needed — `serde::Serialize`/`Deserialize` derives replace them.
//! This file is kept as an empty placeholder so external imports don't break.
