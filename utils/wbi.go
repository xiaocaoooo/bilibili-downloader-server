package utils

import (
	"crypto/md5"
	"encoding/hex"
	"fmt"
	"net/url"
	"sort"
	"strings"
	"time"
)

// mixinKeyEncTab 是 WBI 签名中用于混洗 img_key + sub_key 的固定数组
var mixinKeyEncTab = []int{
	46, 47, 18, 2, 53, 8, 23, 32, 15, 50, 10, 31, 58, 3, 45, 35, 27, 43, 5, 49,
	33, 9, 42, 19, 29, 28, 14, 39, 12, 38, 41, 13, 37, 48, 7, 16, 24, 55, 40, 61,
	26, 17, 0, 1, 60, 51, 30, 4, 22, 25, 54, 21, 56, 59, 6, 63, 57, 62, 11, 36,
	20, 34, 44, 52,
}

// WbiKeys 存储从 nav API 获取的 WBI 密钥
type WbiKeys struct {
	ImgKey string `json:"img_key"`
	SubKey string `json:"sub_key"`
}

// GetMixinKey 对原始 key（img_key + sub_key）进行混洗，返回前 32 个字符
// 参数 orig: img_key 和 sub_key 拼接后的字符串（共 64 字符）
// 返回：混洗后的前 32 个字符作为 mixinKey
func GetMixinKey(orig string) string {
	var sb strings.Builder
	sb.Grow(32)
	for _, i := range mixinKeyEncTab {
		if i < len(orig) {
			sb.WriteByte(orig[i])
		}
	}
	result := sb.String()
	if len(result) > 32 {
		return result[:32]
	}
	return result
}

// EncWbi 生成 WBI 签名
// 参数 params: 原始参数字典
// 参数 imgKey: 从 nav API 获取的 img_key
// 参数 subKey: 从 nav API 获取的 sub_key
// 返回：签名后的参数字典（包含 wts 和 w_rid）
//
// 签名流程：
// 1. 添加 wts（当前时间戳，秒）
// 2. 将所有参数按 key 的 ASCII 码排序
// 3. 过滤掉 value 中的 !、'、(、)、* 字符
// 4. 拼接成 URL 查询字符串
// 5. 在字符串末尾添加 mixinKey
// 6. 对结果进行 MD5 哈希，得到 w_rid
func EncWbi(params map[string]interface{}, imgKey, subKey string) map[string]interface{} {
	// 生成 mixinKey
	mixinKey := GetMixinKey(imgKey + subKey)

	// 添加当前时间戳（秒）
	currTime := time.Now().Unix()
	result := make(map[string]interface{}, len(params)+2)
	for k, v := range params {
		result[k] = v
	}
	result["wts"] = currTime

	// 提取所有 key 并排序
	keys := make([]string, 0, len(result))
	for k := range result {
		keys = append(keys, k)
	}
	sort.Strings(keys)

	// 构建查询字符串，过滤特殊字符
	var queryBuilder strings.Builder
	for i, key := range keys {
		if i > 0 {
			queryBuilder.WriteByte('&')
		}
		value := result[key]
		// 将 value 转换为字符串并过滤特殊字符
		valueStr := fmt.Sprintf("%v", value)
		filteredValue := filterSpecialChars(valueStr)
		// URL 编码
		queryBuilder.WriteString(urlEncode(key))
		queryBuilder.WriteByte('=')
		queryBuilder.WriteString(urlEncode(filteredValue))
	}

	// 在末尾添加 mixinKey 并进行 MD5 哈希
	queryStr := queryBuilder.String() + mixinKey
	md5Hash := md5.Sum([]byte(queryStr))
	wRid := hex.EncodeToString(md5Hash[:])

	// 添加 w_rid 到结果
	result["w_rid"] = wRid

	return result
}

// filterSpecialChars 过滤掉 value 中的 !、'、(、)、* 字符
func filterSpecialChars(s string) string {
	var sb strings.Builder
	sb.Grow(len(s))
	for _, c := range s {
		switch c {
		case '!', '\'', '(', ')', '*':
			// 跳过这些特殊字符
		default:
			sb.WriteRune(c)
		}
	}
	return sb.String()
}

// urlEncode 对字符串进行 URL 编码
func urlEncode(s string) string {
	return url.QueryEscape(s)
}

// ExtractKeyFromURL 从 URL 中提取文件名（去掉 .png 扩展名）
// 例如：https://i0.hdslb.com/bfs/wbi/7cd084941338484aae1ad9425b84077c.png -> 7cd084941338484aae1ad9425b84077c
// 导出函数供 service 包使用
func ExtractKeyFromURL(u string) string {
	// 找到最后一个 / 的位置
	lastSlash := strings.LastIndex(u, "/")
	if lastSlash == -1 {
		return u
	}
	filename := u[lastSlash+1:]
	// 去掉 .png 扩展名
	return strings.TrimSuffix(filename, ".png")
}
