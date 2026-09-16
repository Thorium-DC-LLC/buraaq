use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

#[cfg(windows)]
#[link(name = "advapi32")]
extern "system" {
    fn SystemFunction036(buf: *mut u8, len: u32) -> u8;
}

use crate::bundle::{ident_ok, unpack};
use crate::launch::{list_live, replace_live, stop_named};
use crate::paths::{dock_root, token_path};

pub struct DockOptions {
    pub bind: String,
    pub token: String,
}

pub fn serve_dock(opts: DockOptions) -> Result<(), String> {
    fs::create_dir_all(dock_root()).map_err(|e| e.to_string())?;
    let listener = TcpListener::bind(&opts.bind).map_err(|e| {
        format!("cannot bind dock on {}: {e}", opts.bind)
    })?;
    eprintln!("buraaq dock");
    eprintln!("  http://{}", opts.bind);
    eprintln!("  token file {}", token_path().display());
    eprintln!("  PUT /v1/apps/<name>  with Authorization: Bearer <token>");
    for incoming in listener.incoming() {
        match incoming {
            Ok(s) => {
                if let Err(e) = handle_conn(s, &opts.token) {
                    eprintln!("dock: {e}");
                }
            }
            Err(e) => eprintln!("dock accept: {e}"),
        }
    }
    Ok(())
}

fn handle_conn(mut s: TcpStream, token: &str) -> Result<(), String> {
    let mut head = Vec::new();
    let mut tmp = [0u8; 1];
    while head.len() < 64 * 1024 {
        let n = s.read(&mut tmp).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        head.push(tmp[0]);
        if head.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }
    let header_end = head
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| "bad request".to_string())?;
    let header = std::str::from_utf8(&head[..header_end]).unwrap_or("");
    let mut method = "";
    let mut path = "";
    let mut first = true;
    let mut content_len: usize = 0;
    let mut auth = String::new();
    for line in header.lines() {
        if first {
            let mut it = line.split_whitespace();
            method = it.next().unwrap_or("");
            path = it.next().unwrap_or("");
            first = false;
            continue;
        }
        let l = line.to_ascii_lowercase();
        if let Some(v) = l.strip_prefix("content-length:") {
            content_len = v.trim().parse().unwrap_or(0);
        }
        if let Some(v) = line.strip_prefix("Authorization:") {
            auth = v.trim().to_string();
        }
    }
    let extra = head.len().saturating_sub(header_end + 4);
    let mut body = Vec::new();
    if extra > 0 {
        body.extend_from_slice(&head[header_end + 4..]);
    }
    const MAX: usize = 512 * 1024 * 1024;
    if content_len > MAX {
        return reply(&mut s, 413, "bundle too large");
    }
    while body.len() < content_len {
        let mut buf = vec![0u8; (content_len - body.len()).min(1 << 20)];
        let n = s.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&buf[..n]);
    }

    if method == "GET" && path == "/v1/health" {
        return reply(&mut s, 200, "ok");
    }
    if method == "OPTIONS" {
        return reply(&mut s, 204, "");
    }
    if !auth_ok(token, &auth) {
        return reply(&mut s, 401, "unauthorized");
    }
    if method == "GET" && path == "/v1/apps" {
        let mut out = String::new();
        for (name, digest, pid) in list_live().unwrap_or_default() {
            out.push_str(&format!(
                "{name} digest={} pid={}\n",
                digest,
                pid.map(|p| p.to_string()).unwrap_or_else(|| "-".into())
            ));
        }
        if out.is_empty() {
            out.push_str("(none)\n");
        }
        return reply(&mut s, 200, &out);
    }
    if let Some(name) = path.strip_prefix("/v1/apps/") {
        let name = name.trim_end_matches('/');
        if name.is_empty() || name.contains('/') || !ident_ok(name) {
            return reply(&mut s, 400, "bad app name");
        }
        if method == "PUT" {
            let bundle = match unpack(&body) {
                Ok(b) => b,
                Err(e) => return reply(&mut s, 400, &e.to_string()),
            };
            if bundle.name != name {
                return reply(
                    &mut s,
                    400,
                    &format!("bundle name `{}` does not match path `{name}`", bundle.name),
                );
            }
            match replace_live(&bundle, &body) {
                Ok(pid) => {
                    return reply(
                        &mut s,
                        200,
                        &format!("running {name} pid={pid} digest={}", bundle.digest),
                    );
                }
                Err(e) => return reply(&mut s, 500, &e.to_string()),
            }
        }
        if method == "DELETE" {
            stop_named(name).map_err(|e| e.to_string())?;
            return reply(&mut s, 200, "stopped");
        }
        if method == "GET" {
            let live = crate::paths::live_dir(name);
            if !live.is_dir() {
                return reply(&mut s, 404, "not found");
            }
            let digest = fs::read_to_string(live.join("digest")).unwrap_or_default();
            let pid = fs::read_to_string(live.join("app.pid")).unwrap_or_default();
            return reply(
                &mut s,
                200,
                &format!("name={name}\ndigest={}\npid={}", digest.trim(), pid.trim()),
            );
        }
    }
    reply(&mut s, 404, "not found")
}

