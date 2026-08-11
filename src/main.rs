//! Opens a browser wearing a Neurun profile, points it at a page, and leaves.
//!
//! That is the whole program for now: `OpenSession` through the SDK, one
//! `Page.navigate` over the CDP endpoint it hands back, ten seconds to look at,
//! then a close that stores whatever the browser picked up.
//!
//! ```sh
//! NEURUN_URL=http://localhost:8080 NEURUN_API_KEY=neu_… \
//!   cargo run -- bp_01J… https://example.com
//! ```
//!
//! `neurun-browser` has to already be listening on loopback — the control plane
//! never opens a browser, so nothing else will start it.
//!
//! The CDP client is a few lines down there rather than rustenium: the SDK's
//! contract ends at the endpoint URL, and three commands do not need a browser
//! automation library behind them.

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use neurun::{BrowserProfiles, Protocol};
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

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
        .ok_or("a browser profile is required: pass its id, or set NEURUN_PROFILE_ID")?;
    let url = arguments.next().unwrap_or_else(|| DEFAULT_URL.to_string());

    let profiles = BrowserProfiles::from_env()?;
    let session = profiles.open(&profile_id).await?;
    println!(
        "{} is open on {} ({:?})",
        session.profile().name,
        session.endpoint_url(),
        session.protocol()
    );

    // Firefox answers BiDi, and carries no profile either way.
    if session.protocol() != Protocol::Cdp {
        let protocol = session.protocol();
        session.discard().await?;
        return Err(format!("loco drives CDP, and this profile opened {protocol:?}").into());
    }

    match visit(session.endpoint_url(), &url).await {
        Ok(()) => {
            let close = session.close().await?;
            match close.unsaved {
                None => println!("stored {} cookies", close.state.cookies.len()),
                Some(reason) => println!("stored nothing: {reason:?}"),
            }
            Ok(())
        }
        Err(error) => {
            // The run already failed; a noisy shutdown must not replace why.
            let _ = session.discard().await;
            Err(error)
        }
    }
}

/// Navigates the page the browser already has open, and waits there.
///
/// That page rather than a new tab: the browser server captures DOM storage by
/// evaluating a script on the page its own CDP session is attached to, so a
/// second tab would close with the cookies but none of the storage.
async fn visit(endpoint: &str, url: &str) -> Result<()> {
    let mut cdp = Cdp::connect(endpoint).await?;

    let targets = cdp.call("Target.getTargets", json!({}), None).await?;
    let page = targets["targetInfos"]
        .as_array()
        .and_then(|targets| targets.iter().find(|target| target["type"] == "page"))
        .and_then(|target| target["targetId"].as_str())
        .ok_or("the browser opened no page to navigate")?
        .to_string();

    let attached = cdp
        .call(
            "Target.attachToTarget",
            json!({ "targetId": page, "flatten": true }),
            None,
        )
        .await?;
    let page_session = attached["sessionId"]
        .as_str()
        .ok_or("attaching to the page handed back no session")?
        .to_string();

    println!("navigating to {url}");
    let navigated = cdp
        .call("Page.navigate", json!({ "url": url }), Some(&page_session))
        .await?;
    // A navigation that never left the ground still answers Ok, with the reason
    // tucked inside the result.
    if let Some(reason) = navigated["errorText"].as_str() {
        return Err(format!("{url}: {reason}").into());
    }

    println!("holding for {}s", LINGER.as_secs());
    tokio::time::sleep(LINGER).await;
    Ok(())
}

/// Enough CDP to send a command and wait for its answer.
struct Cdp {
    socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
    last_id: u64,
}

impl Cdp {
    async fn connect(endpoint: &str) -> Result<Self> {
        let (socket, _) = tokio_tungstenite::connect_async(endpoint).await?;
        Ok(Self { socket, last_id: 0 })
    }

    /// Sends one command and returns its result.
    ///
    /// `session` scopes the command to an attached target. The attachment is
    /// flat, so that answer comes back down this same socket carrying the id it
    /// was sent with; anything else arriving meanwhile is an event, and this
    /// program has subscribed to none.
    async fn call(&mut self, method: &str, params: Value, session: Option<&str>) -> Result<Value> {
        self.last_id += 1;
        let id = self.last_id;
        let mut command = json!({ "id": id, "method": method, "params": params });
        if let Some(session) = session {
            command["sessionId"] = json!(session);
        }
        self.socket.send(Message::text(command.to_string())).await?;

        while let Some(received) = self.socket.next().await {
            let Message::Text(text) = received? else {
                continue;
            };
            let message: Value = serde_json::from_str(&text)?;
            if message["id"] != json!(id) {
                continue;
            }
            if let Some(error) = message.get("error") {
                return Err(format!("{method}: {error}").into());
            }
            return Ok(message["result"].clone());
        }
        Err(format!("{method}: the browser closed the connection").into())
    }
}

fn environment(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
