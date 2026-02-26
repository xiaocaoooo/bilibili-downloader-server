package handler

import (
	"fmt"
	"io"
	"net/http"
	"strings"

	"bilibili-downloader-server/service"

	"github.com/gin-gonic/gin"
)

// Handler HTTP 请求处理器
type Handler struct {
	apiService *service.ApiService
	downloader *service.Downloader
}

// NewHandler 创建 Handler 实例
// 参数 cookie: 用户 Cookie，用于身份验证
// 返回：配置好的 Handler 实例
func NewHandler(cookie string) *Handler {
	return &Handler{
		apiService: service.NewApiService(cookie),
		downloader: service.NewDownloader(cookie),
	}
}

// Download 处理通用下载请求
// GET /bilibili/download/:id
// 从 URL 参数获取 id，自动判断是 AV 号还是 BV 号，下载视频并返回
func (h *Handler) Download(c *gin.Context) {
	id := c.Param("id")
	if id == "" {
		c.JSON(http.StatusBadRequest, gin.H{
			"error": "缺少视频 ID 参数",
		})
		return
	}

	// 获取 URL 参数 p（分 P 页码）和 quality（清晰度）
	p := c.DefaultQuery("p", "1")
	quality := c.DefaultQuery("quality", "80")

	// 解析 page 参数
	page := 1
	if _, err := fmt.Sscanf(p, "%d", &page); err != nil || page < 1 {
		c.JSON(http.StatusBadRequest, gin.H{
			"error": "无效的分 P 参数",
		})
		return
	}

	// 解析 quality 参数
	qn := 80
	if _, err := fmt.Sscanf(quality, "%d", &qn); err != nil || qn < 1 {
		c.JSON(http.StatusBadRequest, gin.H{
			"error": "无效的清晰度参数",
		})
		return
	}

	// 判断是 AV 号还是 BV 号
	// AV 号：纯数字
	// BV 号：以 BV 开头（不区分大小写）
	var bvid string
	var err error

	if strings.HasPrefix(strings.ToUpper(id), "BV") {
		// BV 号下载
		bvid = id
		// 确保 bvid 以 BV 开头
		if !strings.HasPrefix(bvid, "BV") {
			bvid = "BV" + strings.TrimPrefix(id, "BV")
		}
	} else if isNumeric(id) {
		// AV 号下载，转换为 BV 号
		bvid, err = h.avidToBvid(id, page)
		if err != nil {
			c.JSON(http.StatusInternalServerError, gin.H{
				"error": "AV 号转换失败：" + err.Error(),
			})
			return
		}
	} else {
		c.JSON(http.StatusBadRequest, gin.H{
			"error": "无效的视频 ID 格式",
		})
		return
	}

	// 下载视频
	reader, err := h.downloadVideo(bvid, page, qn)
	if err != nil {
		h.handleError(c, err)
		return
	}
	defer reader.Close()

	// 设置响应头
	c.Header("Content-Type", "video/mp4")
	c.Header("Content-Disposition", fmt.Sprintf("attachment; filename=\"%s.mp4\"", bvid))

	// 将文件内容写入响应体
	_, err = io.Copy(c.Writer, reader)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{
			"error": "写入响应失败：" + err.Error(),
		})
		return
	}
}

// isNumeric 判断字符串是否为纯数字
func isNumeric(s string) bool {
	for _, r := range s {
		if r < '0' || r > '9' {
			return false
		}
	}
	return len(s) > 0
}

