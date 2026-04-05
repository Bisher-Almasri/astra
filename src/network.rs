use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;

#[derive(Debug)]
pub enum NetworkError {
    InvalidUrl(String),
    ConnectionFailed(String),
    TlsError(String),
    HttpError(String),
    Timeout,
    IoError(String),
}

impl std::fmt::Display for NetworkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkError::InvalidUrl(s) => write!(f, "Invalid URL: {}", s),
            NetworkError::ConnectionFailed(s) => write!(f, "Connection failed: {}", s),
            NetworkError::TlsError(s) => write!(f, "TLS error: {}", s),
            NetworkError::HttpError(s) => write!(f, "HTTP error: {}", s),
            NetworkError::Timeout => write!(f, "Connection timed out"),
            NetworkError::IoError(s) => write!(f, "I/O error: {}", s),
        }
    }
}

#[derive(Debug)]
pub struct Response {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

struct ParsedUrl {
    scheme: String,
    host: String,
    port: u16,
    path: String,
}

fn parse_url(url: &str) -> Result<ParsedUrl, NetworkError> {
    let (scheme, rest) = if let Some(r) = url.strip_prefix("https://") {
        ("https".to_string(), r)
    } else if let Some(r) = url.strip_prefix("http://") {
        ("http".to_string(), r)
    } else {
        return Err(NetworkError::InvalidUrl(format!(
            "Unsupported scheme in URL: {}",
            url
        )));
    };

    let (authority, path) = match rest.find('/') {
        Some(idx) => (&rest[..idx], rest[idx..].to_string()),
        None => (rest, "/".to_string()),
    };

    let path = if path.is_empty() { "/".to_string() } else { path };

    let default_port: u16 = if scheme == "https" { 443 } else { 80 };
    let (host, port) = if let Some(colon) = authority.rfind(':') {
        let port_str = &authority[colon + 1..];
        match port_str.parse::<u16>() {
            Ok(p) => (authority[..colon].to_string(), p),
            Err(_) => {
                return Err(NetworkError::InvalidUrl(format!(
                    "Invalid port in URL: {}",
                    url
                )))
            }
        }
    } else {
        (authority.to_string(), default_port)
    };

    if host.is_empty() {
        return Err(NetworkError::InvalidUrl(format!(
            "Missing host in URL: {}",
            url
        )));
    }

    Ok(ParsedUrl { scheme, host, port, path })
}

fn build_request(host: &str, path: &str) -> String {
    format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nUser-Agent: Astra/0.1\r\n\r\n",
        path, host
    )
}

fn parse_response(raw: &[u8]) -> Result<Response, NetworkError> {
    let separator = b"\r\n\r\n";
    let sep_pos = raw
        .windows(4)
        .position(|w| w == separator)
        .ok_or_else(|| NetworkError::HttpError("Missing header/body separator".to_string()))?;

    let header_section = std::str::from_utf8(&raw[..sep_pos])
        .map_err(|e| NetworkError::HttpError(format!("Invalid UTF-8 in headers: {}", e)))?;

    let body = raw[sep_pos + 4..].to_vec();

    let mut lines = header_section.lines();

    let status_line = lines
        .next()
        .ok_or_else(|| NetworkError::HttpError("Empty response".to_string()))?;

    let status = parse_status_line(status_line)?;

    let mut headers = HashMap::new();
    for line in lines {
        if let Some(colon) = line.find(':') {
            let key = line[..colon].trim().to_lowercase();
            let value = line[colon + 1..].trim().to_string();
            headers.insert(key, value);
        }
    }

    Ok(Response { status, headers, body })
}

fn parse_status_line(line: &str) -> Result<u16, NetworkError> {
    let mut parts = line.splitn(3, ' ');
    let version = parts.next().unwrap_or("");
    if !version.starts_with("HTTP/") {
        return Err(NetworkError::HttpError(format!(
            "Invalid HTTP version: {}",
            version
        )));
    }
    let code_str = parts
        .next()
        .ok_or_else(|| NetworkError::HttpError("Missing status code".to_string()))?;
    code_str
        .parse::<u16>()
        .map_err(|_| NetworkError::HttpError(format!("Invalid status code: {}", code_str)))
}

pub struct NetworkStack;

impl NetworkStack {
    pub fn fetch(url: &str) -> Result<Response, NetworkError> {
        let parsed = parse_url(url)?;
        match parsed.scheme.as_str() {
            "http" => Self::fetch_http(&parsed.host, parsed.port, &parsed.path),
            "https" => Self::fetch_https(&parsed.host, parsed.port, &parsed.path),
            _ => Err(NetworkError::InvalidUrl(format!(
                "Unsupported scheme: {}",
                parsed.scheme
            ))),
        }
    }

    pub fn fetch_http(host: &str, port: u16, path: &str) -> Result<Response, NetworkError> {
        let addr = format!("{}:{}", host, port);
        let mut stream = TcpStream::connect(&addr)
            .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;

        let request = build_request(host, path);
        stream
            .write_all(request.as_bytes())
            .map_err(|e| NetworkError::IoError(e.to_string()))?;

        let mut raw = Vec::new();
        stream
            .read_to_end(&mut raw)
            .map_err(|e| NetworkError::IoError(e.to_string()))?;

        parse_response(&raw)
    }

