# loco

Opens a browser wearing a Neurun profile, navigates, waits ten seconds, exits.

## Run

Two things have to be up first: the Neurun API, and `neurun-browser` on
loopback. The control plane never opens a browser, so nothing else will start
that server.

```sh
cd ../browser && cargo run          # 127.0.0.1:1268

export NEURUN_URL=http://localhost:8080
export NEURUN_API_KEY=neu_…         # needs browser_profiles:write
cargo run -- bp_01J… https://example.com
```

The profile id can come from `NEURUN_PROFILE_ID` instead of the first argument;
the URL defaults to `https://example.com`. `NEURUN_BROWSER_ADDR` moves the
browser server, and must stay on loopback.

## What it does

The [Rust SDK](../sdks/rust-sdk) reads the profile and its cookies, opens a
session carrying both, and hands back a CDP endpoint. Driving that endpoint is
this program's half of the loop: attach to the page the browser already has
open, `Page.navigate`, wait, then close — which is what stores whatever the
browser picked up.

The page it already has open, rather than a new tab, because the browser server
captures DOM storage from the page its own CDP session is attached to. A second
tab would close with the cookies but none of the storage.

Chrome only, for now. A Firefox profile opens a BiDi endpoint and closes
carrying nothing, so it exits rather than pretending otherwise.
