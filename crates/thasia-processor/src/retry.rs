use backoff::{future::retry, ExponentialBackoff};
use std::time::Duration;
use thasia_core::ThasiaError;
use tracing::{error, warn};

pub async fn with_retries<F, Fut, T>(
    context: &str,
    operation: F,
) -> std::result::Result<T, ThasiaError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = std::result::Result<T, backoff::Error<ThasiaError>>>,
{
    let backoff = ExponentialBackoff {
        max_elapsed_time: Some(Duration::from_secs(15)),
        ..Default::default()
    };

    retry(backoff, || async {
        operation().await.map_err(|err| {
            warn!("Transient error in {}: {}. Retrying...", context, err);
            err
        })
    })
    .await
    .map_err(|e| {
        error!("Exhausted retries for {}", context);
        e
    })
}
