# Buraaq Ship — pack once, run on any host

**The one obvious way to ship a Buraaq service.** Not Docker: no guest OS, no image layers, no daemon-as-a-product. A **ship** is a hashed native bundle (`.bur`). A **dock** is a small host agent that receives ships and runs them as processes.

Complexity stays in the toolchain. You still write:

```buraaq
use std.keel.{page, api, run}

fn main() {
    page("/", "public/index.html")
    api("items", "title, body")
    run()
}
```

Then locally:

```powershell
buraaq up
```

On a server (AWS, Azure, GCP, Hetzner, or a rack):

```powershell
buraaq land user@HOST --cloud hetzner
buraaq ship HOST
```

Names: [STACK.md](STACK.md). Keel: [SERVICE.md](SERVICE.md). Policy: [../SECURITY.md](../SECURITY.md).

---

## Why this is not Docker

| Docker | Buraaq Ship |
|--------|-------------|
| Guest Linux + runtime + layers | One native binary + its files |
| `Dockerfile`, registry, compose | `buraaq pack` / `ship` / `dock` |
| Minutes of pull/extract | Copy a `.bur` and run |
| Namespace / cgroup isolation | Dedicated directory + process + SHA-256 pin + dock token |

Use Docker when you need a foreign OS. Use Ship when the app **is** Buraaq — compiled for the host, same Keel TLS API.

---

## Commands

| Command | What it does |
|---------|----------------|
| `buraaq up` | Pack, start local Dock if needed, push and run |
| `buraaq pack` | Release-build, write `target/ship/<app>.bur` |
| `buraaq land [user@HOST]` | Write Land kit; optional SSH install of Dock |
| `buraaq launch FILE.bur` | Verify hash, extract, run in the foreground |
| `buraaq ship` | Pack and launch locally |
| `buraaq ship HOST` | Pack and **PUT** the bundle to a dock |
| `buraaq ship HOST --bundle FILE.bur` | Push a ship you already packed (same OS as the host) |
| `buraaq ship --status HOST` | List apps on a dock |
| `buraaq ship --stop NAME HOST` | Stop an app on a dock |
| `buraaq dock` | Host agent on `127.0.0.1:7422` |
| `buraaq dock --public` | Bind `0.0.0.0:7422` |
| `buraaq dock --bind ADDR` | Custom bind (example `0.0.0.0:7422`) |

The `.bur` includes:

- `bin/<exe>` — release native binary
- `public/**` — if present
- `cert.pem` / `key.pem` — if present
- `buraaq.pkg`

Every byte before the trailing digest is SHA-256 hashed. Tamper → refuse to launch.

---

## Dock protocol

Control plane is HTTP/1.1 on port **7422**.

| Method | Path | Role |
|--------|------|------|
| `GET` | `/v1/health` | Liveness (no token) |
| `GET` | `/v1/apps` | List (token) |
| `GET` | `/v1/apps/:name` | One app (token) |
| `PUT` | `/v1/apps/:name` | Upload `.bur` and run (token) |
| `DELETE` | `/v1/apps/:name` | Stop (token) |

Auth: `Authorization: Bearer <token>`.

Token sources, in order:

1. `BURAAQ_DOCK_TOKEN`
2. `%USERPROFILE%\.buraaq\dock\token` (created on first `buraaq dock`)

Copy that file (or the env var) to the machine that runs `buraaq ship HOST`.

Default bind is **localhost**. `--public` is for a real server; put a TLS reverse proxy in front if the dock is on the internet. This version of the control plane is HTTP + bearer token, not TLS. App traffic is still TLS via `std.service` (`8443`).

Max bundle size: 512 MiB.

---

## Where things live

| Path | Role |
|------|------|
| `target/ship/<app>.bur` | Packed ship |
| `~/.buraaq/run/<app>/<digest>/` | Local `launch` extract |
| `~/.buraaq/dock/live/<app>/` | Dock extract + `app.pid` + `app.log` |
| `~/.buraaq/dock/token` | Dock bearer token |

On Windows, `~` is `%USERPROFILE%`.

---

## Security (this version)

- Bundle integrity: SHA-256 of the archive; mismatch is a hard error.
- Paths inside a ship cannot contain `..`, `\`, or a leading `/`.
- App names are identifiers (`[A-Za-z_][A-Za-z0-9_-]*`).
- Dock mutations require the bearer token (constant-time compare).
- Dock listens on loopback unless you pass `--public`.
- Isolation is a private directory and a child process, **not** a VM or Linux container. Do not treat it as multi-tenant kernel isolation.
- The app still uses `std.service` TLS for users. Postgres credentials stay on the **host** environment (`BURAAQ_DATABASE_URL`), not inside the ship unless you packed them.

---

## Walkthrough

1. Write a Keel program (`page` / `api` / `run`). `public/`, `cert.pem`, and `key.pem` in the project root are packed automatically.
2. Locally: `buraaq up` (pack + local Dock + run).
3. On the host: `buraaq land user@HOST --cloud hetzner` then pack **on that OS** and `buraaq ship HOST`.
4. The only host file you edit is `~/.buraaq/dock/env` (`BURAAQ_DATABASE_URL`). Do not `nohup` the Keel binary. Land + ship install **systemd `Restart=always`** so a crash or idle-DB drop comes back.
5. Users hit `http://HOST:8080` and `https://HOST:8443`. Dock `:7422` is control only — prefer loopback + SSH tunnel on public machines.
6. Secrets stay on the **host**. Never in git.

A Windows `.bur` will not run on Linux. `buraaq build --release --emit-ir --target linux` emits IR for a Linux clang link when you cannot pack on the host.

Forge (`examples/forge`) is the complete app that was landed this way.

## Toolchain, not a module

Apps do not `use std.ship`. Shipping is a CLI concern, like `buraaq build`.
