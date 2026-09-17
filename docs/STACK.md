# The Buraaq stack — Keel, Stream, Hold, Grid, Ship, Land, Mind

Buraaq 1.0. Nine names. One path from source to any host. **Lumen** is optional (native window) — not the default.

| Name | What it is | You type |
|------|------------|----------|
| **Keel** | APIs + TLS pages. The hull of the app. | `page`, `api`, `run` |
| **Stream** | Live frames (the WebSocket job). | `stream`, `wire`, `say`, `hear` |
| **Hold** | Named columns (the Pandas job). | `hold`, `stow`, `pick` |
| **Grid** | Numeric arrays (the NumPy job). | `zeros`, `dot`, `matmul` |
| **Ship** | Hashed native bundle (`.bur`). Not a VM. | `buraaq pack` / `buraaq ship` |
| **Dock** | Tiny host agent that receives ships and runs them. | `buraaq dock` |
| **Land** | Same install on AWS, Azure, GCP, Hetzner, or bare metal. | `buraaq land` / `buraaq up` |
| **Mind** | Local models, planning, OpenAI-compatible serve. | `buraaq ai` / `std.ai` |
| **Lumen** | Optional native HD UI. | `app`, `heading`, `show` |

Complexity stays in the compiler and runtime. Application source stays three lines:

```buraaq
fn main() {
    page("/", "public/index.html")
    api("items", "title, body")
    run()
}
```

## Local

```text
buraaq new notes
cd notes
# set BURAAQ_DATABASE_URL to Postgres
buraaq up
```

`up` packs a Ship, starts Dock on this machine if needed, and launches the app. Open `http://127.0.0.1:8080` and `https://127.0.0.1:8443`.

`buraaq new NAME --cli` is the hello-world exception (no server).

`std.service` still works — it is the old name for Keel.

## Any cloud or a rack

The cloud is a hostname plus a firewall. There is no AWS SDK, no Kubernetes, no Dockerfile.

```text
buraaq land user@HOST --cloud hetzner
buraaq ship HOST
```

`--cloud` is `aws`, `azure`, `gcp`, `hetzner`, or `bare`. It only changes the firewall reminder. Land writes `target/land/land.sh` and a systemd unit. Pack the `.bur` **on the same OS as the host** (a Windows ship will not run on Linux).

Contract for Keel: [SERVICE.md](SERVICE.md). Lumen: [LUMEN.md](LUMEN.md). Ship/Dock: [SHIP.md](SHIP.md). Mind: [AI.md](AI.md). Walkthrough: [CRUD_API.md](CRUD_API.md).

## What this is not

Land does not provision VMs, databases, or TLS certificates at the edge. Dock is a process + directory + token, not kernel isolation. Mind runs inference backends as peer processes (or a remote URL) — not a claimed VM boundary. Treat that honestly when you share a host.
