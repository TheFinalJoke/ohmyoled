use crate::createjson::ui;
use oledlib::api::quake::QuakeFeed;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct QuakeOptions {
    pub run: bool,
    pub feed: QuakeFeed,
    #[serde(default)]
    pub cache_ttl_secs: Option<u64>,
}

impl Default for QuakeOptions {
    fn default() -> Self {
        Self {
            run: true,
            feed: QuakeFeed::default(),
            cache_ttl_secs: None,
        }
    }
}

pub fn configure() -> Result<QuakeOptions, String> {
    ui::section("Earthquake");
    ui::hint("USGS feed — pick how chatty you want the tile to be.");

    let slug = ui::choose(
        "Feed",
        &[
            ("significant_day", "Significant events in the last 24h (quietest)"),
            ("4.5_day", "M ≥ 4.5 in the last 24h"),
            ("2.5_day", "M ≥ 2.5 in the last 24h"),
            ("all_day", "Everything in the last 24h (busiest)"),
        ],
        "significant_day",
    );
    let feed = match slug.as_str() {
        "4.5_day" => QuakeFeed::M45Day,
        "2.5_day" => QuakeFeed::M25Day,
        "all_day" => QuakeFeed::AllDay,
        _ => QuakeFeed::SignificantDay,
    };

    let cache_ttl_secs = ui::read_cache_ttl_secs();
    ui::success(&format!("Earthquake — feed={}", feed.slug()));
    Ok(QuakeOptions { run: true, feed, cache_ttl_secs })
}

pub fn summary_line(opts: &QuakeOptions) -> String {
    format!("quake ({})", opts.feed.slug())
}
