# Bilibili 视频下载服务器

一个基于 Go 语言开发的 Bilibili 视频下载服务器，支持通过 HTTP API 下载 Bilibili 视频，自动合并音视频并返回 MP4 文件。

## 项目简介

本项目是一个轻量级的 Bilibili 视频下载服务，通过简单的 HTTP 接口即可下载 Bilibili 视频。支持 AV 号和 BV 号两种视频标识方式，自动处理 WBI 签名认证，使用 FFmpeg 合并音视频流，最终返回完整的 MP4 视频文件。

## 主要功能

- ✅ **支持 AV/BV 号下载** - 自动识别并处理 AV 号和 BV 号
- ✅ **分 P 支持** - 支持下载多 P 视频的指定分 P
- ✅ **清晰度选择** - 支持选择不同清晰度（默认 1080P）
- ✅ **自动合并** - 自动下载并合并音视频流
- ✅ **WBI 签名** - 自动处理 Bilibili API 的 WBI 签名认证
- ✅ **Docker 部署** - 提供 Dockerfile 和 docker-compose.yml，支持一键部署
- ✅ **流式响应** - 下载完成后以流式方式返回视频，节省服务器存储

## 技术栈

| 组件 | 版本/说明 |
|------|-----------|
| **语言** | Go 1.21 |
| **Web 框架** | [Gin](https://github.com/gin-gonic/gin) v1.9.1 |
| **视频处理** | FFmpeg（系统依赖） |
| **认证** | Bilibili WBI 签名算法 |

### 项目结构

```
bilibili-downloader-server/
├── main.go              # 主程序入口
├── handler/
│   └── handler.go       # HTTP 请求处理器
├── service/
│   ├── api.go           # Bilibili API 服务
│   └── downloader.go    # 视频下载器服务
├── utils/
│   └── wbi.go           # WBI 签名工具
├── Dockerfile           # Docker 构建配置
├── docker-compose.yml   # Docker Compose 配置
├── go.mod               # Go 模块依赖
└── go.sum               # Go 依赖校验
```

## 快速开始

### 前置要求

1. **Docker**（使用 Docker 部署时）
2. **Go 1.21+**（本地运行时）
3. **FFmpeg**（本地运行时必须安装，用于合并音视频）
4. **Bilibili Cookie**（用于 API 认证）

### 获取 Bilibili Cookie

1. 打开浏览器访问 [Bilibili](https://www.bilibili.com)
2. 登录你的账号
3. 打开开发者工具（F12），切换到 Network 标签
4. 刷新页面，找到任意请求
5. 复制请求头中的 `Cookie` 值

### 方式一：使用 Docker Hub 镜像（推荐）

无需 clone 代码仓库，直接使用 Docker Hub 镜像运行。

#### 使用 Docker 命令直接运行

```bash
docker run -d \
  --name bilibili-downloader \
  -p 8080:8080 \
  -e BILIBILI_COOKIE="your_cookie_here" \
  xiaocaoooo/bilibili-downloader:latest
```

#### 使用 Docker Compose

创建 `docker-compose.yml` 文件：

```yaml
# docker-compose.yml (使用 Docker Hub 镜像)
version: '3'
services:
  bilibili-downloader:
    image: xiaocaoooo/bilibili-downloader:latest
    container_name: bilibili-downloader
    ports:
      - "8080:8080"
    environment:
      - BILIBILI_COOKIE=your_cookie_here
    restart: unless-stopped
```

启动服务：

```bash
docker-compose up -d
```

验证服务：

```bash
curl http://localhost:8080/bilibili/download/BV1xx411c7mD
```

### 方式二：从源码构建

#### 使用 Docker 构建并运行

1. 克隆项目：
   ```bash
   git clone <repository-url>
   cd bilibili-downloader-server
   ```

2. 构建镜像：
   ```bash
   docker build -t bilibili-downloader-server .
   ```

3. 运行容器：
   ```bash
   docker run -d \
     -p 8080:8080 \
     -e BILIBILI_COOKIE="your_cookie_here" \
     --name bilibili-downloader-server \
     bilibili-downloader-server
   ```

#### 使用 Docker Compose（从源码构建）

1. 克隆项目：
   ```bash
   git clone <repository-url>
   cd bilibili-downloader-server
   ```

2. 创建 `.env` 文件并配置 Cookie：
   ```bash
   # Bilibili Cookie（必填）
   BILIBILI_COOKIE=your_cookie_here
   
   # 服务端口（可选，默认 8080）
   PORT=8080
   ```

3. 启动服务：
   ```bash
   docker-compose up -d
   ```

4. 验证服务：
   ```bash
   curl http://localhost:8080/bilibili/download/BV1xx411c7mD
   ```

### 方式三：本地运行

1. 克隆项目：
   ```bash
   git clone <repository-url>
   cd bilibili-downloader-server
   ```

2. 安装依赖：
   ```bash
   go mod download
   ```

3. 安装 FFmpeg：

   **Ubuntu/Debian:**
   ```bash
   sudo apt update && sudo apt install ffmpeg
   ```

   **macOS:**
   ```bash
   brew install ffmpeg
   ```

   **Windows:**
   从 [FFmpeg 官网](https://ffmpeg.org/download.html) 下载并安装

4. 设置环境变量并运行：
   ```bash
   # Linux/macOS
   export BILIBILI_COOKIE="your_cookie_here"
   go run main.go

   # Windows (PowerShell)
   $env:BILIBILI_COOKIE="your_cookie_here"
   go run main.go
   ```

5. 或者使用 `go run` 直接指定环境变量：
   ```bash
   BILIBILI_COOKIE="your_cookie_here" go run main.go
   ```

## API 接口说明

### 下载视频

**端点:** `GET /bilibili/download/:id`

**参数说明:**

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|------|--------|------|
| `id` | URL 路径 | string | 是 | - | 视频 ID（AV 号或 BV 号） |
| `p` | Query | int | 否 | 1 | 分 P 页码（从 1 开始） |
| `quality` | Query | int | 否 | 80 | 清晰度代码 |

**清晰度代码对照表:**

| 代码 | 清晰度 |
|------|--------|
| 127 | 8K 超高清 |
| 120 | 4K 超清 |
| 116 | 1080P 高帧率 |
| 112 | 1080P+ |
| 80 | 1080P |
| 74 | 720P 高帧率 |
| 64 | 720P |
| 48 | 720P |
| 32 | 480P |
| 16 | 360P |

> ⚠️ **注意:** 高清晰度（112 及以上）需要大会员账号

**请求示例:**

```bash
# 下载 BV 号视频（默认 1080P）
curl -O -J http://localhost:8080/bilibili/download/BV1xx411c7mD

# 下载 AV 号视频
curl -O -J http://localhost:8080/bilibili/download/170001

# 下载指定分 P
curl -O -J "http://localhost:8080/bilibili/download/BV1xx411c7mD?p=2"

# 下载指定清晰度（1080P 高帧率）
curl -O -J "http://localhost:8080/bilibili/download/BV1xx411c7mD?quality=116"

# 组合参数：下载第 2P 的 720P 版本
curl -O -J "http://localhost:8080/bilibili/download/BV1xx411c7mD?p=2&quality=64"
```

**响应:**

- **成功:** 返回 MP4 视频文件流
  - `Content-Type: video/mp4`
  - `Content-Disposition: attachment; filename="{bvid}.mp4"`

- **失败:** 返回 JSON 错误信息

  ```json
  {
    "error": "错误描述信息"
  }
  ```

**HTTP 状态码:**

| 状态码 | 说明 |
|--------|------|
| 200 | 下载成功 |
| 400 | 请求参数错误（无效的视频 ID、分 P 或清晰度） |
| 403 | Cookie 无效或权限不足 |
| 404 | 视频不存在 |
| 500 | 服务器内部错误 |

## 配置说明

### 环境变量

| 变量名 | 必填 | 默认值 | 说明 |
|--------|------|--------|------|
| `BILIBILI_COOKIE` | 是 | - | Bilibili 账号 Cookie，用于 API 认证 |
| `PORT` | 否 | 8080 | 服务器监听端口 |

### Docker 配置

在 [`docker-compose.yml`](docker-compose.yml) 中：

```yaml
services:
  bilibili-downloader-server:
    build:
      context: .
      dockerfile: Dockerfile
    image: xiaocaoooo/bilibili-downloader-server:latest
    container_name: bilibili-downloader-server
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - BILIBILI_COOKIE=${BILIBILI_COOKIE:?BILIBILI_COOKIE 环境变量必须设置}
      - PORT=8080
    volumes:
      - ./downloads:/app/downloads  # 可选：挂载下载目录
    healthcheck:
      test: ["CMD", "wget", "--no-verbose", "--tries=1", "--spider", "http://localhost:8080/bilibili/download/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 10s
```

## 使用示例

### 使用 cURL 下载

```bash
# 基础下载
curl -O -J http://localhost:8080/bilibili/download/BV1xx411c7mD

# 下载并指定文件名
curl -o my-video.mp4 http://localhost:8080/bilibili/download/BV1xx411c7mD

# 下载指定分 P
curl -O -J "http://localhost:8080/bilibili/download/BV1xx411c7mD?p=3"
```

### 使用 wget 下载

```bash
# 基础下载
wget http://localhost:8080/bilibili/download/BV1xx411c7mD

# 指定输出文件名
wget -O my-video.mp4 http://localhost:8080/bilibili/download/BV1xx411c7mD
```

### 在浏览器中下载

直接在浏览器地址栏输入：
```
http://localhost:8080/bilibili/download/BV1xx411c7mD
```

浏览器会自动开始下载视频文件。

### 使用 Python 脚本下载

```python
import requests

url = "http://localhost:8080/bilibili/download/BV1xx411c7mD"
params = {"p": 1, "quality": 80}

response = requests.get(url, params=params, stream=True)

with open("video.mp4", "wb") as f:
    for chunk in response.iter_content(chunk_size=8192):
        f.write(chunk)

print("下载完成!")
```

## 常见问题

### 1. "Cookie 无效或权限不足"

- 确保 Cookie 已正确设置
- Cookie 可能已过期，请重新获取
- 某些视频需要大会员权限才能下载高清晰度

### 2. "FFmpeg 未安装"

- 请确保系统中已安装 FFmpeg
- 使用 `ffmpeg -version` 验证安装
- Docker 镜像已预装 FFmpeg，无需额外配置

### 3. "视频不存在"

- 检查视频 ID 是否正确
- 确认视频未被删除或设为私有
- 尝试使用 BV 号而非 AV 号

### 4. 下载速度慢

- 服务器与 Bilibili 服务器之间的网络状况
- 尝试降低清晰度要求
- 确保 Cookie 有效（登录状态）

## 注意事项

1. **版权说明** - 请遵守 Bilibili 的用户协议和版权规定，仅下载允许下载的内容
2. **Cookie 安全** - 妥善保管你的 Cookie，不要泄露给他人
3. **使用限制** - 避免频繁请求，防止被 Bilibili 封禁
4. **存储说明** - 本服务采用流式传输，不会在服务器保存视频文件

## 许可证

MIT License

## 贡献

欢迎提交 Issue 和 Pull Request！
