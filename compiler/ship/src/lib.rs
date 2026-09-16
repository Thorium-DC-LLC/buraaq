//! Buraaq Ship — native bundles (`.bur`), local launch, and a host dock.
//!
//! This is not Linux containers. A ship is a hashed archive of a compiled
//! binary plus its files. A dock is a small host agent that receives ships
//! and runs them as processes.

mod bundle;
mod client;
mod dock;
mod land;
mod launch;
mod paths;

pub use bundle::{extract_to, ident_ok, pack, unpack, Bundle, PackRequest};
pub use client::{
    delete as dock_delete, dock_addr, get as dock_get, health as dock_health, normalize_host, push,
    resolve_token,
};
pub use dock::{load_or_create_token, serve_dock, DockOptions};
pub use land::{ssh_bootstrap, write_kit as write_land_kit, Cloud};
pub use launch::{
    launch_bundle, launch_extracted, list_live, replace_live, stop_named, stop_pid, LaunchMode,
};

pub use paths::{dock_root, token_path};

pub const DOCK_PORT: u16 = 7422;
pub const MAGIC: &[u8; 4] = b"BUR1";
