package service

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"sort"
	"strings"
	"sync"
	"time"

	"bilibili-downloader-server/utils"
)

// API 基础 URL 常量
const (
	// BaseURL API 基础域名
	BaseURL = "https://api.bilibili.com"
	// VideoURL 视频页面域名
	VideoURL = "https://www.bilibili.com"
)

// API 端点路径常量
const (
	// PagelistEndpoint 获取视频 CID 的端点
	PagelistEndpoint = "/x/player/pagelist"
	// NavEndpoint 获取 WBI Keys 的端点
	NavEndpoint = "/x/web-interface/nav"
	// PlayUrlEndpoint 获取播放地址的端点
	PlayUrlEndpoint = "/x/player/wbi/playurl"
)

// 默认请求头
const (
	// DefaultUserAgent 默认 User-Agent
	DefaultUserAgent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
	// DefaultReferer 默认 Referer
	DefaultReferer = "https://www.bilibili.com/"
)

// 播放地址请求参数
const (
	// DefaultQn 默认画质质量（1080P）
	DefaultQn = 80
	// DefaultFnver 默认版本
	DefaultFnver = 0
	// DefaultFnval 默认流类型（DASH）
	DefaultFnval = 4048
	// DefaultFourk 默认支持 4K
	DefaultFourk = 1
)

// CidInfo 视频 CID 信息
type CidInfo struct {
	Cid      int64  `json:"cid"`
	Page     int    `json:"page"`
	Part     string `json:"part"`
	Duration int    `json:"duration"`
	Vid      string `json:"vid"`
	Weblink  string `json:"weblink"`
}

// Dimension 视频尺寸信息
type Dimension struct {
	Width  int `json:"width"`
	Height int `json:"height"`
	Rotate int `json:"rotate"`
}

// PagelistResponse 获取 CID 的 API 响应
type PagelistResponse struct {
	Code    int         `json:"code"`
	Message string      `json:"message"`
	Ttl     int         `json:"ttl"`
	Data    []CidInfo   `json:"data"`
}

// PlayUrlData 播放地址数据
type PlayUrlData struct {
	Dash    DashData `json:"dash"`
	Quality int      `json:"quality"`
	Format  string   `json:"format"`
}

// DashData DASH 数据，包含视频和音频轨道
type DashData struct {
	Video []VideoTrack `json:"video"`
	Audio []AudioTrack `json:"audio"`
}

// VideoTrack 视频轨道信息
type VideoTrack struct {
	Id           int            `json:"id"`
	BaseUrl      string         `json:"base_url"`
	BackupUrl    []string       `json:"backup_url"`
	Bandwidth    int            `json:"bandwidth"`
	MimeType     string         `json:"mimeType"`
	Codecs       string         `json:"codecs"`
	Width        int            `json:"width"`
	Height       int            `json:"height"`
	FrameRate    string         `json:"frameRate"`
	Sar          string         `json:"sar"`
	StartWithSap int            `json:"startWithSap"`
	SegmentBase  SegmentBase    `json:"SegmentBase"`
	Codecid      int            `json:"codecid"`
}

// AudioTrack 音频轨道信息
type AudioTrack struct {
	Id           int         `json:"id"`
	BaseUrl      string      `json:"base_url"`
	BackupUrl    []string    `json:"backup_url"`
	Bandwidth    int         `json:"bandwidth"`
	MimeType     string      `json:"mimeType"`
	Codecs       string      `json:"codecs"`
	Width        int         `json:"width"`
	Height       int         `json:"height"`
	FrameRate    string      `json:"frameRate"`
	Sar          string      `json:"sar"`
	StartWithSap int         `json:"startWithSap"`
	SegmentBase  SegmentBase `json:"SegmentBase"`
	Codecid      int         `json:"codecid"`
}

// SegmentBase 分段信息
type SegmentBase struct {
	Initialization string `json:"Initialization"`
	IndexRange   string `json:"indexRange"`
}

