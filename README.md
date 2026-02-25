# Pulsar

An open-source, real-time communication platform built with Rust and SvelteKit.

> ⚠️ **Early development** — not ready for production use.

## Stack

- **Backend**: Rust (axum, tokio)
- **Frontend**: SvelteKit + Svelte 5
- **Voice/Video**: LiveKit (WebRTC)
- **Database**: PostgreSQL → ScyllaDB
- **Message broker**: NATS JetStream

## Building
```bash
cargo build
```

## Running
```bash
RUST_LOG=info cargo run -p pulsar-api
```

## License

[AGPL-3.0-or-later](https://www.gnu.org/licenses/agpl-3.0.html)
