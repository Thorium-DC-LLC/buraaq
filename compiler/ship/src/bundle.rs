use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use thiserror::Error;
use walkdir::WalkDir;

use crate::MAGIC;

#[derive(Debug, Error)]
pub enum BundleError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    Msg(String),
}

#[derive(Clone, Debug)]
pub struct PackRequest {
    pub name: String,
    pub version: String,
    pub exe: PathBuf,
    pub root: PathBuf,
    pub dest: PathBuf,
}

#[derive(Clone, Debug)]
pub struct Bundle {
    pub name: String,
    pub version: String,
    pub exe: String,
    pub digest: String,
    pub files: Vec<(String, Vec<u8>)>,
}

pub fn pack(req: &PackRequest) -> Result<PathBuf, BundleError> {
    if !req.exe.is_file() {
        return Err(BundleError::Msg(format!(
            "binary not found: {}",
            req.exe.display()
        )));
    }
    if !ident_ok(&req.name) {
        return Err(BundleError::Msg(
            "app name must be letters, digits, or _".into(),
        ));
    }

    let exe_name = req
        .exe
        .file_name()
        .ok_or_else(|| BundleError::Msg("binary has no file name".into()))?
        .to_string_lossy()
        .into_owned();

    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    files.push((
        format!("bin/{exe_name}"),
        fs::read(&req.exe)?,
    ));

    let pkg = req.root.join("buraaq.pkg");
    if pkg.is_file() {
        files.push(("buraaq.pkg".into(), fs::read(&pkg)?));
    }
    for name in ["cert.pem", "key.pem"] {
        let p = req.root.join(name);
        if p.is_file() {
            files.push((name.into(), fs::read(&p)?));
        }
    }
    let public = req.root.join("public");
    if public.is_dir() {
        for entry in WalkDir::new(&public).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }
            let rel = entry
                .path()
                .strip_prefix(&req.root)
                .unwrap_or(entry.path());
            let key = rel.to_string_lossy().replace('\\', "/");
            if !path_ok(&key) {
                continue;
            }
            files.push((key, fs::read(entry.path())?));
        }
    }

    let payload = encode(&req.name, &req.version, &exe_name, &files)?;
    if let Some(parent) = req.dest.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&req.dest, payload)?;
    Ok(req.dest.clone())
}

pub fn unpack(bytes: &[u8]) -> Result<Bundle, BundleError> {
    decode(bytes)
}

pub fn extract_to(bundle: &Bundle, dir: &Path) -> Result<(), BundleError> {
    fs::create_dir_all(dir)?;
    for (path, data) in &bundle.files {
        if !path_ok(path) {
            return Err(BundleError::Msg(format!("unsafe path in bundle: {path}")));
        }
        let dest = dir.join(path);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(dest, data)?;
    }
    Ok(())
}

fn encode(
    name: &str,
    version: &str,
    exe: &str,
    files: &[(String, Vec<u8>)],
) -> Result<Vec<u8>, BundleError> {
    let manifest = format!("name={name}\nversion={version}\nexe={exe}\nfiles={}\n", files.len());
    let mut body = Vec::new();
    body.extend_from_slice(MAGIC);
    body.extend_from_slice(&1u32.to_le_bytes());
    let mb = manifest.as_bytes();
    body.extend_from_slice(&(mb.len() as u32).to_le_bytes());
    body.extend_from_slice(mb);
    body.extend_from_slice(&(files.len() as u32).to_le_bytes());
    for (path, data) in files {
        if !path_ok(path) {
            return Err(BundleError::Msg(format!("unsafe path: {path}")));
        }
        let pb = path.as_bytes();
        if pb.len() > 65535 {
            return Err(BundleError::Msg("path too long".into()));
        }
        body.extend_from_slice(&(pb.len() as u16).to_le_bytes());
        body.extend_from_slice(pb);
        body.extend_from_slice(&(data.len() as u64).to_le_bytes());
        body.extend_from_slice(data);
    }
    let digest = Sha256::digest(&body);
    body.extend_from_slice(&digest);
    Ok(body)
}

