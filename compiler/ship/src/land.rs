//! Land — put a Dock on any cloud VM or bare metal.
//!
//! One script, one systemd unit. AWS / Azure / GCP / Hetzner / a box in a
//! rack are the same: open ports, run Land, then `buraaq ship HOST`.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cloud {
    Aws,
    Azure,
    Gcp,
    Hetzner,
    Bare,
}

impl Cloud {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "aws" | "amazon" | "ec2" => Some(Self::Aws),
            "azure" => Some(Self::Azure),
            "gcp" | "google" | "gce" => Some(Self::Gcp),
            "hetzner" | "hz" | "hcloud" => Some(Self::Hetzner),
            "bare" | "baremetal" | "metal" | "vps" | "any" => Some(Self::Bare),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Aws => "AWS",
            Self::Azure => "Azure",
            Self::Gcp => "GCP",
            Self::Hetzner => "Hetzner",
            Self::Bare => "bare metal",
        }
    }

    pub fn firewall(self) -> &'static str {
        match self {
            Self::Aws => "EC2 security group: inbound TCP 7422 (Dock), 8080, 8443 (Keel). 443 if you terminate TLS at a load balancer.",
            Self::Azure => "NSG: inbound TCP 7422, 8080, 8443 (and 443 if used).",
            Self::Gcp => "VPC firewall: tcp:7422,8080,8443 (and 443 if used).",
            Self::Hetzner => "Hetzner Cloud Firewall: inbound TCP 7422 (Dock), 8080, 8443 (Keel). Add 443 if TLS terminates at a load balancer. ufw: allow 7422,8080,8443/tcp.",
            Self::Bare => "Open TCP 7422, 8080, 8443 on the host firewall (nftables, ufw, or the datacenter panel).",
        }
    }
}

#[derive(Debug, Error)]
pub enum LandError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    Msg(String),
}

pub fn write_kit(dir: &Path, cloud: Cloud) -> Result<PathBuf, LandError> {
    fs::create_dir_all(dir)?;
    fs::write(dir.join("land.sh"), LAND_SH)?;
    fs::write(dir.join("buraaq-dock.service"), DOCK_UNIT)?;
    fs::write(dir.join("env.example"), ENV_EXAMPLE)?;
    fs::write(dir.join("cloud.txt"), format!("{}\n{}\n", cloud.label(), cloud.firewall()))?;
    fs::write(dir.join("NEXT.txt"), next_txt(cloud))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(dir.join("land.sh"))?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(dir.join("land.sh"), perms)?;
    }
    Ok(dir.to_path_buf())
}

fn next_txt(cloud: Cloud) -> String {
    format!(
        "Buraaq Land - {label}\n\n\
         1. Copy a Linux `buraaq` binary onto the host (`/usr/local/bin/buraaq`).\n\
            Pack the app ON THAT OS: a Windows .bur will not run on Linux.\n\
         2. {fw}\n\
         3. On the host:  bash land.sh\n\
         4. Copy ~/.buraaq/dock/token (or BURAAQ_DOCK_TOKEN) to your laptop.\n\
         5. Copy env.example to ~/.buraaq/dock/env and set BURAAQ_DATABASE_URL\n\
            (Postgres next to the app. Neon: sslmode=require. Dock copies this into\n\
            every launched ship so Keel can reconnect after the pooler drops idle).\n\
         6. From the project on Linux:  buraaq pack && buraaq ship HOST\n\n\
         Same path on AWS, Azure, GCP, Hetzner, and a rack. The cloud is a hostname.\n",
        label = cloud.label(),
        fw = cloud.firewall()
    )
}

pub fn ssh_bootstrap(spec: &str, kit: &Path) -> Result<(), LandError> {
    let script = kit.join("land.sh");
    if !script.is_file() {
        return Err(LandError::Msg("land.sh missing — write the kit first".into()));
    }
    let scp = Command::new("scp")
        .args([
            script.to_str().ok_or_else(|| LandError::Msg("path".into()))?,
            &format!("{spec}:/tmp/buraaq-land.sh"),
        ])
        .status()
        .map_err(|e| LandError::Msg(format!("scp not available: {e}")))?;
    if !scp.success() {
        return Err(LandError::Msg(
            "scp failed — copy target/land/land.sh to the host and run bash land.sh".into(),
        ));
    }
    let ssh = Command::new("ssh")
        .args([spec, "bash", "/tmp/buraaq-land.sh"])
        .status()
        .map_err(|e| LandError::Msg(format!("ssh not available: {e}")))?;
    if !ssh.success() {
        return Err(LandError::Msg("remote land.sh exited non-zero".into()));
    }
    Ok(())
}

