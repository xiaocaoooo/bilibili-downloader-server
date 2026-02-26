# 构建阶段 (Go)
FROM golang:1.21-alpine AS builder
RUN apk add --no-cache git
WORKDIR /build
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN CGO_ENABLED=0 GOOS=linux go build -ldflags="-s -w" -o server .

# 运行阶段
FROM alpine:3.19

# 设置时区和证书
RUN apk --no-cache add ca-certificates tzdata

# 核心优化：引入静态 FFmpeg
COPY --from=mwader/static-ffmpeg:6.1.1 /ffmpeg /usr/local/bin/

# 权限处理
RUN addgroup -g 1000 appgroup && \
    adduser -u 1000 -G appgroup -D appuser
WORKDIR /app
COPY --from=builder /build/server ./bilibili-downloader-server
RUN chown -R appuser:appgroup /app

USER appuser
EXPOSE 8080

# 健康检查保持不变
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8080/bilibili/download/health || exit 1

ENTRYPOINT ["./bilibili-downloader-server"]
