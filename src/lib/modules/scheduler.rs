//! Round-robin module scheduler.
//!
//! Owns the `RGBMatrix` and lends a `&mut RGBMatrix` to each module in turn,
//! looping forever. Errors are logged; one module's failure never tears down
//! the panel.

use crate::modules::{DynModule, error::ModuleError};
use ohmyoled_matrix::RGBMatrix;

/// Run the scheduler. Returns only on irrecoverable error; SIGINT bypasses this
/// via the libc handler installed in `main.rs`.
pub async fn run(mut matrix: RGBMatrix, mut modules: Vec<Box<dyn DynModule>>) -> Result<(), ModuleError> {
    if modules.is_empty() {
        log::warn!("scheduler: no modules enabled; idling");
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
    }

    loop {
        for module in modules.iter_mut() {
            if let Err(e) = module.poll_and_render(&mut matrix).await {
                log::error!("[{}] cycle failed: {e}", module.id());
            }
        }
    }
}
