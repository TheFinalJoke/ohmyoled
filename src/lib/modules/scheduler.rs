//! Round-robin module scheduler.
//!
//! Owns the `RGBMatrix` and lends a `&mut RGBMatrix` to each module in turn,
//! looping forever. Errors are logged; one module's failure never tears down
//! the panel.

use crate::modules::eink_module::DynEinkModule;
use crate::modules::{DynModule, error::ModuleError};
use ohmyoled_matrix::{EinkDisplay, RGBMatrix};

/// Run the scheduler. Returns only on irrecoverable error; SIGINT bypasses this
/// via the libc handler installed in `main.rs`.
pub async fn run(mut matrix: RGBMatrix, mut modules: Vec<Box<dyn DynModule>>) -> Result<(), ModuleError> {
    if modules.is_empty() {
        log::warn!("scheduler: no modules enabled; idling");
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
    }

    let ids: Vec<&'static str> = modules.iter().map(|m| m.id()).collect();
    log::info!(
        "scheduler: starting with {} module(s): {}",
        modules.len(),
        ids.join(", ")
    );

    let mut cycle: u64 = 0;
    loop {
        cycle = cycle.wrapping_add(1);
        log::debug!("scheduler: cycle {cycle} begin");
        let started = std::time::Instant::now();
        for module in modules.iter_mut() {
            log::debug!("scheduler: cycle {cycle} -> [{}]", module.id());
            if let Err(e) = module.render_cached(&mut matrix).await {
                log::error!("[{}] cycle failed: {e}", module.id());
            }
        }
        log::debug!(
            "scheduler: cycle {cycle} complete in {} ms",
            started.elapsed().as_millis()
        );
    }
}

/// Run the e-paper scheduler. Identical round-robin shape to [`run`], but
/// lends a `&mut EinkDisplay` to each module. Each e-ink render composes one
/// static screen, pushes it, and dwells for its `cycle_duration` — so the
/// loop naturally paces itself to the slow refresh of the panel.
pub async fn run_eink(
    mut display: EinkDisplay,
    mut modules: Vec<Box<dyn DynEinkModule>>,
) -> Result<(), ModuleError> {
    if modules.is_empty() {
        log::warn!("eink scheduler: no modules enabled; idling");
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
    }

    let ids: Vec<&'static str> = modules.iter().map(|m| m.id()).collect();
    log::info!(
        "eink scheduler: starting with {} module(s): {}",
        modules.len(),
        ids.join(", ")
    );

    let mut cycle: u64 = 0;
    loop {
        cycle = cycle.wrapping_add(1);
        log::debug!("eink scheduler: cycle {cycle} begin");
        let started = std::time::Instant::now();
        for module in modules.iter_mut() {
            log::debug!("eink scheduler: cycle {cycle} -> [{}]", module.id());
            if let Err(e) = module.render_cached(&mut display).await {
                log::error!("[{}] eink cycle failed: {e}", module.id());
            }
        }
        log::debug!(
            "eink scheduler: cycle {cycle} complete in {} ms",
            started.elapsed().as_millis()
        );
    }
}
