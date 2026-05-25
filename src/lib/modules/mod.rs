//! Module bundling: pair a `Collector` with a `Renderer` and the scheduler
//! drives the rest. Everything in the data-fetch + render pipeline is async,
//! built on `async_trait` so the traits stay dyn-safe.
//!
//! ## Polling model
//!
//! Each module gets its own background tokio task that polls the collector
//! on a TTL schedule (defaults to `collector.refresh_interval()`, overridable
//! per-section via `cache_ttl_secs`). The fresh value is published into a
//! `tokio::sync::watch` channel; the scheduler's render loop reads the
//! current value via `borrow()` — never blocks on network.
//!
//! Setting `cache_ttl_secs: 0` opts the section out of the background task:
//! the renderer calls `collector.poll().await` inline on every render. Slow
//! (the panel stalls during the round-trip) but always shows freshest data.
//!
//! ## Adding a new module (e.g. `calendar`)
//!
//! 1. Add a `CalendarOptions` struct to `src/createjson/` with `#[derive(Deserialize)]`.
//! 2. `#[async_trait] impl Collector for CalendarCollector` in `oledlib::api::calendar`.
//! 3. `#[async_trait] impl Renderer for CalendarMatrix` in `oledlib::matrix::calendar`.
//!    Optionally expose `Self::new_async()` if construction does I/O.
//! 4. Add one match arm in [`registry::build`] that `.await`s the constructors
//!    and calls `Module::new(collector, renderer, ttl)`.
//!
//! The [`scheduler::run`] loop picks it up automatically.

pub mod error;
pub mod registry;
pub mod scheduler;

use crate::api::Collector;
use crate::matrix::Renderer;
use crate::modules::error::ModuleError;
use async_trait::async_trait;
use ohmyoled_matrix::RGBMatrix;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;

/// Where this module gets its data each render:
/// - `Cached`: a background tokio task owns the collector and writes the
///   latest value into a `watch::channel`; this slot reads from the other
///   end. Renders never block on network, even for slow APIs.
/// - `Inline`: no background task (the user opted out with `cache_ttl_secs: 0`).
///   The renderer calls `collector.poll().await` directly on each render.
///   Always-fresh data; panel stalls for the duration of the round-trip.
enum PollSource<C: Collector> {
    Cached(watch::Receiver<Option<Arc<C::Output>>>),
    Inline(C),
}

/// One module = one collector + one renderer whose `Data` types align.
pub struct Module<C: Collector, R: Renderer<Data = C::Output>> {
    id: &'static str,
    source: PollSource<C>,
    renderer: R,
    last_good: Option<Arc<C::Output>>,
}

impl<C, R> Module<C, R>
where
    C: Collector + 'static,
    R: Renderer<Data = C::Output>,
    C::Output: Send + Sync + 'static,
{
    /// Build a module with explicit cache TTL.
    ///
    /// - `Duration::ZERO` ⇒ no background task; the renderer calls
    ///   `collector.poll().await` inline on every render.
    /// - Any other duration ⇒ spawn a background poll task that sleeps
    ///   for `ttl` between polls; renders read the cached value.
    pub fn new(collector: C, renderer: R, ttl: Duration) -> Self {
        let id = collector.id();
        if ttl.is_zero() {
            return Self {
                id,
                source: PollSource::Inline(collector),
                renderer,
                last_good: None,
            };
        }
        let (tx, rx) = watch::channel::<Option<Arc<C::Output>>>(None);
        tokio::spawn(background_poll(collector, tx, ttl, id));
        Self {
            id,
            source: PollSource::Cached(rx),
            renderer,
            last_good: None,
        }
    }
}

/// Background poll loop — owns the collector, writes each fresh value
/// into `tx` as an `Arc`, sleeps `ttl` between polls. Exits cleanly if
/// every receiver has been dropped (panel shutdown).
async fn background_poll<C>(
    collector: C,
    tx: watch::Sender<Option<Arc<C::Output>>>,
    ttl: Duration,
    id: &'static str,
) where
    C: Collector + 'static,
    C::Output: Send + Sync + 'static,
{
    log::debug!("[{id}] background poll task started (ttl={:?})", ttl);
    loop {
        if tx.is_closed() {
            log::debug!("[{id}] watch receivers dropped; stopping poll task");
            return;
        }
        let started = std::time::Instant::now();
        match collector.poll().await {
            Ok(data) => {
                log::debug!(
                    "[{id}] poll ok in {} ms",
                    started.elapsed().as_millis()
                );
                let _ = tx.send(Some(Arc::new(data)));
            }
            Err(e) => {
                log::warn!("[{id}] poll failed: {e}; keeping previous cached value");
            }
        }
        tokio::time::sleep(ttl).await;
    }
}

