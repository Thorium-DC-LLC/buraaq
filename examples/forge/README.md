# Forge

Production Buraaq app: ownership, borrows, spawn, generics, enums, Keel APIs, Ship, Land.

Postgres is **not** in source. Set `BURAAQ_DATABASE_URL` on the **host** (Neon pooler: `sslmode=require`). Keel reconnects if the pooler drops an idle connection. On Hetzner that belongs in `/opt/buraaq/forge.env` or `~/.buraaq/dock/env` — not in git.

```text
# language coverage only (no HTTP)
set BURAAQ_COVERAGE_ONLY=1
buraaq run

# Keel against Neon
set BURAAQ_DATABASE_URL=...
buraaq run

# local ship
buraaq pack
buraaq up

# Hetzner Dock (Linux host; pack the .bur on Linux)
buraaq land --cloud hetzner
# copy target/land/land.sh to the VM, then:
#   buraaq land user@HOST --cloud hetzner
#   buraaq pack && buraaq ship HOST
```

A Windows `.bur` will not run on a Linux Hetzner box. Land writes the kit either way.

Writes from curl need `X-Api-Key: forge-key` unless the browser posts same-origin.
