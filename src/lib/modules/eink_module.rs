//! E-paper module bundling — the e-ink sibling of [`super::Module`].
//!
//! Pairs a [`Collector`] with an [`EinkRenderer`] and reuses the parent
//! module's polling + cold-start machinery ([`super::make_poll_source`] /
//! [`super::acquire_snapshot`]) verbatim. Only the render target differs: an
//! [`EinkDisplay`] instead of an `RGBMatrix`.

use crate::api::Collector;
use crate::matrix::eink_renderer::EinkRenderer;
use crate::modules::error::ModuleError;
use crate::modules::{acquire_snapshot, make_poll_source, PollSource};
use async_trait::async_trait;
use ohmyoled_matrix::EinkDisplay;
use std::sync::Arc;
use std::time::Duration;

/// One e-paper module = one collector + one e-ink renderer whose `Data` align.
pub struct EinkModule<C: Collector, R: EinkRenderer<Data = C::Output>> {
    id: &'static str,
    source: PollSource<C>,
    renderer: R,
    last_good: Option<Arc<C::Output>>,
}

impl<C, R> EinkModule<C, R>
where
    C: Collector + 'static,
    R: EinkRenderer<Data = C::Output>,
    C::Output: Send + Sync + 'static,
{
    /// Build an e-paper module with explicit cache TTL — same semantics as
    /// [`super::Module::new`] (`Duration::ZERO` ⇒ inline polling).
    pub fn new(collector: C, renderer: R, ttl: Duration) -> Self {
        let id = collector.id();
        Self {
            id,
            source: make_poll_source(collector, ttl),
            renderer,
            last_good: None,
        }
    }
}

/// Object-safe view over an [`EinkModule`]. The e-paper scheduler holds
/// `Box<dyn DynEinkModule>`.
#[async_trait]
pub trait DynEinkModule: Send {
    fn id(&self) -> &'static str;
    /// Render the latest cached value onto the e-paper `display`. Pulls from
    /// the watch channel (or inline-polls) just like the LED path.
    async fn render_cached(&mut self, display: &mut EinkDisplay) -> Result<(), ModuleError>;
}

#[async_trait]
impl<C, R> DynEinkModule for EinkModule<C, R>
where
    C: Collector + 'static,
    R: EinkRenderer<Data = C::Output> + 'static,
    C::Output: Send + Sync + 'static,
{
    fn id(&self) -> &'static str {
        self.id
    }

    async fn render_cached(&mut self, display: &mut EinkDisplay) -> Result<(), ModuleError> {
        let id = self.id;
        match acquire_snapshot(id, &mut self.source, &mut self.last_good).await {
            Some(data) => {
                let started = std::time::Instant::now();
                log::debug!("[{id}] eink render begin");
                let r = self
                    .renderer
                    .render(display, data.as_ref())
                    .await
                    .map_err(Into::into);
                log::debug!("[{id}] eink render done in {} ms", started.elapsed().as_millis());
                r
            }
            None => {
                log::warn!("[{id}] no data yet; skipping eink render");
                Ok(())
            }
        }
    }
}
