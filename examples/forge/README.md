# Forge

Production Buraaq app: ownership, borrows, spawn, generics, enums, Keel APIs, Ship, Land.

Postgres is **not** in source. One host file: `~/.buraaq/dock/env` (or `/opt/buraaq/forge.env` if you started the binary by hand). Neon: `sslmode=require`. Never git that file.

Do **not** `nohup` the binary on a VM. Land + ship install systemd `Restart=always`. That is how the next outage is avoided.

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

# Hetzner (Linux host; pack the .bur on Linux)
buraaq land --cloud hetzner
# then on the VM: bash land.sh
# edit ~/.buraaq/dock/env once
# from Linux: buraaq pack && buraaq ship HOST
```

A Windows `.bur` will not run on a Linux Hetzner box.

Writes from curl need `X-Api-Key: forge-key` unless the browser posts same-origin.
