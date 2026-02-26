package main

import (
	"fmt"
	"log"
	"os"
	"os/exec"

	"bilibili-downloader-server/handler"

	"github.com/gin-gonic/gin"
)

const (
	// 默认端口
	defaultPort = "8080"
	// 环境变量名
	envCookie = "BILIBILI_COOKIE"
	envPort   = "PORT"
)

func main() {
	// 1. 读取环境变量
	cookie := os.Getenv(envCookie)
	if cookie == "" {
		log.Fatalf("Error: Environment variable %s must be set\n", envCookie)
	}

	port := os.Getenv(envPort)
	if port == "" {
		port = defaultPort
	}

	// 2. 启动检查
	// 检查 FFmpeg 是否已安装
	if err := checkFFmpeg(); err != nil {
		log.Fatalf("Error: %v\nPlease ensure FFmpeg is installed\n", err)
	}

	log.Println("✓ FFmpeg installed")
	log.Println("✓ Cookie configured")

	// 3. 创建 Handler
	h := handler.NewHandler(cookie)

	// 4. 设置 Gin 模式
	gin.SetMode(gin.ReleaseMode)
	router := gin.Default()

	// 5. 定义路由
	// 健康检查路由
	router.GET("/bilibili/download/health", h.Health)
	// 通用下载路由，支持 AV 号和 BV 号
	router.GET("/bilibili/download/:id", h.Download)

	// 6. 启动服务器
	addr := ":" + port
	log.Printf("🚀 Server starting, listening on: %s\n", addr)
	log.Printf("📥 Download endpoints:\n")
	log.Printf("   - GET http://localhost%s/bilibili/download/:bvid\n", addr)
	log.Printf("   - GET http://localhost%s/bilibili/download/:avid\n", addr)

	if err := router.Run(addr); err != nil {
		log.Fatalf("Failed to start server: %v\n", err)
	}
}

// checkFFmpeg 检查 FFmpeg 是否已安装
func checkFFmpeg() error {
	cmd := exec.Command("ffmpeg", "-version")
	if err := cmd.Run(); err != nil {
		return fmt.Errorf("FFmpeg not installed or unavailable: %w", err)
	}
	return nil
}
