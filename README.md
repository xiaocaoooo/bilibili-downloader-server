# Bilibili Downloader Server

A lightweight HTTP service that downloads bilibili videos, merges audio/video with FFmpeg, and streams an MP4 response.

> Migrated from Go to **Rust (Axum)**. Public API is now under `/api/v1` and errors follow **RFC 7807** (`application/problem+json`).

## Features

- AV / BV id download
- Multi-part (`p`) selection
- Quality selection (`quality` / qn, default `80` = 1080P)
- Automatic WBI signing
- Concurrent audio/video download + FFmpeg merge
- Streaming MP4 response (temp files cleaned after transfer)
- OpenAPI + Swagger UI
- Docker / Compose deployment

## Tech Stack

| Component | Detail |
|-----------|--------|
| Language | Rust 2024 edition |
| Web | Axum |
| OpenAPI | utoipa + utoipa-swagger-ui |
| HTTP client | reqwest (rustls) |
| Media | FFmpeg |

## API

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/v1/health` | Health check |
| `GET` | `/api/v1/videos/{id}/download` | Download video as MP4 |
| `GET` | `/swagger-ui/` | Swagger UI |
| `GET` | `/api-docs/openapi.json` | OpenAPI document |
| `GET` | `/api/v1/docs` | Redirect to Swagger UI |

### Download

```http
GET /api/v1/videos/{id}/download?p=1&quality=80
```

| Param | In | Type | Required | Default | Description |
|-------|----|------|----------|---------|-------------|
| `id` | path | string | yes | - | BV id (`BVxxxx`) or AV number |
| `p` | query | int | no | `1` | Part number (1-based) |
| `quality` | query | int | no | `80` | Quality code (qn) |

**Success:** `Content-Type: video/mp4` with `Content-Disposition: attachment; filename="{bvid}.mp4"`.

**Errors:** RFC 7807 problem details, for example:

```json
{
  "type": "about:blank",
  "title": "Not Found",
  "status": 404,
  "detail": "Video not found: ...",
  "instance": "/api/v1/videos/BVxxxx/download",
  "code": "VIDEO_NOT_FOUND"
}
```

### Quality codes

| qn | Quality |
|----|---------|
| 16 | 360P |
| 32 | 480P |
| 64 | 720P |
| 80 | 1080P (default) |
| 112 | 1080P+ |
| 116 | 1080P 60fps |
| 120 | 4K |

Higher qualities usually require a logged-in cookie (and sometimes VIP).

## Quick start

### Requirements

- Docker **or** Rust toolchain + FFmpeg
- bilibili cookie (`BILIBILI_COOKIE`)

### Get a cookie

1. Open [bilibili.com](https://www.bilibili.com) and log in
2. DevTools → Network → any request → copy the `Cookie` request header

### Docker Hub image

```bash
docker run -d \
  --name bilibili-downloader-server \
  -p 8080:8080 \
  -e BILIBILI_COOKIE="your_cookie_here" \
  xiaocaoooo/bilibili-downloader-server:latest
```

### Docker Compose (build from source)

```bash
cp .env.example .env
# edit BILIBILI_COOKIE
docker compose up -d --build
```

### Local run

```bash
# Arch Linux
sudo pacman -S ffmpeg

export BILIBILI_COOKIE="your_cookie_here"
cargo run --release
```

Optional: `PORT=8080` (default `8080`).

## Examples

```bash
# Health
curl http://localhost:8080/api/v1/health

# BV download
curl -O -J "http://localhost:8080/api/v1/videos/BV1xx411c7mD/download"

# AV download
curl -O -J "http://localhost:8080/api/v1/videos/170001/download"

# Part 2 + 720P
curl -O -J "http://localhost:8080/api/v1/videos/BV1xx411c7mD/download?p=2&quality=64"
```

Swagger UI: [http://localhost:8080/swagger-ui/](http://localhost:8080/swagger-ui/)

## Environment variables

| Name | Required | Default | Description |
|------|----------|---------|-------------|
| `BILIBILI_COOKIE` | yes | - | bilibili account cookie |
| `PORT` | no | `8080` | Listen port |
| `RUST_LOG` | no | `info` | tracing filter |

## Project layout

```
src/
  main.rs         # entry / router
  routes.rs       # HTTP handlers
  bilibili.rs     # bilibili API client + WBI keys
  wbi.rs          # WBI signature
  downloader.rs   # download + FFmpeg merge
  bvid.rs         # BV helpers
  error.rs        # RFC 7807 errors
  openapi.rs      # OpenAPI doc
  config.rs       # env config
  state.rs        # shared state
```

## Security notes

- Keep your cookie private
- Avoid high-frequency requests
- The service streams responses and does not permanently store videos

## License

MIT