// PlayUrlResponse 获取播放地址的 API 响应
type PlayUrlResponse struct {
	Code    int          `json:"code"`
	Message string       `json:"message"`
	Ttl     int          `json:"ttl"`
	Data    PlayUrlData  `json:"data"`
}

// NavResponse 获取用户导航信息的 API 响应
type NavResponse struct {
	Code    int      `json:"code"`
	Message string   `json:"message"`
	Ttl     int      `json:"ttl"`
	Data    NavData  `json:"data"`
}

// NavData 用户导航数据
type NavData struct {
	WbiImg WbiImgData `json:"wbi_img"`
}

// WbiImgData WBI 图片数据
type WbiImgData struct {
	ImgUrl string `json:"img_url"`
	SubUrl string `json:"sub_url"`
}

// WbiKeys WBI 密钥对
type WbiKeys struct {
	ImgKey string `json:"img_key"`
	SubKey string `json:"sub_key"`
}

// ApiService Bilibili API 服务
type ApiService struct {
	cookie     string
	httpClient *http.Client
	wbiKeys    *WbiKeys
	wbiMutex   sync.RWMutex
}

// NewApiService 创建 ApiService 实例
// 参数 cookie: 用户 Cookie，用于身份验证
// 返回：配置好的 ApiService 实例
func NewApiService(cookie string) *ApiService {
	return &ApiService{
		cookie: cookie,
		httpClient: &http.Client{
			Timeout: 30 * time.Second,
		},
		wbiKeys: nil,
	}
}

// GetCid 根据 BV 号获取视频 CID
// 参数 bvid: 视频的 BV 号
// 参数 page: 分 P 页码（从 1 开始）
// 返回：视频 CID 和错误信息
//
// API 端点：GET /x/player/pagelist?bvid={bvid}&page={page}
func (s *ApiService) GetCid(bvid string, page int) (int64, error) {
	apiUrl := fmt.Sprintf("%s%s?bvid=%s&page=%d", BaseURL, PagelistEndpoint, bvid, page)

	req, err := http.NewRequest(http.MethodGet, apiUrl, nil)
	if err != nil {
		return 0, fmt.Errorf("Failed to create request: %w", err)
	}

	// 设置请求头
	s.setHeaders(req, "")

	// 发送请求
	resp, err := s.httpClient.Do(req)
	if err != nil {
		return 0, fmt.Errorf("Request failed: %w", err)
	}
	defer resp.Body.Close()

	// 读取响应体
	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return 0, fmt.Errorf("Failed to read response body: %w", err)
	}

	// 解析 JSON 响应
	var pagelistResp PagelistResponse
	if err := json.Unmarshal(body, &pagelistResp); err != nil {
		return 0, fmt.Errorf("Failed to parse JSON: %w", err)
	}

	// 检查响应码
	if pagelistResp.Code != 0 {
		return 0, fmt.Errorf("API returned error: code=%d, message=%s", pagelistResp.Code, pagelistResp.Message)
	}

	// 检查数据是否为空
	if len(pagelistResp.Data) == 0 {
		return 0, fmt.Errorf("Video page information not found")
	}

	// 返回第一个分 P 的 CID
	return pagelistResp.Data[0].Cid, nil
}

