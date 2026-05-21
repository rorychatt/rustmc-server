use std::future::Future;

#[allow(dead_code)]
pub async fn retry_test<F, Fut>(test_name: &str, max_attempts: u32, f: F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
{
    let mut last_err = None;
    for attempt in 1..=max_attempts {
        match f().await {
            Ok(()) => return,
            Err(e) => {
                let msg = e.to_string();
                let is_transient = msg.contains("os error 10054")
                    || msg.contains("connection reset")
                    || msg.contains("ConnectionReset");
                if is_transient && attempt < max_attempts {
                    eprintln!("{test_name} attempt {attempt} failed transiently: {e}, retrying...");
                    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                    last_err = Some(e);
                    continue;
                }
                panic!("{test_name} failed after {attempt} attempts: {e}");
            }
        }
    }
    panic!(
        "{test_name} failed after {max_attempts} attempts: {}",
        last_err.unwrap()
    );
}
