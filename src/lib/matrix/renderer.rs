//! `Renderer` — the trait every matrix renderer implements.
//!
//! A renderer takes the data its paired `Collector` produced and draws frames
//! onto the `RGBMatrix` for one cycle (typically a scroll loop + a dwell).

use crate::matrix::error::RenderError;
use async_trait::async_trait;
use ohmyoled_matrix::RGBMatrix;
use std::time::Duration;

#[async_trait]
pub trait Renderer: Send {
    /// The data this renderer consumes (matches `Collector::Output`).
    type Data: Send + Sync + 'static;

    /// Stable identifier for logging (e.g. `"weather"`).
    fn id(&self) -> &'static str;

    /// Total wall-clock time one full `render()` invocation takes.
    ///
    /// The scheduler uses this to budget panel time per module.
    fn cycle_duration(&self) -> Duration;

    /// Render one complete cycle onto `matrix`. Should clear the panel on exit
    /// so the next module starts from a clean state.
    async fn render(&mut self, matrix: &mut RGBMatrix, data: &Self::Data) -> Result<(), RenderError>;
}