// GetWbiKeys 获取 WBI 签名密钥
// 返回：WbiKeys 结构体和错误信息
//
// API 端点：GET /x/web-interface/nav
// 注意：WBI Keys 会被缓存，避免重复请求
func (s *ApiService) GetWbiKeys() (*WbiKeys, error) {
	// 先尝试读取缓存
	s.wbiMutex.RLock()
	if s.wbiKeys != nil {
		s.wbiMutex.RUnlock()
		return s.wbiKeys, nil
	}
	s.wbiMutex.RUnlock()

	// 缓存未命中，需要获取
	s.wbiMutex.Lock()
	defer s.wbiMutex.Unlock()

	// 双重检查锁
	if s.wbiKeys != nil {
		return s.wbiKeys, nil
	}

	apiUrl := fmt.Sprintf("%s%s", BaseURL, NavEndpoint)

	req, err := http.NewRequest(http.MethodGet, apiUrl, nil)
	if err != nil {
		return nil, fmt.Errorf("Failed to create request: %w", err)
	}

	// 设置请求头
	s.setHeaders(req, "")

	// 发送请求
	resp, err := s.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("Request failed: %w", err)
	}
	defer resp.Body.Close()

	// 读取响应体
	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("Failed to read response body: %w", err)
	}

	// 解析 JSON 响应
	var navResp NavResponse
	if err := json.Unmarshal(body, &navResp); err != nil {
		return nil, fmt.Errorf("Failed to parse JSON: %w", err)
	}

	// 检查响应码
	if navResp.Code != 0 {
		return nil, fmt.Errorf("API returned error: code=%d, message=%s", navResp.Code, navResp.Message)
	}

	// 从 URL 中提取 img_key 和 sub_key
	imgKey := utils.ExtractKeyFromURL(navResp.Data.WbiImg.ImgUrl)
	subKey := utils.ExtractKeyFromURL(navResp.Data.WbiImg.SubUrl)

	if imgKey == "" || subKey == "" {
		return nil, fmt.Errorf("Invalid WBI key format")
	}

	// 缓存密钥
	s.wbiKeys = &WbiKeys{
		ImgKey: imgKey,
		SubKey: subKey,
	}

	return s.wbiKeys, nil
}

// GetPlayUrl 获取视频播放地址
// 参数 bvid: 视频 BV 号
// 参数 cid: 视频 CID
// 参数 quality: 清晰度（qn 值，默认 80）
// 返回：PlayUrlData 结构体和错误信息
//
// API 端点：GET /x/player/wbi/playurl
// 注意：此方法需要 WBI 签名，会自动调用 GetWbiKeys 获取密钥
func (s *ApiService) GetPlayUrl(bvid string, cid int64, quality int) (*PlayUrlData, error) {
	// 获取 WBI Keys
	wbiKeys, err := s.GetWbiKeys()
	if err != nil {
		return nil, fmt.Errorf("Failed to get WBI Keys: %w", err)
	}

	// 构建原始参数
	params := map[string]interface{}{
		"bvid":   bvid,
		"cid":    cid,
		"qn":     quality,
		"fnver":  DefaultFnver,
		"fnval":  DefaultFnval,
		"fourk":  DefaultFourk,
	}

	// 生成签名参数
	signedParams := utils.EncWbi(params, wbiKeys.ImgKey, wbiKeys.SubKey)

	// 构建查询字符串
	query := buildQueryString(signedParams)

	// 构建完整 URL
	apiUrl := fmt.Sprintf("%s%s?%s", BaseURL, PlayUrlEndpoint, query)

	req, err := http.NewRequest(http.MethodGet, apiUrl, nil)
	if err != nil {
		return nil, fmt.Errorf("Failed to create request: %w", err)
	}

	// 设置请求头，Referer 需要包含 BV 号
	referer := fmt.Sprintf("%s/video/%s/", VideoURL, bvid)
	s.setHeaders(req, referer)

	// 发送请求
	resp, err := s.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("Request failed: %w", err)
	}
	defer resp.Body.Close()

	// 读取响应体
	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("Failed to read response body: %w", err)
	}

	// 解析 JSON 响应
	var playUrlResp PlayUrlResponse
	if err := json.Unmarshal(body, &playUrlResp); err != nil {
		return nil, fmt.Errorf("Failed to parse JSON: %w", err)
	}

	// 检查响应码
	if playUrlResp.Code != 0 {
		return nil, fmt.Errorf("API returned error: code=%d, message=%s", playUrlResp.Code, playUrlResp.Message)
	}

	return &playUrlResp.Data, nil
}

