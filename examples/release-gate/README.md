# Release-gate sample programs

Small programs used while exercising Keel, print, and auto-import. UI is not the point.

- `print_types.bq` — `print`/`println` on text, int, float, bool; explicit `print_int` / `print_float` / `print_bool`
- `print_stress.bq` — mixed types, multi-arg space join, interpolation, no `use`
- `grid_hold.bq` — `std.grid` + `std.hold` + `std.math`, no `use`
- `ledger.bq` — a banking CLI without `use`
- `keel_page/` — page-only Keel service (no Postgres) for CORS and API-key checks
