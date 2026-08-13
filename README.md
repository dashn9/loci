# loco

Opens a browser session through Neurun, navigates, waits ten seconds, exits.

## Run

The control plane has to be up and listening on loopback. Neurun spawns the
browser service itself, so nothing else needs starting.

```sh
export NEURUN_GRPC_ADDRESS=127.0.0.1:7000
export NEURUN_EXECUTION_TOKEN=net_…
cargo run -- bp_01J… https://example.com
```

The profile id can come from `NEURUN_PROFILE_ID` instead of the first argument;
leaving it out opens a plain browser. The URL defaults to `https://example.com`.

## What it does

The [Rust SDK](../sdks/rust-sdk) asks Neurun for a session, gets an id back, and
drives that id. This program never learns where the browser runs and cannot be
told — which is exactly why the dashboard can list this session and stream its
display while it runs.

`execute` carries an opaque command. Neurun brokers sessions rather than browser
semantics and never parses one, so the encoding here is between loco and
`neurun-browser`.

## Credentials

`NEURUN_EXECUTION_TOKEN` is what a worker mints for a real execution. Running
loco by hand means supplying one yourself; there is no API key path, because a
handler is code the control plane started rather than a client that signed in.
