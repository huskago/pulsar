# Pulsar

An open-source, self-hosted communication platform, real-time chat, voice, and video. Built with Rust, designed for privacy and security.

> Early development. Not production-ready.

The official client is **[Orbit](https://git.huskago.ovh/huskago/orbit)**, a spatial, multi-server interface built with SolidJS + Tauri.

## Stack

| Layer | Technology |
|---|---|
| API | Rust · axum · tokio |
| Gateway (WebSocket) | Rust · axum |
| Messages | ScyllaDB (encrypted at rest) |
| Relational data | PostgreSQL |
| Pub/sub | NATS JetStream |
| File storage | MinIO (S3-compatible, encrypted at rest) |
| Sessions / DEK cache | Redis |
| Voice / Video | LiveKit (WebRTC) |

## Security

- Messages and files are encrypted at rest with AES-256-GCM using per-channel keys (envelope encryption: DEK/KEK pattern)
- Auth uses short-lived JWTs + per-device refresh tokens with rotation on every use
- JWT revocation via Redis blocklist on logout
- MinIO bucket is private, files are served through presigned URLs (15 min TTL)

## Quick start

```bash
cp .env.example .env
# Fill in JWT_SECRET, KEK_SECRET (64 hex chars), and LIVEKIT_URL
docker compose up -d
```

The API is available at `http://localhost:3000` and the gateway at `ws://localhost:3001/gateway`.

## Local development

Start only the backing services:

```bash
docker compose up -d db nats minio scylladb redis livekit
```

Then run the backend:

```bash
# API (port 3000)
RUST_LOG=info cargo run -p pulsar-api

# Gateway (port 3001)
RUST_LOG=info cargo run -p pulsar-gateway
```

For the client, see [Orbit](https://git.huskago.ovh/huskago/orbit).

## Environment variables

Copy `.env.example` to `.env`. Required variables:

| Variable | Description                                                |
|---|------------------------------------------------------------|
| `DATABASE_URL` | PostgreSQL connection string                               |
| `JWT_SECRET` | Secret for signing JWTs (any strong random string)         |
| `KEK_SECRET` | 64 hex characters (32 bytes), key encryption key           |
| `LIVEKIT_URL` | Publicly reachable LiveKit URL (browser connects directly) |
| `LIVEKIT_API_KEY` / `LIVEKIT_API_SECRET` | LiveKit credentials                                        |
| `STORAGE_ENDPOINT` | MinIO URL (default: `http://localhost:9000`)               |

## Related

- [Orbit](https://git.huskago.ovh/huskago/orbit) - official client (SolidJS + Tauri)
- [Pulsar](https://git.huskago.ovh/huskago/pulsar) - this repository

## License

[AGPL-3.0-or-later](https://www.gnu.org/licenses/agpl-3.0.html)