/// Object-safe view over a `Module`. The scheduler holds `Box<dyn DynModule>`.
#[async_trait]
pub trait DynModule: Send {
    fn id(&self) -> &'static str;
    /// Render the latest cached value. Pulls from the watch::channel
    /// (or inline-polls if the module opted out of caching) and hands
    /// it to the renderer. Never blocks on network in the cached path.
    async fn render_cached(&mut self, matrix: &mut RGBMatrix) -> Result<(), ModuleError>;
}

#[async_trait]
impl<C, R> DynModule for Module<C, R>
where
    C: Collector + 'static,
    R: Renderer<Data = C::Output> + 'static,
    C::Output: Send + Sync + 'static,
{
    fn id(&self) -> &'static str {
        self.id
    }

    async fn render_cached(&mut self, matrix: &mut RGBMatrix) -> Result<(), ModuleError> {
        let id = self.id;
        // Pick up the latest snapshot (background or inline).
        let snapshot: Option<Arc<C::Output>> = match &mut self.source {
            PollSource::Cached(rx) => rx.borrow().clone(),
            PollSource::Inline(collector) => {
                let started = std::time::Instant::now();
                match collector.poll().await {
                    Ok(data) => {
                        log::debug!(
                            "[{id}] inline poll ok in {} ms",
                            started.elapsed().as_millis()
                        );
                        Some(Arc::new(data))
                    }
                    Err(e) => {
                        log::warn!("[{id}] inline poll failed: {e}; using last-good");
                        None
                    }
                }
            }
        };
        if snapshot.is_some() {
            self.last_good = snapshot;
        }
        match &self.last_good {
            Some(data) => {
                let started = std::time::Instant::now();
                log::debug!("[{id}] render begin");
                let r = self.renderer.render(matrix, data.as_ref()).await.map_err(Into::into);
                log::debug!(
                    "[{id}] render done in {} ms",
                    started.elapsed().as_millis()
                );
                r
            }
            None => {
                log::warn!("[{id}] no data yet; skipping render");
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::error::ApiError;
    use crate::matrix::error::RenderError;
    use ohmyoled_matrix::{MatrixOptions, RGBMatrix};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// Collector that just counts the number of times `poll()` is called.
    struct CountingCollector {
        polls: Arc<AtomicUsize>,
        interval: Duration,
    }

    #[async_trait]
    impl Collector for CountingCollector {
        type Output = u32;
        fn id(&self) -> &'static str {
            "counter"
        }
        fn refresh_interval(&self) -> Duration {
            self.interval
        }
        async fn poll(&self) -> Result<Self::Output, ApiError> {
            let n = self.polls.fetch_add(1, Ordering::SeqCst);
            Ok(n as u32)
        }
    }

    /// Renderer that just counts how often `render` was called.
    struct CountingRenderer {
        renders: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl Renderer for CountingRenderer {
        type Data = u32;
        fn id(&self) -> &'static str {
            "counter"
        }
        fn cycle_duration(&self) -> Duration {
            Duration::from_millis(1)
        }
        async fn render(
            &mut self,
            _matrix: &mut RGBMatrix,
            _data: &u32,
        ) -> Result<(), RenderError> {
            self.renders.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn cached_module_renders_without_polling_inline() {
        // With a non-zero TTL, a background task owns the collector;
        // render_cached pulls from the watch::channel and never polls
        // inline. Verify by checking poll count stays at 1 across
        // many renders inside the first TTL window.
        let polls = Arc::new(AtomicUsize::new(0));
        let renders = Arc::new(AtomicUsize::new(0));
        let mut module = Module::new(
            CountingCollector {
                polls: polls.clone(),
                interval: Duration::from_secs(60),
            },
            CountingRenderer {
                renders: renders.clone(),
            },
            Duration::from_secs(60),
        );
        // Yield so the spawned poll task runs its first poll.
        tokio::task::yield_now().await;
        tokio::time::sleep(Duration::from_millis(1)).await;

        let mut matrix = RGBMatrix::test(MatrixOptions::default());
        for _ in 0..10 {
            module.render_cached(&mut matrix).await.unwrap();
        }
        // Exactly one poll from the background task; ten renders.
        assert_eq!(polls.load(Ordering::SeqCst), 1, "background task should poll once before sleeping for the 60 s ttl");
        assert_eq!(renders.load(Ordering::SeqCst), 10);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn inline_module_polls_every_render() {
        // `Duration::ZERO` opts the section out of background caching:
        // the renderer calls collector.poll() on every render.
        let polls = Arc::new(AtomicUsize::new(0));
        let renders = Arc::new(AtomicUsize::new(0));
        let mut module = Module::new(
            CountingCollector {
                polls: polls.clone(),
                interval: Duration::from_secs(60),
            },
            CountingRenderer {
                renders: renders.clone(),
            },
            Duration::ZERO,
        );
        let mut matrix = RGBMatrix::test(MatrixOptions::default());
        for _ in 0..5 {
            module.render_cached(&mut matrix).await.unwrap();
        }
        assert_eq!(polls.load(Ordering::SeqCst), 5);
        assert_eq!(renders.load(Ordering::SeqCst), 5);
    }
}
