//! AV/BV conversion (bilibili public algorithm).

const XOR_CODE: i64 = 23_442_827_791_579;
const MAX_CODE: i64 = 2_251_799_813_685_247;
const CHARTS: &[u8] = b"FcwAPNKTMug3GV5Lj7EJnHpWsx4tb8haYeviqBz6rkCy12mUSDQX9RdoZf";
const BASE: i64 = 58;

fn swap_bytes(mut s: Vec<u8>, x: usize, y: usize) -> Vec<u8> {
    s.swap(x, y);
    s
}

#[allow(dead_code)]
fn char_index(c: u8) -> Option<i64> {
    CHARTS.iter().position(|&x| x == c).map(|i| i as i64)
}

/// Convert AV number to BV id.
pub fn avid_to_bvid(avid: u64) -> String {
    let avid = avid as i64;
    let mut arr = vec![b'B', b'V', b'1', 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let mut bv_idx = arr.len() - 1;
    let mut temp = (avid | (MAX_CODE + 1)) ^ XOR_CODE;

    while temp > 0 {
        let idx = (temp % BASE) as usize;
        arr[bv_idx] = CHARTS[idx];
        temp /= BASE;
        if bv_idx == 0 {
            break;
        }
        bv_idx -= 1;
    }

    let raw = swap_bytes(swap_bytes(arr, 3, 9), 4, 7);
    String::from_utf8(raw).expect("BV id is valid UTF-8")
}

/// Convert BV id to AV number.
#[allow(dead_code)] // used by unit tests; kept as public conversion helper
pub fn bvid_to_avid(bvid: &str) -> Option<u64> {
    if bvid.len() != 12 {
        return None;
    }
    let bytes = bvid.as_bytes().to_vec();
    let s = swap_bytes(swap_bytes(bytes, 3, 9), 4, 7);
    let mut temp: i64 = 0;
    for &c in s.iter().skip(3) {
        let idx = char_index(c)?;
        temp = temp * BASE + idx;
    }
    let avid = (temp & MAX_CODE) ^ XOR_CODE;
    if avid < 0 { None } else { Some(avid as u64) }
}

/// Normalize a BV-like id to `BV` + original body (case-preserving body).
pub fn normalize_bvid(id: &str) -> Option<String> {
    if id.len() < 3 {
        return None;
    }
    if id[..2].eq_ignore_ascii_case("BV") {
        let mut out = String::with_capacity(id.len());
        out.push_str("BV");
        out.push_str(&id[2..]);
        return Some(out);
    }
    None
}

pub fn is_numeric_aid(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn av_bv_roundtrip_classic() {
        // Classic pair used across bilibili docs / demos.
        assert_eq!(avid_to_bvid(2), "BV1xx411c7mD");
        assert_eq!(bvid_to_avid("BV1xx411c7mD"), Some(2));
    }

    #[test]
    fn av_bv_additional_pairs() {
        assert_eq!(avid_to_bvid(7), "BV1xx411c7m9");
        assert_eq!(bvid_to_avid("BV1xx411c7m9"), Some(7));

        assert_eq!(avid_to_bvid(170001), "BV17x411w7KC");
        assert_eq!(bvid_to_avid("BV17x411w7KC"), Some(170001));

        assert_eq!(avid_to_bvid(10000), "BV1bx411c7ux");
        assert_eq!(bvid_to_avid("BV1bx411c7ux"), Some(10000));
    }

    #[test]
    fn normalize_bvid_case() {
        assert_eq!(
            normalize_bvid("bv1xx411c7mD").as_deref(),
            Some("BV1xx411c7mD")
        );
    }

    #[test]
    fn numeric_aid() {
        assert!(is_numeric_aid("170001"));
        assert!(!is_numeric_aid("BV1xx"));
        assert!(!is_numeric_aid(""));
    }
}
