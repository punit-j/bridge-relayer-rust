pub async fn request_interval(seconds: u64) -> tokio::time::Interval {
    tokio::time::interval_at(
        tokio::time::Instant::now() + tokio::time::Duration::from_secs(seconds),
        tokio::time::Duration::from_secs(seconds),
    )
}
