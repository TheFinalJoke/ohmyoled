//! `EinkRenderer` — the trait every e-paper renderer implements.
//!
//! The e-paper sibling of [`crate::matrix::Renderer`]. Where the LED
//! `Renderer` runs a sub-second animated scroll loop against an
//! [`ohmyoled_matrix::RGBMatrix`], an `EinkRenderer` paints **one static,
//! full-resolution screen** onto an [`ohmyoled_matrix::EinkDisplay`] and lets
//! it dwell — e-paper refreshes slowly and persists, so there is no animation.
//!
//! Renderers keep the same `frame(&self, data, w, h) -> RgbImage` shape used
//! everywhere else (composed *white-foreground on black*, the convention the
//! `draw_*` primitives expect); [`ohmyoled_matrix::EinkDisplay::show`] handles
//! thresholding that to black-ink-on-white and pushing it to the panel.

use crate::matrix::error::RenderError;
use async_trait::async_trait;
use ohmyoled_matrix::EinkDisplay;
use std::time::Duration;

#[async_trait]
pub trait EinkRenderer: Send {
    /// The data this renderer consumes (matches `Collector::Output`).
    type Data: Send + Sync + 'static;

    /// Stable identifier for logging (e.g. `"weather"`).
    fn id(&self) -> &'static str;

    /// How long this static screen dwells before the scheduler moves on.
    ///
    /// Much longer than an LED cycle — e-paper refresh is slow and the image
    /// persists, so a tile typically stays up for tens of seconds to minutes.
    fn cycle_duration(&self) -> Duration;

    /// Compose one full screen and push it to `display`. Implementations build
    /// an `RgbImage` at `display.width() × display.height()`, call
    /// `display.show(&img)`, then dwell for [`Self::cycle_duration`].
    async fn render(
        &mut self,
        display: &mut EinkDisplay,
        data: &Self::Data,
    ) -> Result<(), RenderError>;
}

/// Sleep until the next whole-second boundary. Per-second tick renderers (the
/// clock, the launch countdown) call this between fast refreshes so their
/// updates stay aligned to the wall clock even as the panel refresh consumes
/// part of each second.
pub async fn sleep_to_next_second() {
    use chrono::Timelike;
    let ns = chrono::Local::now().nanosecond().min(999_999_999);
    tokio::time::sleep(Duration::from_nanos((1_000_000_000 - ns) as u64)).await;
}
