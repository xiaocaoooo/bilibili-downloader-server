package service

import (
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
	"time"
)

// Downloader 视频下载器
type Downloader struct {
	httpClient *http.Client
	cookie     string
	referer    string
}

// DownloadResult 下载结果
type DownloadResult struct {
	VideoPath string // 视频文件本地路径
	AudioPath string // 音频文件本地路径
	Err       error  // 错误信息
}

// NewDownloader 创建下载器实例
// 参数 cookie: 用户 Cookie，用于身份验证
// 返回：配置好的 Downloader 实例
func NewDownloader(cookie string) *Downloader {
	return &Downloader{
		httpClient: &http.Client{
			Timeout: 300 * time.Second, // 下载超时时间设置为 5 分钟
		},
		cookie: cookie,
	}
}

// DownloadFile 下载单个文件
// 参数 url: 下载地址
// 参数 referer: Referer 头
// 参数 filename: 保存的文件名
// 返回：错误信息
func (d *Downloader) DownloadFile(url, referer, filename string) error {
	req, err := http.NewRequest(http.MethodGet, url, nil)
	if err != nil {
		return fmt.Errorf("创建请求失败：%w", err)
	}

	// 设置请求头
	d.setDownloadHeaders(req, referer)

	// 发送请求
	resp, err := d.httpClient.Do(req)
	if err != nil {
		return fmt.Errorf("请求失败：%w", err)
	}
	defer resp.Body.Close()

	// 检查响应状态
	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("下载失败，状态码：%d", resp.StatusCode)
	}

	// 创建文件
	file, err := os.Create(filename)
	if err != nil {
		return fmt.Errorf("创建文件失败：%w", err)
	}
	defer file.Close()

	// 写入文件
	_, err = io.Copy(file, resp.Body)
	if err != nil {
		return fmt.Errorf("写入文件失败：%w", err)
	}

	return nil
}

// setDownloadHeaders 设置下载请求头
// 参数 req: HTTP 请求
// 参数 referer: Referer 头
func (d *Downloader) setDownloadHeaders(req *http.Request, referer string) {
	// 设置 User-Agent
	req.Header.Set("User-Agent", DefaultUserAgent)

	// 设置 Referer
	if referer != "" {
		req.Header.Set("Referer", referer)
	} else if d.referer != "" {
		req.Header.Set("Referer", d.referer)
	} else {
		req.Header.Set("Referer", DefaultReferer)
	}

	// 设置 Cookie
	if d.cookie != "" {
		req.Header.Set("Cookie", d.cookie)
	}

	// 设置 Accept-Encoding 为 identity，避免压缩
	req.Header.Set("Accept-Encoding", "identity")
}

