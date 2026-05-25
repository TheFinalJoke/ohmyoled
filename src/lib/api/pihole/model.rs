//! Normalized Pi-hole summary stats.

#[derive(Debug, Clone, Copy)]
pub struct PiholeSummary {
    /// Percent of today's DNS queries that were blocked (0.0..=100.0).
    pub percent_blocked: f32,
    /// Total DNS queries Pi-hole has processed today.
    pub queries_today: u32,
    /// Subset of `queries_today` that hit the block list.
    pub blocked_today: u32,
    /// Distinct client devices Pi-hole has seen today. Useful for at-
    /// a-glance "how busy is the network".
    pub unique_clients: u32,
}