const LAND_SH: &str = r#"#!/bin/sh
# Buraaq Land — install Dock on this host (Linux).
set -e
echo "Buraaq Land"
if ! command -v buraaq >/dev/null 2>&1; then
  echo "buraaq is not on PATH."
  echo "Copy the Linux compiler to /usr/local/bin/buraaq, then re-run: bash land.sh"
  exit 1
fi
install_libpq() {
  if ldconfig -p 2>/dev/null | grep -q 'libpq.so'; then
    return 0
  fi
  echo "Installing libpq5 (Keel Postgres client)"
  if command -v apt-get >/dev/null 2>&1; then
    if [ "$(id -u)" = 0 ]; then
      apt-get update -qq && apt-get install -y libpq5 || true
    elif command -v sudo >/dev/null 2>&1; then
      sudo apt-get update -qq && sudo apt-get install -y libpq5 || true
    fi
  fi
}
install_libpq
mkdir -p "$HOME/.buraaq/dock"
if [ ! -f "$HOME/.buraaq/dock/env" ]; then
  cat > "$HOME/.buraaq/dock/env" <<'ENV'
# Postgres for Keel apps launched by Dock. Neon: sslmode=require.
# BURAAQ_DATABASE_URL=
# BURAAQ_API_KEY=
ENV
  chmod 600 "$HOME/.buraaq/dock/env" 2>/dev/null || true
  echo "Wrote $HOME/.buraaq/dock/env — set BURAAQ_DATABASE_URL before shipping."
fi
load_dock_env() {
  if [ -f "$HOME/.buraaq/dock/env" ]; then
    set -a
    # shellcheck disable=SC1091
    . "$HOME/.buraaq/dock/env"
    set +a
  fi
}
UNIT_DIR="$HOME/.config/systemd/user"
if command -v systemctl >/dev/null 2>&1; then
  mkdir -p "$UNIT_DIR"
  cat > "$UNIT_DIR/buraaq-dock.service" <<'UNIT'
[Unit]
Description=Buraaq Dock
After=network.target

[Service]
ExecStart=buraaq dock --public
Restart=on-failure
EnvironmentFile=-%h/.buraaq/dock/env

[Install]
WantedBy=default.target
UNIT
  systemctl --user daemon-reload || true
  systemctl --user enable --now buraaq-dock.service || {
    echo "user systemd not available; starting Dock in the background"
    load_dock_env
    nohup buraaq dock --public >/tmp/buraaq-dock.log 2>&1 &
  }
else
  load_dock_env
  nohup buraaq dock --public >/tmp/buraaq-dock.log 2>&1 &
  echo "no systemd — Dock started with nohup (log /tmp/buraaq-dock.log)"
fi
echo "Dock should answer on :7422"
echo "Token: $HOME/.buraaq/dock/token"
echo "App TLS is Keel on 8443. Control plane is HTTP + token — put a reverse proxy in front if this host is public."
echo "If this is Hetzner: allow TCP 7422, 8080, 8443 on the Cloud Firewall."
"#;

const DOCK_UNIT: &str = r#"[Unit]
Description=Buraaq Dock
After=network.target

[Service]
ExecStart=buraaq dock --public
Restart=on-failure
EnvironmentFile=-%h/.buraaq/dock/env

[Install]
WantedBy=default.target
"#;

const ENV_EXAMPLE: &str = "# Copy to ~/.buraaq/dock/env on the host. Do not commit secrets.\n\
# Neon pooler: postgresql://USER:PASS@HOST/neondb?sslmode=require\n\
BURAAQ_DATABASE_URL=\n\
BURAAQ_API_KEY=\n";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hetzner_kit_is_ready_to_copy() {
        let dir = std::env::temp_dir().join(format!("bq-land-hz-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        write_kit(&dir, Cloud::Hetzner).expect("write hetzner kit");
        let next = fs::read_to_string(dir.join("NEXT.txt")).unwrap();
        assert!(next.contains("Hetzner"), "{next}");
        assert!(next.contains("7422"), "{next}");
        assert!(next.contains("BURAAQ_DATABASE_URL"), "{next}");
        let sh = fs::read_to_string(dir.join("land.sh")).unwrap();
        assert!(sh.contains("buraaq dock --public"), "{sh}");
        assert!(sh.contains("EnvironmentFile"), "{sh}");
        assert!(sh.contains("libpq5"), "{sh}");
        assert!(sh.contains("load_dock_env"), "{sh}");
        let cloud = fs::read_to_string(dir.join("cloud.txt")).unwrap();
        assert!(cloud.contains("Hetzner"), "{cloud}");
        let env = fs::read_to_string(dir.join("env.example")).unwrap();
        assert!(env.contains("BURAAQ_DATABASE_URL"), "{env}");
        assert!(!env.contains("npg_"), "must not ship a live password");
        let _ = fs::remove_dir_all(&dir);
    }
}
