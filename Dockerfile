# 多阶段构建 - 构建阶段
FROM golang:1.21-alpine AS builder

# 安装必要的构建工具
RUN apk add --no-cache git

# 设置工作目录
WORKDIR /build

# 复制 go.mod 和 go.sum
COPY go.mod go.sum ./

# 下载依赖
RUN go mod download

# 复制源代码
COPY . .

# 编译应用（静态链接，适用于 Alpine）
RUN CGO_ENABLED=0 GOOS=linux go build -a -installsuffix cgo -o bilibili-downloader-server .

# 运行阶段
FROM alpine:3.19

# 安装运行时依赖（FFmpeg）
RUN apk --no-cache add ffmpeg ca-certificates

# 创建非 root 用户
RUN addgroup -g 1000 appgroup && \
    adduser -u 1000 -G appgroup -D appuser

# 设置工作目录
WORKDIR /app

# 从构建阶段复制二进制文件
COPY --from=builder /build/bilibili-downloader-server .

# 更改文件所有者
RUN chown -R appuser:appgroup /app

# 切换到非 root 用户
USER appuser

# 暴露默认端口
EXPOSE 8080

# 健康检查
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8080/bilibili/download/health || exit 1

# 启动应用
ENTRYPOINT ["./bilibili-downloader-server"]
