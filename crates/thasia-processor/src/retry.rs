use backoff::{ExponentialBackoff, future::retry};
use std::time::Duration;
use tracing::{error, warn};

pub async fn with_retries<F, Fut, T, E>(context: &str, operation: F) -> std::result::Result<T, E>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = std::result::Result<T, backoff::Error<E>>>,
    E: std::error::Error + Send + Sync + 'static,
{
    let backoff = ExponentialBackoff {
        max_elapsed_time: Some(Duration::from_secs(15)),
        ..Default::default()
    };

    retry(backoff, || async {
        operation().await.inspect_err(|err| {
            warn!("Transient error in {}: {}. Retrying...", context, err);
        })
    })
    .await
    .inspect_err(|_| {
        error!("Exhausted retries for {}", context);
    })
}