fn auth_ok(token: &str, header: &str) -> bool {
    let got = header.strip_prefix("Bearer ").unwrap_or(header).trim();
    if token.is_empty() || got.len() != token.len() {
        return false;
    }
    let mut d = 0u8;
    for (a, b) in token.bytes().zip(got.bytes()) {
        d |= a ^ b;
    }
    d == 0
}

fn reply(s: &mut TcpStream, status: u16, body: &str) -> Result<(), String> {
    let reason = match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        413 => "Payload Too Large",
        _ => "Error",
    };
    let msg = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\n\r\n{body}",
        body.len()
    );
    s.write_all(msg.as_bytes()).map_err(|e| e.to_string())
}

pub fn load_or_create_token() -> Result<String, String> {
    fs::create_dir_all(dock_root()).map_err(|e| e.to_string())?;
    let p = token_path();
    if p.is_file() {
        return Ok(fs::read_to_string(p).map_err(|e| e.to_string())?.trim().to_string());
    }
    let tok = random_token();
    fs::write(&p, &tok).map_err(|e| e.to_string())?;
    eprintln!("  created token {}", p.display());
    Ok(tok)
}

fn random_token() -> String {
    let mut buf = [0u8; 32];
    fill_random(&mut buf);
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

fn fill_random(buf: &mut [u8]) {
    #[cfg(windows)]
    {
        unsafe {
            let _ = SystemFunction036(buf.as_mut_ptr(), buf.len() as u32);
        }
    }
    #[cfg(unix)]
    {
        if let Ok(mut f) = fs::File::open("/dev/urandom") {
            let _ = f.read_exact(buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::thread;
    use std::time::Duration;

    fn raw(addr: &str, req: &str) -> String {
        let mut s = TcpStream::connect(addr).unwrap();
        s.write_all(req.as_bytes()).unwrap();
        let mut buf = Vec::new();
        s.read_to_end(&mut buf).unwrap();
        String::from_utf8_lossy(&buf).into_owned()
    }

    #[test]
    fn health_open_apps_need_token() {
        let bind = format!("127.0.0.1:{}", 17000 + (std::process::id() % 2000));
        let bind2 = bind.clone();
        thread::spawn(move || {
            let _ = serve_dock(DockOptions {
                bind: bind2,
                token: "secret-dock-token".into(),
            });
        });
        let mut ok = false;
        for _ in 0..40 {
            if TcpStream::connect(&bind).is_ok() {
                ok = true;
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
        assert!(ok, "dock did not bind {bind}");
        let health = raw(
            &bind,
            "GET /v1/health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        );
        assert!(health.contains("200"), "{health}");
        let unauth = raw(
            &bind,
            "GET /v1/apps HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        );
        assert!(unauth.contains("401"), "{unauth}");
        let auth = raw(
            &bind,
            "GET /v1/apps HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer secret-dock-token\r\nConnection: close\r\n\r\n",
        );
        assert!(auth.contains("200"), "{auth}");
        let bad = raw(
            &bind,
            "GET /v1/apps HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer wrong\r\nConnection: close\r\n\r\n",
        );
        assert!(bad.contains("401"), "{bad}");
    }
}