    pub fn fetch_https(host: &str, port: u16, path: &str) -> Result<Response, NetworkError> {
        let connector = native_tls::TlsConnector::new()
            .map_err(|e| NetworkError::TlsError(e.to_string()))?;

        let addr = format!("{}:{}", host, port);
        let tcp = TcpStream::connect(&addr)
            .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;

        let mut stream = connector
            .connect(host, tcp)
            .map_err(|e| NetworkError::TlsError(e.to_string()))?;

        let request = build_request(host, path);
        stream
            .write_all(request.as_bytes())
            .map_err(|e| NetworkError::IoError(e.to_string()))?;

        let mut raw = Vec::new();
        stream
            .read_to_end(&mut raw)
            .map_err(|e| NetworkError::IoError(e.to_string()))?;

        parse_response(&raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_http_url_default_port() {
        let p = parse_url("http://example.com/index.html").unwrap();
        assert_eq!(p.scheme, "http");
        assert_eq!(p.host, "example.com");
        assert_eq!(p.port, 80);
        assert_eq!(p.path, "/index.html");
    }

    #[test]
    fn test_parse_https_url_default_port() {
        let p = parse_url("https://example.com/").unwrap();
        assert_eq!(p.scheme, "https");
        assert_eq!(p.host, "example.com");
        assert_eq!(p.port, 443);
        assert_eq!(p.path, "/");
    }

    #[test]
    fn test_parse_url_custom_port() {
        let p = parse_url("http://localhost:8080/api").unwrap();
        assert_eq!(p.host, "localhost");
        assert_eq!(p.port, 8080);
        assert_eq!(p.path, "/api");
    }

    #[test]
    fn test_parse_url_no_path_defaults_to_slash() {
        let p = parse_url("http://example.com").unwrap();
        assert_eq!(p.path, "/");
    }

    #[test]
    fn test_parse_url_invalid_scheme() {
        assert!(matches!(
            parse_url("ftp://example.com"),
            Err(NetworkError::InvalidUrl(_))
        ));
    }

    #[test]
    fn test_parse_url_missing_host() {
        assert!(matches!(
            parse_url("http:///path"),
            Err(NetworkError::InvalidUrl(_))
        ));
    }

    #[test]
    fn test_parse_url_invalid_port() {
        assert!(matches!(
            parse_url("http://example.com:abc/"),
            Err(NetworkError::InvalidUrl(_))
        ));
    }

    #[test]
    fn test_build_request_format() {
        let req = build_request("example.com", "/index.html");
        assert!(req.starts_with("GET /index.html HTTP/1.1\r\n"));
        assert!(req.contains("Host: example.com\r\n"));
        assert!(req.contains("Connection: close\r\n"));
        assert!(req.contains("User-Agent: Astra/0.1\r\n"));
        assert!(req.ends_with("\r\n\r\n"));
    }

    #[test]
    fn test_parse_response_200() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 5\r\n\r\nhello";
        let resp = parse_response(raw).unwrap();
        assert_eq!(resp.status, 200);
        assert_eq!(resp.headers.get("content-type").unwrap(), "text/html");
        assert_eq!(resp.headers.get("content-length").unwrap(), "5");
        assert_eq!(resp.body, b"hello");
    }

    #[test]
    fn test_parse_response_404() {
        let raw = b"HTTP/1.1 404 Not Found\r\n\r\n";
        let resp = parse_response(raw).unwrap();
        assert_eq!(resp.status, 404);
        assert!(resp.body.is_empty());
    }

    #[test]
    fn test_parse_response_empty_body() {
        let raw = b"HTTP/1.1 204 No Content\r\nX-Custom: value\r\n\r\n";
        let resp = parse_response(raw).unwrap();
        assert_eq!(resp.status, 204);
        assert_eq!(resp.headers.get("x-custom").unwrap(), "value");
        assert!(resp.body.is_empty());
    }

    #[test]
    fn test_parse_response_binary_body() {
        let mut raw = b"HTTP/1.1 200 OK\r\n\r\n".to_vec();
        raw.extend_from_slice(&[0u8, 1, 2, 3, 255]);
        let resp = parse_response(&raw).unwrap();
        assert_eq!(resp.body, &[0u8, 1, 2, 3, 255]);
    }

    #[test]
    fn test_parse_response_missing_separator() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n";
        assert!(matches!(
            parse_response(raw),
            Err(NetworkError::HttpError(_))
        ));
    }

    #[test]
    fn test_parse_response_invalid_status_line() {
        let raw = b"BOGUS 200 OK\r\n\r\n";
        assert!(matches!(
            parse_response(raw),
            Err(NetworkError::HttpError(_))
        ));
    }

    #[test]
    fn test_parse_response_header_keys_lowercased() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\n";
        let resp = parse_response(raw).unwrap();
        assert!(resp.headers.contains_key("content-type"));
        assert!(!resp.headers.contains_key("Content-Type"));
    }
}
