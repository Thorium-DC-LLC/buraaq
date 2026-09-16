use std::fs;
use std::io::{self, Read, Write};
use std::net::TcpStream;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    Msg(String),
}

/// Push a `.bur` to a dock: `PUT /v1/apps/{name}`.
pub fn push(host: &str, token: &str, name: &str, bur: &[u8]) -> Result<String, ClientError> {
    let addr = normalize_host(host);
    let mut stream = TcpStream::connect(&addr).map_err(|e| {
        ClientError::Msg(format!(
            "cannot reach dock at {addr}: {e}\nstart one with: buraaq dock"
        ))
    })?;
    let path = format!("/v1/apps/{name}");
    let host_header = addr.split(':').next().unwrap_or("127.0.0.1");
    let req = format!(
        "PUT {path} HTTP/1.1\r\nHost: {host_header}\r\nAuthorization: Bearer {token}\r\nContent-Type: application/buraaq-ship\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        bur.len()
    );
    stream.write_all(req.as_bytes())?;
    stream.write_all(bur)?;
    stream.flush()?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;
    let text = String::from_utf8_lossy(&buf).into_owned();
    if let Some(rest) = text.split("\r\n\r\n").nth(1) {
        if text.contains("200 OK") || text.contains("201") {
            return Ok(rest.trim().to_string());
        }
        return Err(ClientError::Msg(rest.trim().to_string()));
    }
    Err(ClientError::Msg(text))
}

pub fn get(host: &str, token: &str, path: &str) -> Result<String, ClientError> {
    let addr = normalize_host(host);
    let mut stream = TcpStream::connect(&addr)?;
    let host_header = addr.split(':').next().unwrap_or("127.0.0.1");
    let req = format!(
        "GET {path} HTTP/1.1\r\nHost: {host_header}\r\nAuthorization: Bearer {token}\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(req.as_bytes())?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;
    let text = String::from_utf8_lossy(&buf).into_owned();
    Ok(text.split("\r\n\r\n").nth(1).unwrap_or(&text).trim().to_string())
}

pub fn delete(host: &str, token: &str, name: &str) -> Result<String, ClientError> {
    let addr = normalize_host(host);
    let mut stream = TcpStream::connect(&addr)?;
    let path = format!("/v1/apps/{name}");
    let host_header = addr.split(':').next().unwrap_or("127.0.0.1");
    let req = format!(
        "DELETE {path} HTTP/1.1\r\nHost: {host_header}\r\nAuthorization: Bearer {token}\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(req.as_bytes())?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

pub fn normalize_host(host: &str) -> String {
    if host.contains(']') {
        return host.to_string();
    }
    if host.contains(':') {
        host.to_string()
    } else {
        format!("{host}:{}", crate::DOCK_PORT)
    }
}

/// `user@203.0.113.10` → `203.0.113.10`. SSH specs belong to `buraaq land`.
pub fn dock_addr(spec: &str) -> String {
    let host = spec.rsplit_once('@').map(|(_, h)| h).unwrap_or(spec);
    normalize_host(host)
}

pub fn health(host: &str) -> Result<String, ClientError> {
    let addr = dock_addr(host);
    let mut stream = TcpStream::connect(&addr).map_err(|e| {
        ClientError::Msg(format!("cannot reach dock at {addr}: {e}"))
    })?;
    let host_header = addr.split(':').next().unwrap_or("127.0.0.1");
    let req = format!("GET /v1/health HTTP/1.1\r\nHost: {host_header}\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes())?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;
    let text = String::from_utf8_lossy(&buf).into_owned();
    Ok(text
        .split("\r\n\r\n")
        .nth(1)
        .unwrap_or(&text)
        .trim()
        .to_string())
}

pub fn resolve_token(explicit: Option<&str>) -> Result<String, ClientError> {
    if let Some(t) = explicit {
        if !t.is_empty() {
            return Ok(t.to_string());
        }
    }
    if let Ok(t) = std::env::var("BURAAQ_DOCK_TOKEN") {
        if !t.is_empty() {
            return Ok(t);
        }
    }
    let p = crate::paths::token_path();
    if p.is_file() {
        return Ok(fs::read_to_string(p)?.trim().to_string());
    }
    Err(ClientError::Msg(
        "no dock token (set BURAAQ_DOCK_TOKEN or run `buraaq dock` once)".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_spec_strips_user() {
        assert_eq!(dock_addr("root@203.0.113.10"), "203.0.113.10:7422");
        assert_eq!(dock_addr("127.0.0.1"), "127.0.0.1:7422");
    }
}
