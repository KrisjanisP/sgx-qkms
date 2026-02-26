use percent_encoding::percent_decode_str;
use std::collections::HashMap;
use std::error::Error;
use std::io::Read;

pub enum HttpMethod {
    Get,
    Post,
    Other,
}

pub struct ParsedRequest {
    pub method: HttpMethod,
    pub path: String,
    pub query_params: HashMap<String, String>,
    pub body: Option<String>,
}

pub struct HttpResponse {
    pub status: u16,
    pub content_type: &'static str,
    pub body: String,
}

impl HttpResponse {
    pub fn to_http_bytes(&self) -> Vec<u8> {
        let status_text = match self.status {
            200 => "OK",
            400 => "Bad Request",
            401 => "Unauthorized",
            404 => "Not Found",
            405 => "Method Not Allowed",
            503 => "Service Unavailable",
            _ => "Internal Server Error",
        };

        let response = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            self.status,
            status_text,
            self.content_type,
            self.body.len(),
            self.body
        );
        response.into_bytes()
    }
}

pub fn read_http_request(stream: &mut impl Read) -> Result<Vec<u8>, Box<dyn Error>> {
    const MAX_HEADER_BYTES: usize = 64 * 1024;
    let mut buf = Vec::with_capacity(2048);
    let mut chunk = [0_u8; 1024];

    loop {
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);

        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
        if buf.len() > MAX_HEADER_BYTES {
            return Err("request headers too large".into());
        }
    }

    Ok(buf)
}

pub fn parse_http_request(raw: &[u8]) -> Result<ParsedRequest, Box<dyn Error>> {
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut req = httparse::Request::new(&mut headers);

    let header_len = match req.parse(raw)? {
        httparse::Status::Complete(len) => len,
        httparse::Status::Partial => return Err("incomplete HTTP request".into()),
    };

    let method = match req.method.ok_or("missing HTTP method")? {
        "GET" => HttpMethod::Get,
        "POST" => HttpMethod::Post,
        _ => HttpMethod::Other,
    };

    let raw_path = req.path.ok_or("missing request path")?;

    let (path, query_params) = match raw_path.split_once('?') {
        Some((p, qs)) => (decode_percent(p), parse_query_string(qs)),
        None => (decode_percent(raw_path), HashMap::new()),
    };

    let body = if raw.len() > header_len {
        Some(String::from_utf8_lossy(&raw[header_len..]).into_owned())
    } else {
        None
    };

    Ok(ParsedRequest {
        method,
        path,
        query_params,
        body,
    })
}

fn decode_percent(s: &str) -> String {
    percent_decode_str(s).decode_utf8_lossy().into_owned()
}

fn parse_query_string(qs: &str) -> HashMap<String, String> {
    qs.split('&')
        .filter(|pair| !pair.is_empty())
        .filter_map(|pair| {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            Some((decode_percent(k), decode_percent(v)))
        })
        .collect()
}
