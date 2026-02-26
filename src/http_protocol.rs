use percent_encoding::percent_decode_str;
use std::collections::HashMap;
use std::error::Error;
use std::io::Read;

pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Other,
}

pub struct ParsedRequest {
    pub method: HttpMethod,
    pub path: String,
    pub query_params: HashMap<String, String>,
    pub body: Option<String>,
    pub raw_body: Option<Vec<u8>>,
}

pub struct HttpResponse {
    pub status: u16,
    pub content_type: &'static str,
    pub body: String,
}

pub struct BinaryHttpResponse {
    pub status: u16,
    pub content_type: &'static str,
    pub body: Vec<u8>,
}

pub fn read_http_request(stream: &mut impl Read) -> Result<Vec<u8>, Box<dyn Error>> {
    read_http_message(stream, 64 * 1024, 1024 * 1024, "request")
}

pub fn read_http_response(stream: &mut impl Read) -> Result<Vec<u8>, Box<dyn Error>> {
    read_http_message(stream, 64 * 1024, 16 * 1024 * 1024, "response")
}

fn read_http_message(
    stream: &mut impl Read,
    max_header_bytes: usize,
    max_body_bytes: usize,
    kind: &str,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut buf = Vec::with_capacity(2048);
    let mut chunk = [0_u8; 1024];

    let header_end = loop {
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            break buf.len();
        }
        buf.extend_from_slice(&chunk[..n]);

        if let Some(pos) = find_header_end(&buf) {
            break pos;
        }
        if buf.len() > max_header_bytes {
            return Err(format!("{kind} headers too large").into());
        }
    };

    let body_start = header_end + 4;
    if let Some(content_length) = extract_content_length(&buf[..header_end]) {
        if content_length > max_body_bytes {
            return Err(format!("{kind} body too large").into());
        }
        let total_needed = body_start + content_length;
        while buf.len() < total_needed {
            let n = stream.read(&mut chunk)?;
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);
        }
    } else {
        let _ = stream.read_to_end(&mut buf);
    }

    Ok(buf)
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

fn extract_content_length(header_bytes: &[u8]) -> Option<usize> {
    let header_str = std::str::from_utf8(header_bytes).ok()?;
    for line in header_str.lines() {
        if let Some(val) = line
            .strip_prefix("Content-Length:")
            .or_else(|| line.strip_prefix("content-length:"))
        {
            return val.trim().parse().ok();
        }
    }
    None
}

pub fn parse_http_request(raw: &[u8]) -> Result<ParsedRequest, Box<dyn Error>> {
    let request = parse_http_request_message(raw)?;

    let method = match *request.method() {
        http::Method::GET => HttpMethod::Get,
        http::Method::POST => HttpMethod::Post,
        http::Method::PUT => HttpMethod::Put,
        http::Method::DELETE => HttpMethod::Delete,
        _ => HttpMethod::Other,
    };

    let uri = request.uri().to_string();
    let (path, query_params) = match uri.split_once('?') {
        Some((p, qs)) => (decode_percent(p), parse_query_string(qs)),
        None => (decode_percent(&uri), HashMap::new()),
    };

    let body_bytes = request.body().clone();
    let (body, raw_body) = if body_bytes.is_empty() {
        (None, None)
    } else {
        (
            Some(String::from_utf8_lossy(&body_bytes).into_owned()),
            Some(body_bytes),
        )
    };

    Ok(ParsedRequest {
        method,
        path,
        query_params,
        body,
        raw_body,
    })
}

pub fn parse_http_request_message(raw: &[u8]) -> Result<http::Request<Vec<u8>>, Box<dyn Error>> {
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut req = httparse::Request::new(&mut headers);

    let header_len = match req.parse(raw)? {
        httparse::Status::Complete(len) => len,
        httparse::Status::Partial => return Err("incomplete HTTP request".into()),
    };

    let method = req.method.ok_or("missing HTTP method")?;
    let raw_path = req.path.ok_or("missing request path")?;
    let version = match req.version.unwrap_or(1) {
        0 => http::Version::HTTP_10,
        _ => http::Version::HTTP_11,
    };

    let mut builder = http::Request::builder()
        .method(method)
        .uri(raw_path)
        .version(version);

    for header in req.headers.iter() {
        if header.name.is_empty() {
            continue;
        }
        builder = builder.header(header.name, http::HeaderValue::from_bytes(header.value)?);
    }

    let body = if raw.len() > header_len {
        raw[header_len..].to_vec()
    } else {
        Vec::new()
    };

    Ok(builder.body(body)?)
}

