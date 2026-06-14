//! Round-robin module scheduler.
//!
//! Owns the `RGBMatrix` and lends a `&mut RGBMatrix` to each module in turn,
//! looping forever. Errors are logged; one module's failure never tears down
//! the panel.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::modules::eink_module::DynEinkModule;
use crate::modules::{DynModule, error::ModuleError};
use ohmyoled_matrix::{EinkDisplay, RGBMatrix};

/// Set by the SIGINT/SIGTERM handler in `main.rs`. The e-ink scheduler polls
/// this and clears the panel before exiting — e-paper holds its last image
/// after power-off, unlike the LED panel which a power cut blanks. See
/// [`EINK_ACTIVE`].
pub static SHUTDOWN: AtomicBool = AtomicBool::new(false);

/// True once [`run_eink`] is live and will perform the clear-and-exit itself.
/// The signal handler checks this: when set it defers to the scheduler (so the
/// slow SPI clear runs on the scheduler's thread, never in the handler); when
/// clear it exits immediately, preserving the LED-only fast path.
pub static EINK_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Completes shortly after [`SHUTDOWN`] is set. Polls the flag rather than using
/// `tokio::signal` so the libc handler in `main.rs` stays the single signal
/// owner. Cheap: instantiated per-render inside a `select!` and dropped as soon
/// as the render wins.
async fn await_shutdown() {
    while !SHUTDOWN.load(Ordering::SeqCst) {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

/// Blank the panel and terminate. Runs on the scheduler thread where the display
/// is owned and no render is in flight, so the SPI clear is safe. `_exit` skips
/// async-drop teardown (unreliable across reqwest + tokio under signal),
/// matching the LED handler in `main.rs`. Never returns.
fn clear_and_exit(display: &mut EinkDisplay) -> ! {
    log::info!("eink scheduler: shutdown signal — clearing panel");
    display.clear();
    unsafe { libc::_exit(130) }
}

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
    // Announce that we own the clear-on-exit. A signal can also have arrived
    // during startup, before this point — honor it now.
    EINK_ACTIVE.store(true, Ordering::SeqCst);
    if SHUTDOWN.load(Ordering::SeqCst) {
        clear_and_exit(&mut display);
    }

    if modules.is_empty() {
        log::warn!("eink scheduler: no modules enabled; idling");
        loop {
            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {}
                _ = await_shutdown() => clear_and_exit(&mut display),
            }
        }
    }

    let ids: Vec<&'static str> = modules.iter().map(|m| m.id()).collect();
    log::info!(
        "eink scheduler: starting with {} module(s): {}",
        modules.len(),
        ids.join(", ")
    );

    let mut cycle: u64 = 0;
    'outer: loop {
        cycle = cycle.wrapping_add(1);
        log::debug!("eink scheduler: cycle {cycle} begin");
        let started = std::time::Instant::now();
        for module in modules.iter_mut() {
            log::debug!("eink scheduler: cycle {cycle} -> [{}]", module.id());
            // Race the render against shutdown. The render is only cancellable
            // during its post-push dwell (the frame is already on the panel), so
            // dropping it here never interrupts an in-flight SPI transfer.
            tokio::select! {
                r = module.render_cached(&mut display) => {
                    if let Err(e) = r {
                        log::error!("[{}] eink cycle failed: {e}", module.id());
                    }
                }
                _ = await_shutdown() => break 'outer,
            }
        }
        log::debug!(
            "eink scheduler: cycle {cycle} complete in {} ms",
            started.elapsed().as_millis()
        );
    }
    // Reached only on shutdown: the loop's `break` drops the render future, so
    // the display is free to clear here.
    clear_and_exit(&mut display);
}