// setHeaders 设置 HTTP 请求头
// 参数 req: HTTP 请求
// 参数 referer: Referer 头，如果为空则使用默认值
func (s *ApiService) setHeaders(req *http.Request, referer string) {
	// 设置 User-Agent
	req.Header.Set("User-Agent", DefaultUserAgent)

	// 设置 Referer
	if referer != "" {
		req.Header.Set("Referer", referer)
	} else {
		req.Header.Set("Referer", DefaultReferer)
	}

	// 设置 Cookie
	if s.cookie != "" {
		req.Header.Set("Cookie", s.cookie)
	}
}


// buildQueryString 构建查询字符串
// 参数 params: 参数字典
// 返回：URL 编码后的查询字符串（不包含前导 ?）
func buildQueryString(params map[string]interface{}) string {
	// 提取所有 key 并排序
	keys := make([]string, 0, len(params))
	for k := range params {
		keys = append(keys, k)
	}
	// 使用标准库排序
	sort.Strings(keys)
	// 使用标准库排序
	sort.Strings(keys)

	// 构建查询字符串
	var queryBuilder strings.Builder
	for i, key := range keys {
		if i > 0 {
			queryBuilder.WriteByte('&')
		}
		value := params[key]
		valueStr := fmt.Sprintf("%v", value)
		queryBuilder.WriteString(url.QueryEscape(key))
		queryBuilder.WriteByte('=')
		queryBuilder.WriteString(url.QueryEscape(valueStr))
	}

	return queryBuilder.String()
}

// RefreshWbiKeys 强制刷新 WBI Keys 缓存
// 用于在缓存失效时重新获取密钥
func (s *ApiService) RefreshWbiKeys() (*WbiKeys, error) {
	s.wbiMutex.Lock()
	defer s.wbiMutex.Unlock()

	// 清空缓存
	s.wbiKeys = nil

	// 重新获取
	return s.GetWbiKeys()
}

// GetVideoUrl 获取视频下载地址（优先使用 baseUrl，备用 backupUrl）
// 参数 video: VideoTrack 结构体
// 返回：视频下载地址
func GetVideoUrl(video VideoTrack) string {
	if video.BaseUrl != "" {
		return video.BaseUrl
	}
	if len(video.BackupUrl) > 0 {
		return video.BackupUrl[0]
	}
	return ""
}

// GetAudioUrl 获取音频下载地址（优先使用 baseUrl，备用 backupUrl）
// 参数 audio: AudioTrack 结构体
// 返回：音频下载地址
func GetAudioUrl(audio AudioTrack) string {
	if audio.BaseUrl != "" {
		return audio.BaseUrl
	}
	if len(audio.BackupUrl) > 0 {
		return audio.BackupUrl[0]
	}
	return ""
}

// EnsureContext 确保请求带有 context
// 参数 ctx: 上下文
// 参数 req: HTTP 请求
// 返回：带有上下文的请求
func EnsureContext(ctx context.Context, req *http.Request) *http.Request {
	if ctx == nil {
		ctx = context.Background()
	}
	return req.WithContext(ctx)
}

// SetHeadersForRequest 设置 HTTP 请求头（导出方法供 handler 使用）
// 参数 req: HTTP 请求
// 参数 referer: Referer 头，如果为空则使用默认值
func (s *ApiService) SetHeadersForRequest(req *http.Request, referer string) {
	s.setHeaders(req, referer)
}

// GetHttpClient 获取 HTTP 客户端（导出方法供 handler 使用）
// 返回：http.Client 指针
func (s *ApiService) GetHttpClient() *http.Client {
	return s.httpClient
}

// UnmarshalPagelistResponse 解析 PagelistResponse JSON（导出函数供 handler 使用）
// 参数 data: JSON 数据
// 参数 resp: 响应结构体指针
// 返回：错误信息
func UnmarshalPagelistResponse(data []byte, resp *PagelistResponse) error {
	return json.Unmarshal(data, resp)
}