// downloadVideo 执行视频下载流程
// 参数 bvid: 视频 BV 号
// 参数 page: 分 P 页码
// 参数 quality: 清晰度
// 返回：视频文件读取器和错误信息
func (h *Handler) downloadVideo(bvid string, page int, quality int) (io.ReadCloser, error) {
	// 1. 获取 CID
	cid, err := h.apiService.GetCid(bvid, page)
	if err != nil {
		return nil, fmt.Errorf("获取 CID 失败：%w", err)
	}

	// 2. 获取播放地址
	playUrlData, err := h.apiService.GetPlayUrl(bvid, cid, quality)
	if err != nil {
		return nil, fmt.Errorf("获取播放地址失败：%w", err)
	}

	// 3. 提取视频和音频地址
	if len(playUrlData.Dash.Video) == 0 || len(playUrlData.Dash.Audio) == 0 {
		return nil, fmt.Errorf("未找到视频或音频流")
	}

	videoUrl := service.GetVideoUrl(playUrlData.Dash.Video[0])
	audioUrl := service.GetAudioUrl(playUrlData.Dash.Audio[0])

	if videoUrl == "" || audioUrl == "" {
		return nil, fmt.Errorf("视频或音频地址为空")
	}

	// 4. 下载并合并
	reader, err := h.downloader.DownloadAndMerge(videoUrl, audioUrl, bvid)
	if err != nil {
		return nil, fmt.Errorf("下载合并失败：%w", err)
	}

	return reader, nil
}

// handleError 处理错误并返回适当的 HTTP 状态码
func (h *Handler) handleError(c *gin.Context, err error) {
	errStr := err.Error()

	// 检查是否是视频不存在的错误
	if strings.Contains(errStr, "未找到视频") || strings.Contains(errStr, "10002") {
		c.JSON(http.StatusNotFound, gin.H{
			"error": "视频不存在：" + err.Error(),
		})
		return
	}

	// 检查是否是 Cookie 无效的错误
	if strings.Contains(errStr, "权限") || strings.Contains(errStr, "-403") || strings.Contains(errStr, "Cookie") {
		c.JSON(http.StatusForbidden, gin.H{
			"error": "Cookie 无效或权限不足：" + err.Error(),
		})
		return
	}

	// 其他错误返回 500
	c.JSON(http.StatusInternalServerError, gin.H{
		"error": "服务器错误：" + err.Error(),
	})
}

// avidToBvid 将 AV 号转换为 BV 号
// 通过调用 Bilibili pagelist API 获取视频信息，从响应中提取 BV 号
// 参数 avid: AV 号
// 参数 page: 分 P 页码（用于获取对应分 P 的 BV 号）
func (h *Handler) avidToBvid(avid string, page int) (string, error) {
	// 由于 AV 号转 BV 号需要特定的算法，这里通过 API 获取视频信息
	// 调用 pagelist API 通过 aid 获取视频信息，响应中包含 bvid
	apiUrl := fmt.Sprintf("%s%s?aid=%s&page=%d", service.BaseURL, service.PagelistEndpoint, avid, page)

	req, err := http.NewRequest(http.MethodGet, apiUrl, nil)
	if err != nil {
		return "", fmt.Errorf("创建请求失败：%w", err)
	}

	// 设置请求头
	h.apiService.SetHeadersForRequest(req, "")

	// 发送请求
	resp, err := h.apiService.GetHttpClient().Do(req)
	if err != nil {
		return "", fmt.Errorf("请求失败：%w", err)
	}
	defer resp.Body.Close()

	// 读取响应体
	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return "", fmt.Errorf("读取响应体失败：%w", err)
	}

	// 解析 JSON 响应
	var pagelistResp service.PagelistResponse
	if err := service.UnmarshalPagelistResponse(body, &pagelistResp); err != nil {
		return "", fmt.Errorf("解析 JSON 失败：%w", err)
	}

	// 检查响应码
	if pagelistResp.Code != 0 {
		return "", fmt.Errorf("API 返回错误：code=%d, message=%s", pagelistResp.Code, pagelistResp.Message)
	}

	// 检查数据是否为空
	if len(pagelistResp.Data) == 0 {
		return "", fmt.Errorf("未找到视频信息")
	}

	// 返回 BV 号（从 Vid 字段获取）
	bvid := pagelistResp.Data[0].Vid
	if bvid == "" {
		// 如果 Vid 为空，尝试从第一个分 P 信息中获取
		// 注意：当前 API 响应中 Vid 字段可能为空，需要后续补充
		return "", fmt.Errorf("未找到 BV 号信息")
	}
	return bvid, nil
}