// DownloadAndMerge 并发下载音视频并合并
// 参数 videoUrl: 视频下载地址
// 参数 audioUrl: 音频下载地址
// 参数 bvid: 视频 BV 号，用于生成 Referer
// 返回：合并后的视频流和错误信息
func (d *Downloader) DownloadAndMerge(videoUrl, audioUrl, bvid string) (io.ReadCloser, error) {
	// 创建临时目录
	tempDir, err := os.MkdirTemp("", "bilibili_downloader_*")
	if err != nil {
		return nil, fmt.Errorf("创建临时目录失败：%w", err)
	}
	defer func() {
		// 延迟清理临时目录（在函数返回前）
		// 注意：这里不能立即删除，因为需要等合并完成
	}()

	// 生成唯一文件名
	timestamp := time.Now().UnixNano()
	videoPath := filepath.Join(tempDir, fmt.Sprintf("video_%d.mp4", timestamp))
	audioPath := filepath.Join(tempDir, fmt.Sprintf("audio_%d.m4a", timestamp))
	outputPath := filepath.Join(tempDir, fmt.Sprintf("output_%d.mp4", timestamp))

	// 设置 Referer
	referer := fmt.Sprintf("%s/video/%s/", VideoURL, bvid)

	// 使用 channel 接收下载结果
	resultChan := make(chan *DownloadResult, 2)
	var wg sync.WaitGroup

	// 并发下载视频
	wg.Add(1)
	go func() {
		defer wg.Done()
		err := d.DownloadFile(videoUrl, referer, videoPath)
		resultChan <- &DownloadResult{
			VideoPath: videoPath,
			Err:       err,
		}
	}()

	// 并发下载音频
	wg.Add(1)
	go func() {
		defer wg.Done()
		err := d.DownloadFile(audioUrl, referer, audioPath)
		resultChan <- &DownloadResult{
			AudioPath: audioPath,
			Err:       err,
		}
	}()

	// 等待所有下载完成
	go func() {
		wg.Wait()
		close(resultChan)
	}()

	// 收集下载结果
	var videoErr, audioErr error
	for result := range resultChan {
		if result.Err != nil {
			if result.VideoPath != "" {
				videoErr = result.Err
			} else {
				audioErr = result.Err
			}
		}
	}

	// 检查下载错误
	if videoErr != nil {
		// 清理临时文件
		cleanupFiles(tempDir, videoPath, audioPath, outputPath)
		return nil, fmt.Errorf("视频下载失败：%w", videoErr)
	}
	if audioErr != nil {
		// 清理临时文件
		cleanupFiles(tempDir, videoPath, audioPath, outputPath)
		return nil, fmt.Errorf("音频下载失败：%w", audioErr)
	}

	// 使用 FFmpeg 合并
	err = d.mergeWithFfmpeg(videoPath, audioPath, outputPath)
	if err != nil {
		cleanupFiles(tempDir, videoPath, audioPath, outputPath)
		return nil, fmt.Errorf("FFmpeg 合并失败：%w", err)
	}

	// 清理音视频临时文件，保留输出文件
	cleanupFiles("", videoPath, audioPath)

	// 打开合并后的文件
	file, err := os.Open(outputPath)
	if err != nil {
		// 清理临时目录
		os.RemoveAll(tempDir)
		return nil, fmt.Errorf("打开输出文件失败：%w", err)
	}

	// 返回文件读取器，并在关闭时清理临时文件
	return &cleanupReadCloser{
		File:     file,
		tempDir:  tempDir,
		filePath: outputPath,
	}, nil
}

// cleanupReadCloser 包装 os.File，在关闭时清理临时文件
type cleanupReadCloser struct {
	*os.File
	tempDir  string
	filePath string
	closed   bool
	once     sync.Once
}

// Close 关闭文件并清理临时文件
func (c *cleanupReadCloser) Close() error {
	var err error
	c.once.Do(func() {
		c.closed = true
		// 先关闭文件
		err = c.File.Close()
		// 清理临时文件和目录
		os.Remove(c.filePath)
		os.RemoveAll(c.tempDir)
	})
	return err
}

// mergeWithFfmpeg 调用 FFmpeg 合并音视频
// 参数 videoPath: 视频文件路径
// 参数 audioPath: 音频文件路径
// 参数 outputPath: 输出文件路径
// 返回：错误信息
func (d *Downloader) mergeWithFfmpeg(videoPath, audioPath, outputPath string) error {
	// 检查 FFmpeg 是否安装
	ffmpegPath, err := exec.LookPath("ffmpeg")
	if err != nil {
		return fmt.Errorf("未找到 FFmpeg，请确保已安装：%w", err)
	}

	// 构建 FFmpeg 命令
	// ffmpeg -y -i video.mp4 -i audio.m4a -c copy output.mp4
	cmd := exec.Command(ffmpegPath,
		"-y",           // 覆盖输出文件
		"-i", videoPath, // 输入视频
		"-i", audioPath, // 输入音频
		"-c", "copy",   // 直接复制流，不重新编码
		outputPath,     // 输出文件
	)

	// 执行命令
	output, err := cmd.CombinedOutput()
	if err != nil {
		return fmt.Errorf("FFmpeg 执行失败：%w, 输出：%s", err, string(output))
	}

	return nil
}

// cleanupFiles 清理临时文件
// 参数 tempDir: 临时目录路径（如果为空则不删除目录）
// 参数 files: 要删除的文件路径列表
func cleanupFiles(tempDir string, files ...string) {
	for _, file := range files {
		if file != "" {
			os.Remove(file)
		}
	}
	if tempDir != "" {
		os.RemoveAll(tempDir)
	}
}

// CheckFfmpegInstalled 检查系统中是否安装了 FFmpeg
// 返回：是否安装和错误信息
func CheckFfmpegInstalled() (bool, error) {
	_, err := exec.LookPath("ffmpeg")
	if err != nil {
		if strings.Contains(err.Error(), "executable file not found") {
			return false, nil
		}
		return false, err
	}
	return true, nil
}
