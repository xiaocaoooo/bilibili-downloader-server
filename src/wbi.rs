use std::collections::BTreeMap;

use md5::{Digest, Md5};
use url::form_urlencoded;

/// Fixed shuffle table used by bilibili WBI signing.
const MIXIN_KEY_ENC_TAB: [usize; 64] = [
    46, 47, 18, 2, 53, 8, 23, 32, 15, 50, 10, 31, 58, 3, 45, 35, 27, 43, 5, 49, 33, 9, 42, 19, 29,
    28, 14, 39, 12, 38, 41, 13, 37, 48, 7, 16, 24, 55, 40, 61, 26, 17, 0, 1, 60, 51, 30, 4, 22, 25,
    54, 21, 56, 59, 6, 63, 57, 62, 11, 36, 20, 34, 44, 52,
];

#[derive(Debug, Clone)]
pub struct WbiKeys {
    pub img_key: String,
    pub sub_key: String,
}

/// Shuffle `img_key + sub_key` and take the first 32 characters.
pub fn get_mixin_key(orig: &str) -> String {
    let bytes = orig.as_bytes();
    let mut out = String::with_capacity(32);
    for &index in &MIXIN_KEY_ENC_TAB {
        if index < bytes.len() {
            out.push(bytes[index] as char);
        }
        if out.len() >= 32 {
            break;
        }
    }
    out.truncate(32);
    out
}

fn filter_special_chars(value: &str) -> String {
    value
        .chars()
        .filter(|c| !matches!(c, '!' | '\'' | '(' | ')' | '*'))
        .collect()
}

/// Encode params with WBI (`wts` + `w_rid`).
pub fn enc_wbi(
    params: BTreeMap<String, String>,
    img_key: &str,
    sub_key: &str,
    wts: i64,
) -> BTreeMap<String, String> {
    let mixin_key = get_mixin_key(&format!("{img_key}{sub_key}"));

    let mut result = params;
    result.insert("wts".to_string(), wts.to_string());

    let mut pairs: Vec<(String, String)> = result
        .iter()
        .map(|(k, v)| (k.clone(), filter_special_chars(v)))
        .collect();
    pairs.sort_by(|a, b| a.0.cmp(&b.0));

    let query: String = pairs
        .iter()
        .map(|(k, v)| {
            format!(
                "{}={}",
                form_urlencoded::byte_serialize(k.as_bytes()).collect::<String>(),
                form_urlencoded::byte_serialize(v.as_bytes()).collect::<String>()
            )
        })
        .collect::<Vec<_>>()
        .join("&");

    let mut hasher = Md5::new();
    hasher.update(format!("{query}{mixin_key}").as_bytes());
    let w_rid = format!("{:x}", hasher.finalize());

    // Keep filtered values in the final map for request building consistency.
    let mut signed = BTreeMap::new();
    for (k, v) in pairs {
        signed.insert(k, v);
    }
    signed.insert("w_rid".to_string(), w_rid);
    signed
}

/// Extract filename stem from a WBI image URL.
pub fn extract_key_from_url(url: &str) -> String {
    let filename = url.rsplit('/').next().unwrap_or(url);
    filename
        .strip_suffix(".png")
        .unwrap_or(filename)
        .to_string()
}

/// Build a sorted query string from signed params.
pub fn build_query_string(params: &BTreeMap<String, String>) -> String {
    params
        .iter()
        .map(|(k, v)| {
            format!(
                "{}={}",
                form_urlencoded::byte_serialize(k.as_bytes()).collect::<String>(),
                form_urlencoded::byte_serialize(v.as_bytes()).collect::<String>()
            )
        })
        .collect::<Vec<_>>()
        .join("&")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixin_key_is_32_chars() {
        let img = "7cd084941338484aae1ad9425b84077c";
        let sub = "4932aff0a65dab3b0e98f1c6e2c7c8d0";
        let key = get_mixin_key(&format!("{img}{sub}"));
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn extract_key_strips_png() {
        let url = "https://i0.hdslb.com/bfs/wbi/7cd084941338484aae1ad9425b84077c.png";
        assert_eq!(
            extract_key_from_url(url),
            "7cd084941338484aae1ad9425b84077c"
        );
    }

    #[test]
    fn enc_wbi_adds_wts_and_w_rid() {
        let mut params = BTreeMap::new();
        params.insert("bvid".to_string(), "BV1xx411c7mD".to_string());
        params.insert("cid".to_string(), "1".to_string());

        let signed = enc_wbi(
            params,
            "7cd084941338484aae1ad9425b84077c",
            "4932aff0a65dab3b0e98f1c6e2c7c8d0",
            1_700_000_000,
        );

        assert_eq!(signed.get("wts").map(String::as_str), Some("1700000000"));
        assert_eq!(signed.get("w_rid").map(|s| s.len()), Some(32));
        assert!(signed.contains_key("bvid"));
    }

    #[test]
    fn filter_removes_special_chars_via_enc() {
        let mut params = BTreeMap::new();
        params.insert("foo".to_string(), "a!b'c(d)e*f".to_string());
        let signed = enc_wbi(
            params,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            1,
        );
        assert_eq!(signed.get("foo").map(String::as_str), Some("abcdef"));
    }
}
