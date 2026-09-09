//! 下载响应中的文件名解析。

use reqwest::Response;

/// 简单百分号解码（`%XX`），非 UTF-8 字节以 `U+FFFD` 替换。
pub(crate) fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                out.push(hi << 4 | lo);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// 清理文件名，防止目录穿越：只保留最后一段、拒绝 `..` 与空名、要求含扩展名。
fn sanitize_filename(filename: &str) -> Option<String> {
    let name = filename.rsplit(['/', '\\']).next().unwrap_or(filename);
    if name.is_empty() || name == "." || name == ".." || name.starts_with("..") {
        return None;
    }
    if !name.contains('.') {
        return None;
    }
    Some(name.to_string())
}

/// 解析 `Content-Disposition` 中的文件名（`filename*=` RFC 5987 优先，`filename=` 兜底）。
fn parse_content_disposition(header: &str) -> Option<String> {
    let header_lower = header.to_lowercase();

    // 优先 `filename*=`（RFC 5987 编码）
    if let Some(start) = header_lower.find("filename*=") {
        let rest = &header[start + 10..];
        if let Some(quote_pos) = rest.find("''") {
            let encoded = rest[quote_pos + 2..].split(';').next().unwrap_or("").trim();
            let decoded = percent_decode(encoded.trim_matches('"'));
            if !decoded.is_empty() {
                return Some(decoded);
            }
        }
    }

    // 兜底 `filename=`（排除 `filename*=`）
    let mut search_start = 0;
    while let Some(pos) = header_lower[search_start..].find("filename=") {
        let absolute = search_start + pos;
        if absolute > 0 && header.as_bytes().get(absolute - 1) == Some(&b'*') {
            search_start = absolute + 9;
            continue;
        }
        let rest = &header[absolute + 9..];
        let name = rest
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .trim_matches('"')
            .to_string();
        if !name.is_empty() {
            return Some(name);
        }
        break;
    }
    None
}

/// 从响应中提取文件名：优先 `Content-Disposition`，其次 302 重定向后的最终 URL 路径。
pub(crate) fn extract_filename_from_response(response: &Response) -> Option<String> {
    if let Some(cd) = response.headers().get("content-disposition") {
        if let Ok(cd_str) = cd.to_str() {
            if let Some(name) = parse_content_disposition(cd_str) {
                if let Some(safe) = sanitize_filename(&name) {
                    return Some(safe);
                }
            }
        }
    }

    let path = response.url().path();
    if let Some(last_segment) = path.rsplit('/').next() {
        if let Some(safe) = sanitize_filename(&percent_decode(last_segment)) {
            return Some(safe);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(
            sanitize_filename("OEA-windows-x86_64-v0.1.0.zip"),
            Some("OEA-windows-x86_64-v0.1.0.zip".to_string())
        );
        assert_eq!(sanitize_filename(".."), None);
        assert_eq!(sanitize_filename("a/b/c.zip"), Some("c.zip".to_string()));
        assert_eq!(sanitize_filename("noext"), None);
        assert_eq!(sanitize_filename(""), None);
    }

    #[test]
    fn test_parse_content_disposition() {
        assert_eq!(
            parse_content_disposition("attachment; filename=\"a.zip\""),
            Some("a.zip".to_string())
        );
        assert_eq!(
            parse_content_disposition("attachment; filename*=UTF-8''a%20b.zip"),
            Some("a b.zip".to_string())
        );
        assert_eq!(
            parse_content_disposition("attachment; filename=a.zip; size=1"),
            Some("a.zip".to_string())
        );
        assert_eq!(parse_content_disposition("attachment"), None);
    }

    #[test]
    fn test_percent_decode() {
        assert_eq!(percent_decode("a%20b%2Fc"), "a b/c");
        assert_eq!(percent_decode("plain"), "plain");
    }
}
