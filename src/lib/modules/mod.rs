//! Module bundling: pair a `Collector` with a `Renderer` and the scheduler
//! drives the rest. Everything in the data-fetch + render pipeline is async,
//! built on `async_trait` so the traits stay dyn-safe.
//!
//! To add a new module (e.g. `calendar`):
//!
//! 1. Add a `CalendarOptions` struct to `src/createjson/` with `#[derive(Deserialize)]`.
//! 2. `#[async_trait] impl Collector for CalendarCollector` in `oledlib::api::calendar`.
//! 3. `#[async_trait] impl Renderer for CalendarMatrix` in `oledlib::matrix::calendar`.
//!    Optionally expose `Self::new_async()` if construction does I/O.
//! 4. Add one match arm in [`registry::build`] that `.await`s the constructors
//!    and pushes a `Module::new(collector, renderer)`.
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

/// One module = one collector + one renderer whose `Data` types align.
pub struct Module<C: Collector, R: Renderer<Data = C::Output>> {
    pub collector: C,
    pub renderer: R,
    last_good: Option<C::Output>,
}

impl<C, R> Module<C, R>
where
    C: Collector,
    R: Renderer<Data = C::Output>,
{
    pub fn new(collector: C, renderer: R) -> Self {
        Self {
            collector,
            renderer,
            last_good: None,
        }
    }
}

/// Object-safe view over a `Module`. The scheduler holds `Box<dyn DynModule>`.
#[async_trait]
pub trait DynModule: Send {
    fn id(&self) -> &'static str;
    async fn poll_and_render(&mut self, matrix: &mut RGBMatrix) -> Result<(), ModuleError>;
}

#[async_trait]
impl<C, R> DynModule for Module<C, R>
where
    C: Collector + 'static,
    R: Renderer<Data = C::Output> + 'static,
    C::Output: Send + Sync,
{
    fn id(&self) -> &'static str {
        self.collector.id()
    }

    async fn poll_and_render(&mut self, matrix: &mut RGBMatrix) -> Result<(), ModuleError> {
        // Try a fresh poll. On success, replace the cached value.
        // On failure, fall back to the last good value so the panel stays alive.
        match self.collector.poll().await {
            Ok(data) => self.last_good = Some(data),
            Err(e) => {
                log::warn!("[{}] poll failed: {e}; using last-good data", self.collector.id());
            }
        }

        match &self.last_good {
            Some(data) => self.renderer.render(matrix, data).await.map_err(Into::into),
            None => {
                log::warn!("[{}] no data yet; skipping render", self.collector.id());
                Ok(())
            }
        }
    }
}
