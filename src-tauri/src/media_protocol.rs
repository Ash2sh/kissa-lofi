use crate::models::AppConfig;
use crate::storage::LocalStorage;
use percent_encoding::percent_decode_str;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use tauri::http::{header, Request, Response, StatusCode};

const MAX_CHUNK_SIZE: u64 = 1024 * 1024; // 1 MiB streaming chunk for ultra-low RAM footprint

pub fn handle_media_request(
    request: Request<Vec<u8>>,
    storage: &LocalStorage,
    config: &AppConfig,
) -> Response<Vec<u8>> {
    let uri = request.uri();
    let query = uri.query().unwrap_or("");
    let raw_path = query
        .split('&')
        .find_map(|pair| pair.strip_prefix("path="))
        .unwrap_or("");

    let decoded = match percent_decode_str(raw_path).decode_utf8() {
        Ok(s) => s.into_owned(),
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "Invalid media path encoding"),
    };

    if decoded.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "Missing media path");
    }

    let path = PathBuf::from(&decoded);
    if !storage.is_path_allowed(&path, config) {
        return error_response(StatusCode::FORBIDDEN, "Path is outside configured local media libraries");
    }

    let mut file = match File::open(&path) {
        Ok(f) => f,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Local media file not found"),
    };

    let file_len = match file.metadata() {
        Ok(m) => m.len(),
        Err(_) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Unable to inspect file"),
    };

    let mime = mime_guess::from_path(&path)
        .first_or_octet_stream()
        .essence_str()
        .to_string();

    if let Some(range_val) = request.headers().get(header::RANGE).and_then(|v| v.to_str().ok()) {
        if let Some((start, end)) = parse_byte_range(range_val, file_len) {
            let capped_end = end.min(start.saturating_add(MAX_CHUNK_SIZE - 1)).min(file_len.saturating_sub(1));
            let length = capped_end.saturating_sub(start) + 1;
            let mut buffer = vec![0u8; length as usize];
            if file.seek(SeekFrom::Start(start)).is_err() || file.read_exact(&mut buffer).is_err() {
                return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed reading file range");
            }
            return Response::builder()
                .status(StatusCode::PARTIAL_CONTENT)
                .header(header::CONTENT_TYPE, mime)
                .header(header::ACCEPT_RANGES, "bytes")
                .header(header::CONTENT_LENGTH, length.to_string())
                .header(
                    header::CONTENT_RANGE,
                    format!("bytes {start}-{capped_end}/{file_len}"),
                )
                .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                .body(buffer)
                .unwrap_or_else(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR, "Response build error"));
        }
    }

    // Non-range request (typically images or small audio files)
    let mut buffer = Vec::with_capacity(file_len.min(16 * 1024 * 1024) as usize);
    if file.read_to_end(&mut buffer).is_err() {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed reading media file");
    }

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_LENGTH, buffer.len().to_string())
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .body(buffer)
        .unwrap_or_else(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR, "Response build error"))
}

fn parse_byte_range(header_val: &str, total_len: u64) -> Option<(u64, u64)> {
    if total_len == 0 {
        return None;
    }
    let spec = header_val.strip_prefix("bytes=")?;
    let first = spec.split(',').next()?.trim();
    let (start_s, end_s) = first.split_once('-')?;

    if start_s.is_empty() {
        let suffix: u64 = end_s.parse().ok()?;
        let start = total_len.saturating_sub(suffix);
        return Some((start, total_len - 1));
    }

    let start: u64 = start_s.parse().ok()?;
    if start >= total_len {
        return None;
    }
    let end = if end_s.is_empty() {
        total_len - 1
    } else {
        end_s.parse::<u64>().ok()?.min(total_len - 1)
    };
    if end < start {
        return None;
    }
    Some((start, end))
}

fn error_response(status: StatusCode, msg: &str) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(msg.as_bytes().to_vec())
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::parse_byte_range;

    #[test]
    fn parses_standard_and_suffix_byte_ranges() {
        assert_eq!(parse_byte_range("bytes=0-1023", 4096), Some((0, 1023)));
        assert_eq!(parse_byte_range("bytes=500-", 4096), Some((500, 4095)));
        assert_eq!(parse_byte_range("bytes=-256", 4096), Some((3840, 4095)));
        assert_eq!(parse_byte_range("bytes=9999-", 4096), None);
        assert_eq!(parse_byte_range("bytes=200-100", 4096), None);
    }
}
