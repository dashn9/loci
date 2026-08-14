//! Opens a browser session through Neurun, drives it, and leaves.
//!
//! ```sh
//! NEURUN_GRPC_ADDRESS=127.0.0.1:7000 NEURUN_EXECUTION_TOKEN=net_… \
//!   cargo run -- bp_01J… https://example.com
//! ```
//!
//! The control plane has to already be listening on loopback. Neurun is the
//! broker: this program never learns where the browser runs, and cannot be told.

use std::time::Duration;

use neurun::Browser;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// How long the page stays up before the session closes.
const LINGER: Duration = Duration::from_secs(10);

const DEFAULT_URL: &str = "https://example.com";

#[tokio::main]
async fn main() -> Result<()> {
    let mut arguments = std::env::args().skip(1);
    let profile_id = arguments
        .next()
        .or_else(|| environment("NEURUN_PROFILE_ID"))
        .unwrap_or_default();
    let url = arguments.next().unwrap_or_else(|| DEFAULT_URL.to_string());

    let mut session = Browser::from_env()?
        .open_with("chrome", &profile_id)
        .await?;
    println!(
        "session {} is {} ({})",
        session.id(),
        session.info().status,
        if profile_id.is_empty() {
            "a plain browser".to_string()
        } else {
            format!("wearing {profile_id}")
        }
    );

    let outcome = visit(&mut session, &url).await;

    // A session left to expire is correct but slow: the dashboard shows a
    // browser that is not there until the lease runs out. So close either way,
    // and do not let a noisy shutdown replace why the run failed.
    match session.close().await {
        Ok(()) => outcome,
        Err(error) => outcome.and(Err(error.into())),
    }
}

async fn visit(session: &mut neurun::Session, url: &str) -> Result<()> {
    println!("navigating to {url}");
    session.navigate(url).await?;
    session.wait_for_navigation().await?;

    println!("holding for {}s", LINGER.as_secs());
    tokio::time::sleep(LINGER).await;
    Ok(())
}

fn environment(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
