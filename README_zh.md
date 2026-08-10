# Bilibili 视频下载服务器

轻量 HTTP 服务：下载 B 站视频，用 FFmpeg 合并音视频，并以 MP4 流式返回。

> 已从 Go 迁移为 **Rust (Axum)**。对外接口位于 `/api/v1`，错误响应遵循 **RFC 7807**（`application/problem+json`）。

## 功能

- 支持 AV / BV 号下载
- 多分 P（`p`）选择
- 清晰度选择（`quality` / qn，默认 `80` = 1080P）
- 自动 WBI 签名
- 音视频并发下载 + FFmpeg 合并
- MP4 流式响应（传输结束后清理临时文件）
- OpenAPI + Swagger UI
- Docker / Compose 部署

## 技术栈

| 组件 | 说明 |
|------|------|
| 语言 | Rust 2024 edition |
| Web | Axum |
| OpenAPI | utoipa + utoipa-swagger-ui |
| HTTP 客户端 | reqwest (rustls) |
| 媒体处理 | FFmpeg |

## API

| 方法 | 路径 | 说明 |
|------|------|------|
| `GET` | `/api/v1/health` | 健康检查 |
| `GET` | `/api/v1/videos/{id}/download` | 下载视频为 MP4 |
| `GET` | `/swagger-ui/` | Swagger UI |
| `GET` | `/api-docs/openapi.json` | OpenAPI 文档 |
| `GET` | `/api/v1/docs` | 跳转到 Swagger UI |

### 下载接口

```http
GET /api/v1/videos/{id}/download?p=1&quality=80
```

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|------|--------|------|
| `id` | path | string | 是 | - | BV 号（`BVxxxx`）或 AV 号 |
| `p` | query | int | 否 | `1` | 分 P（从 1 开始） |
| `quality` | query | int | 否 | `80` | 清晰度代码（qn） |

**成功：** `Content-Type: video/mp4`，`Content-Disposition: attachment; filename="{bvid}.mp4"`。

**失败：** RFC 7807 Problem Details，例如：

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

### 清晰度代码

| qn | 清晰度 |
|----|--------|
| 16 | 360P |
| 32 | 480P |
| 64 | 720P |
| 80 | 1080P（默认） |
| 112 | 1080P+ |
| 116 | 1080P 60帧 |
| 120 | 4K |

更高清晰度通常需要登录 Cookie（部分需要大会员）。

## 快速开始

### 前置要求

- Docker **或** Rust 工具链 + FFmpeg
- B 站 Cookie（`BILIBILI_COOKIE`）

### 获取 Cookie

1. 打开 [bilibili.com](https://www.bilibili.com) 并登录
2. 开发者工具 → Network → 任意请求 → 复制请求头中的 `Cookie`

### Docker Hub 镜像

```bash
docker run -d \
  --name bilibili-downloader-server \
  -p 8080:8080 \
  -e BILIBILI_COOKIE="your_cookie_here" \
  xiaocaoooo/bilibili-downloader-server:latest
```

### Docker Compose（源码构建）

```bash
cp .env.example .env
# 编辑 BILIBILI_COOKIE
docker compose up -d --build
```

### 本地运行

```bash
# Arch Linux
sudo pacman -S ffmpeg

export BILIBILI_COOKIE="your_cookie_here"
cargo run --release
```

可选：`PORT=8080`（默认 `8080`）。

## 使用示例

```bash
# 健康检查
curl http://localhost:8080/api/v1/health

# BV 下载
curl -O -J "http://localhost:8080/api/v1/videos/BV1xx411c7mD/download"

# AV 下载
curl -O -J "http://localhost:8080/api/v1/videos/170001/download"

# 第 2P + 720P
curl -O -J "http://localhost:8080/api/v1/videos/BV1xx411c7mD/download?p=2&quality=64"
```

Swagger UI：[http://localhost:8080/swagger-ui/](http://localhost:8080/swagger-ui/)

## 环境变量

| 名称 | 必填 | 默认值 | 说明 |
|------|------|--------|------|
| `BILIBILI_COOKIE` | 是 | - | B 站账号 Cookie |
| `PORT` | 否 | `8080` | 监听端口 |
| `RUST_LOG` | 否 | `info` | tracing 日志过滤 |

## 项目结构

```
src/
  main.rs         # 入口 / 路由
  routes.rs       # HTTP 处理
  bilibili.rs     # B 站 API 客户端与 WBI key
  wbi.rs          # WBI 签名
  downloader.rs   # 下载与 FFmpeg 合并
  bvid.rs         # BV 辅助
  error.rs        # RFC 7807 错误
  openapi.rs      # OpenAPI 文档
  config.rs       # 环境配置
  state.rs        # 共享状态
```

## 安全说明

- 妥善保管 Cookie，不要泄露
- 避免高频请求
- 服务以流式传输返回，不会永久保存视频

## 许可证

MIT