pub fn parse_http_response_message(raw: &[u8]) -> Result<http::Response<Vec<u8>>, Box<dyn Error>> {
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut resp = httparse::Response::new(&mut headers);

    let header_len = match resp.parse(raw)? {
        httparse::Status::Complete(len) => len,
        httparse::Status::Partial => return Err("incomplete HTTP response".into()),
    };

    let status = resp.code.ok_or("missing HTTP status code")?;
    let version = match resp.version.unwrap_or(1) {
        0 => http::Version::HTTP_10,
        _ => http::Version::HTTP_11,
    };

    let mut builder = http::Response::builder().status(status).version(version);
    for header in resp.headers.iter() {
        if header.name.is_empty() {
            continue;
        }
        builder = builder.header(header.name, http::HeaderValue::from_bytes(header.value)?);
    }

    let body = if raw.len() > header_len {
        raw[header_len..].to_vec()
    } else {
        Vec::new()
    };

    Ok(builder.body(body)?)
}

pub fn serialize_http_request(request: &http::Request<Vec<u8>>) -> Result<Vec<u8>, Box<dyn Error>> {
    use std::io::Write;

    let mut out = Vec::new();
    let target = request
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");
    write!(&mut out, "{} {} HTTP/1.1\r\n", request.method(), target)?;

    let mut has_host = false;
    let mut has_connection = false;
    let mut has_content_length = false;

    for (name, value) in request.headers() {
        if name == http::header::HOST {
            has_host = true;
        }
        if name == http::header::CONNECTION {
            has_connection = true;
        }
        if name == http::header::CONTENT_LENGTH {
            has_content_length = true;
        }
        out.extend_from_slice(name.as_str().as_bytes());
        out.extend_from_slice(b": ");
        out.extend_from_slice(value.as_bytes());
        out.extend_from_slice(b"\r\n");
    }

    if !has_host {
        if let Some(authority) = request.uri().authority() {
            out.extend_from_slice(b"Host: ");
            out.extend_from_slice(authority.as_str().as_bytes());
            out.extend_from_slice(b"\r\n");
        }
    }
    if !has_connection {
        out.extend_from_slice(b"Connection: close\r\n");
    }
    if !has_content_length {
        write!(&mut out, "Content-Length: {}\r\n", request.body().len())?;
    }

    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(request.body());
    Ok(out)
}

pub fn serialize_http_response(response: &http::Response<Vec<u8>>) -> Result<Vec<u8>, Box<dyn Error>> {
    use std::io::Write;

    let mut out = Vec::new();
    let reason = response.status().canonical_reason().unwrap_or("");
    write!(
        &mut out,
        "HTTP/1.1 {} {}\r\n",
        response.status().as_u16(),
        reason
    )?;

    let mut has_connection = false;
    let mut has_content_length = false;
    for (name, value) in response.headers() {
        if name == http::header::CONNECTION {
            has_connection = true;
        }
        if name == http::header::CONTENT_LENGTH {
            has_content_length = true;
        }
        out.extend_from_slice(name.as_str().as_bytes());
        out.extend_from_slice(b": ");
        out.extend_from_slice(value.as_bytes());
        out.extend_from_slice(b"\r\n");
    }

    if !has_connection {
        out.extend_from_slice(b"Connection: close\r\n");
    }
    if !has_content_length {
        write!(&mut out, "Content-Length: {}\r\n", response.body().len())?;
    }

    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(response.body());
    Ok(out)
}

impl HttpResponse {
    pub fn to_http_response(&self) -> Result<http::Response<Vec<u8>>, Box<dyn Error>> {
        Ok(http::Response::builder()
            .status(self.status)
            .header(http::header::CONTENT_TYPE, self.content_type)
            .body(self.body.as_bytes().to_vec())?)
    }
}

impl BinaryHttpResponse {
    pub fn to_http_response(&self) -> Result<http::Response<Vec<u8>>, Box<dyn Error>> {
        Ok(http::Response::builder()
            .status(self.status)
            .header(http::header::CONTENT_TYPE, self.content_type)
            .body(self.body.clone())?)
    }

    pub fn json(status: u16, body: String) -> Self {
        Self {
            status,
            content_type: "application/json",
            body: body.into_bytes(),
        }
    }

    pub fn octet_stream(body: Vec<u8>) -> Self {
        Self {
            status: 200,
            content_type: "application/octet-stream",
            body,
        }
    }
}

fn decode_percent(s: &str) -> String {
    percent_decode_str(s).decode_utf8_lossy().into_owned()
}

fn parse_query_string(qs: &str) -> HashMap<String, String> {
    qs.split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            (decode_percent(k), decode_percent(v))
        })
        .collect()
}
