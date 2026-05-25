//! Launch module — next upcoming orbital launch via Launch Library 2.

use crate::api::collector::Collector;
use crate::api::error::ApiError;
use async_trait::async_trait;
use std::time::Duration;

pub mod lldev;
pub mod model;

pub use model::{LaunchStatus, UpcomingLaunch};

use lldev::{LldevConfig, LldevProvider};

pub enum LaunchSource {
    Lldev(LldevProvider),
}

impl LaunchSource {
    pub async fn poll(&self) -> Result<UpcomingLaunch, ApiError> {
        match self {
            Self::Lldev(c) => c.poll().await,
        }
    }
}

pub struct LaunchCollector {
    source: LaunchSource,
}

impl LaunchCollector {
    pub fn from_lldev(cfg: LldevConfig) -> Result<Self, ApiError> {
        Ok(Self {
            source: LaunchSource::Lldev(LldevProvider::new(cfg)?),
        })
    }
}

#[async_trait]
impl Collector for LaunchCollector {
    type Output = UpcomingLaunch;

    fn id(&self) -> &'static str {
        "launch"
    }

    fn refresh_interval(&self) -> Duration {
        // LL2 anonymous tier is ~15 req/hr per API docs; 30 min/poll
        // leaves headroom for several concurrent modules to share the
        // budget. The countdown ticks live on every render frame so we
        // don't need fresher data than this.
        Duration::from_secs(1800)
    }

    async fn poll(&self) -> Result<UpcomingLaunch, ApiError> {
        self.source.poll().await
    }
}
