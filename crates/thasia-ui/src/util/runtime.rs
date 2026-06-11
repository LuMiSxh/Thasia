use crate::error::{AppError, AppResult};
use std::{future::Future, sync::OnceLock};
use tokio::runtime::{Handle, Runtime};

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

pub fn init() -> AppResult<()> {
    let runtime = Runtime::new().map_err(AppError::Io)?;
    RUNTIME
        .set(runtime)
        .map_err(|_| AppError::Message("Tokio runtime was initialized twice.".into()))
}

pub async fn app<F, T>(future: F) -> AppResult<T>
where
    F: Future<Output = AppResult<T>> + Send + 'static,
    T: Send + 'static,
{
    handle()
        .spawn(future)
        .await
        .map_err(|error| AppError::Message(format!("Backend task failed: {error}")))?
}

pub async fn value<F, T>(future: F) -> AppResult<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    handle()
        .spawn(future)
        .await
        .map_err(|error| AppError::Message(format!("Backend task failed: {error}")))
}

fn handle() -> Handle {
    RUNTIME
        .get()
        .expect("Tokio runtime must be initialized before opening the UI")
        .handle()
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_runtime_supports_tokio_reactor_operations() {
        init().expect("runtime");
        let result = RUNTIME
            .get()
            .expect("runtime")
            .block_on(app(async {
                let value = tokio::task::spawn_blocking(|| 42)
                    .await
                    .map_err(|error| AppError::Message(error.to_string()))?;
                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                Ok(value)
            }))
            .expect("backend operation");
        assert_eq!(result, 42);
    }
}
