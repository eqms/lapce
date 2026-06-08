use anyhow::Result;

/// Thin wrapper around the shared async download core in
/// `lapce-proxy`. All HTTP I/O is delegated to
/// `lapce_proxy::get_url`; no second `reqwest::Client` is
/// constructed here (RT-03, D-02).
pub struct DownloadPipeline;

impl DownloadPipeline {
    pub fn get(
        url: impl reqwest::IntoUrl + Clone,
        user_agent: Option<&str>,
    ) -> Result<reqwest::Response> {
        lapce_proxy::get_url(url, user_agent)
    }
}