fn decode(bytes: &[u8]) -> Result<Bundle, BundleError> {
    if bytes.len() < 4 + 4 + 4 + 32 {
        return Err(BundleError::Msg("bundle too small".into()));
    }
    let (payload, digest) = bytes.split_at(bytes.len() - 32);
    let got = Sha256::digest(payload);
    if !ct_eq(&got, digest) {
        return Err(BundleError::Msg("bundle hash mismatch — file is corrupt or tampered".into()));
    }
    if &payload[0..4] != MAGIC {
        return Err(BundleError::Msg("not a Buraaq ship (.bur)".into()));
    }
    let mut i = 4;
    let ver = read_u32(payload, &mut i)?;
    if ver != 1 {
        return Err(BundleError::Msg(format!("unsupported ship version {ver}")));
    }
    let mlen = read_u32(payload, &mut i)? as usize;
    if i + mlen > payload.len() {
        return Err(BundleError::Msg("truncated manifest".into()));
    }
    let manifest = std::str::from_utf8(&payload[i..i + mlen])
        .map_err(|_| BundleError::Msg("manifest is not utf-8".into()))?;
    i += mlen;
    let meta = parse_manifest(manifest)?;
    let nfiles = read_u32(payload, &mut i)? as usize;
    let mut files = Vec::with_capacity(nfiles);
    for _ in 0..nfiles {
        let plen = read_u16(payload, &mut i)? as usize;
        if i + plen > payload.len() {
            return Err(BundleError::Msg("truncated path".into()));
        }
        let path = std::str::from_utf8(&payload[i..i + plen])
            .map_err(|_| BundleError::Msg("path is not utf-8".into()))?
            .to_string();
        i += plen;
        if !path_ok(&path) {
            return Err(BundleError::Msg(format!("unsafe path in bundle: {path}")));
        }
        let size = read_u64(payload, &mut i)? as usize;
        if i + size > payload.len() {
            return Err(BundleError::Msg("truncated file".into()));
        }
        files.push((path, payload[i..i + size].to_vec()));
        i += size;
    }
    Ok(Bundle {
        name: meta.0,
        version: meta.1,
        exe: meta.2,
        digest: hex(&got),
        files,
    })
}

fn parse_manifest(s: &str) -> Result<(String, String, String), BundleError> {
    let mut name = String::new();
    let mut version = String::new();
    let mut exe = String::new();
    for line in s.lines() {
        if let Some(v) = line.strip_prefix("name=") {
            name = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("version=") {
            version = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("exe=") {
            exe = v.trim().to_string();
        }
    }
    if name.is_empty() || exe.is_empty() {
        return Err(BundleError::Msg("manifest missing name or exe".into()));
    }
    if !ident_ok(&name) {
        return Err(BundleError::Msg("invalid app name in manifest".into()));
    }
    Ok((name, version, exe))
}

fn read_u16(b: &[u8], i: &mut usize) -> Result<u16, BundleError> {
    if *i + 2 > b.len() {
        return Err(BundleError::Msg("truncated u16".into()));
    }
    let v = u16::from_le_bytes(b[*i..*i + 2].try_into().unwrap());
    *i += 2;
    Ok(v)
}

fn read_u32(b: &[u8], i: &mut usize) -> Result<u32, BundleError> {
    if *i + 4 > b.len() {
        return Err(BundleError::Msg("truncated u32".into()));
    }
    let v = u32::from_le_bytes(b[*i..*i + 4].try_into().unwrap());
    *i += 4;
    Ok(v)
}

fn read_u64(b: &[u8], i: &mut usize) -> Result<u64, BundleError> {
    if *i + 8 > b.len() {
        return Err(BundleError::Msg("truncated u64".into()));
    }
    let v = u64::from_le_bytes(b[*i..*i + 8].try_into().unwrap());
    *i += 8;
    Ok(v)
}

pub fn ident_ok(s: &str) -> bool {
    let mut c = s.chars();
    match c.next() {
        Some(ch) if ch.is_ascii_alphabetic() || ch == '_' => {}
        _ => return false,
    }
    c.all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
}

pub fn path_ok(p: &str) -> bool {
    if p.is_empty() || p.starts_with('/') || p.contains("..") || p.contains('\\') {
        return false;
    }
    p.chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' ))
}

fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut d = 0u8;
    for i in 0..a.len() {
        d |= a[i] ^ b[i];
    }
    d == 0
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_tamper() {
        let files = vec![
            ("bin/app.exe".into(), b"hello-bin".to_vec()),
            ("public/index.html".into(), b"<h1>ok</h1>".to_vec()),
        ];
        let bytes = encode("notes", "0.1.0", "app.exe", &files).unwrap();
        let b = decode(&bytes).unwrap();
        assert_eq!(b.name, "notes");
        assert_eq!(b.exe, "app.exe");
        assert_eq!(b.files.len(), 2);
        let mut bad = bytes.clone();
        let n = bad.len();
        bad[n / 2] ^= 1;
        assert!(decode(&bad).is_err());
        assert!(!path_ok("../etc/passwd"));
        assert!(!path_ok("/abs"));
        assert!(path_ok("public/index.html"));
    }
}
