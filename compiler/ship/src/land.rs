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
            (one file; Neon: sslmode=require). That is the only host config.\n\
            Dock binds loopback by default — ship via SSH tunnel:\n\
              ssh -L 7422:127.0.0.1:7422 user@HOST\n\
              buraaq ship 127.0.0.1\n\
            Or edit the unit to `buraaq dock --public` if you intentionally expose :7422.\n\
            `buraaq ship` then installs systemd Restart=always for the app.\n\
         6. From the project on Linux:  buraaq pack && buraaq ship 127.0.0.1\n\n\
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
# This is the only host file you edit. Ship copies it into every app.
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
start_dock_systemd() {
  if [ "$(id -u)" = 0 ] && [ -d /etc/systemd/system ]; then
    cat > /etc/systemd/system/buraaq-dock.service <<'UNIT'
[Unit]
Description=Buraaq Dock
After=network-online.target
Wants=network-online.target

[Service]
ExecStart=buraaq dock
Restart=always
RestartSec=3
EnvironmentFile=-/root/.buraaq/dock/env

[Install]
WantedBy=multi-user.target
UNIT
    systemctl daemon-reload
    systemctl enable --now buraaq-dock.service
    echo "Dock systemd unit: buraaq-dock.service (loopback :7422, Restart=always)"
    return 0
  fi
  if command -v systemctl >/dev/null 2>&1; then
    UNIT_DIR="$HOME/.config/systemd/user"
    mkdir -p "$UNIT_DIR"
    cat > "$UNIT_DIR/buraaq-dock.service" <<'UNIT'
[Unit]
Description=Buraaq Dock
After=network.target

[Service]
ExecStart=buraaq dock
Restart=always
RestartSec=3
EnvironmentFile=-%h/.buraaq/dock/env

[Install]
WantedBy=default.target
UNIT
    loginctl enable-linger "$(id -un)" 2>/dev/null || true
    systemctl --user daemon-reload || true
    systemctl --user enable --now buraaq-dock.service && return 0
  fi
  return 1
}
if command -v systemctl >/dev/null 2>&1 && start_dock_systemd; then
  :
else
  load_dock_env
  nohup buraaq dock >/tmp/buraaq-dock.log 2>&1 &
  echo "no systemd — Dock started with nohup on loopback (log /tmp/buraaq-dock.log)"
fi
echo "Dock should answer on 127.0.0.1:7422"
echo "Token: $HOME/.buraaq/dock/token"
echo "One config file: $HOME/.buraaq/dock/env  (BURAAQ_DATABASE_URL)."
echo "Ship via SSH tunnel: ssh -L 7422:127.0.0.1:7422 user@HOST && buraaq ship 127.0.0.1"
echo "To expose Dock on the network, change ExecStart to: buraaq dock --public"
echo "Apps get systemd Restart=always when you ship. Do not nohup the Keel binary by hand."
echo "App TLS is Keel on 8443. Control plane is HTTP + token on loopback by default."
echo "If this is Hetzner: allow TCP 8080, 8443 on the Cloud Firewall (7422 only if --public)."
"#;

const DOCK_UNIT: &str = r#"[Unit]
Description=Buraaq Dock
After=network.target

[Service]
ExecStart=buraaq dock
Restart=always
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
        assert!(sh.contains("ExecStart=buraaq dock\n"), "{sh}");
        assert!(!sh.contains("ExecStart=buraaq dock --public"), "{sh}");
        assert!(sh.contains("EnvironmentFile"), "{sh}");
        assert!(sh.contains("libpq5"), "{sh}");
        assert!(sh.contains("load_dock_env"), "{sh}");
        assert!(sh.contains("Restart=always"), "{sh}");
        assert!(sh.contains("/etc/systemd/system"), "{sh}");
        assert!(sh.contains("ssh -L 7422"), "{sh}");
        let cloud = fs::read_to_string(dir.join("cloud.txt")).unwrap();
        assert!(cloud.contains("Hetzner"), "{cloud}");
        let env = fs::read_to_string(dir.join("env.example")).unwrap();
        assert!(env.contains("BURAAQ_DATABASE_URL"), "{env}");
        assert!(!env.contains("npg_"), "must not ship a live password");
        let _ = fs::remove_dir_all(&dir);
    }
}
