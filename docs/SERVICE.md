# std.keel — production TLS APIs

**Keel** is the one obvious way to run a Buraaq service (`std.keel`). `std.service` is the same module under the old name.

**The one obvious way to run a Buraaq service.** Sockets, TLS, accept loops, headers, table schema, REST, CORS, and health checks belong in the runtime. Application source declares what to serve.

**How to create one:** [CRUD_API.md](CRUD_API.md) — project, `page` / `api` / `run`, TLS, Postgres, curl.

Full source: `stdlib/src/keel.bq` (`stdlib/src/service.bq` is the compatibility name)  
Runtime: `stdlib/runtime/buraaq_server.c`  
Example: `stdlib/examples/keel.bq`  
Play app: `buraaq-play/webapi`

This module ships with the language. `buraaq build` links it automatically. There is no extra package to install.

---

## The whole program

```buraaq
use std.keel.{page, api, run}

fn main() {
    page("/", "public/index.html")
    api("items", "title, body")
    run()
}
```

That program:

- Serves `public/index.html` at `/`
- Creates a Postgres table `items` with `title` and `body`
- Exposes REST at `/api/items` and `/api/items/:id`
- Serves `/api/health`
- Listens with **TLS first** (port 8443) and HTTP beside it (port 8080)
- Handles CORS preflight (`OPTIONS`)

Do not write listen/accept/reply loops for ordinary APIs. Use `std.http` only to **call** other APIs. Use `std.db` only when you need SQL the resource helper does not cover.

---

## API

| Function | Signature | Role |
|----------|-----------|------|
| `page` | `page(path: text, file: text)` | Serve a file at an exact URL path |
| `api` | `api(name: text, fields: text)` | Postgres table + REST under `/api/{name}` |
| `store` | `store(url: text)` | Database URL (optional; else environment) |
| `key` | `key(secret: text)` | API key for this process (else auto-generated) |
| `origin` | `origin(allowed: text)` | CORS allow-list (`*` or comma-separated origins) |
| `call` | `call(url: text) -> text` | GET another HTTP/HTTPS API; return body |
| `run` | `run()` | Bind TLS + HTTP and serve forever |

Call `page` / `api` / `store` before `run`. `run` does not return.

You may register several pages and several APIs:

```buraaq
page("/", "public/index.html")
page("/about", "public/about.html")
api("items", "title, body")
api("users", "name, email")
run()
```

Limits in this runtime: **32 pages**, **16 APIs**, **8 fields** per API.

---

## `page`

`page("/", "public/index.html")` maps an exact path to a file read at request time.

Content-Type is taken from the **file** extension:

| Extension | Content-Type |
|-----------|----------------|
| `.html`, `.htm` | `text/html; charset=utf-8` |
| `.css` | `text/css; charset=utf-8` |
| `.js` | `text/javascript; charset=utf-8` |
| `.json` | `application/json` |
| `.svg` | `image/svg+xml` |
| `.txt` | `text/plain; charset=utf-8` |
| other | `application/octet-stream` |

Missing files return `404`. Paths are exact matches (no directory listing, no glob).

Buraaq string literals interpolate `{...}`. Put HTML/JSON in **files**, not in `"..."` source strings.

---

## `api` — table + REST

`api("items", "title, body")` is the production CRUD helper.

### Names

`name` and each field must be a SQL identifier: start with a letter or `_`, then letters, digits, or `_`. Comma-separated fields; spaces around commas are allowed.

Illegal names are rejected (`api` returns 0 and the resource is not registered).

### Schema

On `run()`, if Postgres connects, the runtime executes:

```sql
CREATE TABLE IF NOT EXISTS items (
  id SERIAL PRIMARY KEY,
  title TEXT NOT NULL DEFAULT '',
  body TEXT NOT NULL DEFAULT '',
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
)
```

`id` and `created_at` are always added. You only name the text columns.

### Routes

| Method | Path | Result |
|--------|------|--------|
| `GET` | `/api/items` | All rows, `ORDER BY id` |
| `POST` | `/api/items` | Insert from JSON body; `201` |
| `GET` | `/api/items/:id` | One row |
| `PUT` | `/api/items/:id` | Update text fields from JSON |
| `DELETE` | `/api/items/:id` | Delete; returns deleted `id` |
| `OPTIONS` | any | `204` (CORS) |
| `GET` | `/api/health` | `SELECT 1` as JSON |

`:id` is a decimal integer path segment only (`/api/items/12`). Extra path after the id is `404`.

### JSON body

`POST` and `PUT` read string fields from the request body. Quoted JSON is the normal form:

```json
{"title":"Ship Buraaq API","body":"Connected to PostgreSQL"}
```

Values are passed through `PQescapeLiteral` before SQL. Do not concatenate user text into SQL in application code.

### JSON responses

Every SQL result is:

```json
{"ok":true,"error":"","rows":[{"id":"1","title":"...","body":"...","created_at":"..."}]}
```

On failure, `ok` is `false`, `error` is a message, `rows` is `[]`. Row values are JSON strings. `id` and `created_at` are selected as text.

---

## TLS (default)

`run()` binds **HTTPS first**, then HTTP on the matching cleartext port.

| `run()` port argument (C) | TLS | HTTP |
|---------------------------|-----|------|
| `0` (what Buraaq `run()` passes) | `8443` | `8080` |
| `443` (if you call the C entry with 443) | `443` | `80` |

Buraaq `run()` always uses the 8443/8080 pair so a normal user can bind without administrator rights. Production on 443/80 is the same runtime with a privileged bind.

### Certificate files

Looked up in this order:

1. `BURAAQ_TLS_CERT` / `BURAAQ_TLS_KEY`
2. `cert.pem` / `key.pem` in the process working directory

OpenSSL is loaded at runtime (`libssl` / `libcrypto`). On Windows the PostgreSQL `bin` directory is a typical source of those DLLs.

Self-signed localhost example (Git OpenSSL):

```text
openssl req -x509 -newkey rsa:2048 -keyout key.pem -out cert.pem -days 365 -nodes -subj "/CN=localhost"
```

If TLS cannot bind, HTTP still serves the same routes and stderr reports the TLS failure. Clients should use `https://` in production; HTTP exists so local tools and first-load pages work.

---

## Database

`api(...)` needs Postgres. Connection order:

1. `store(url)` if you called it
2. Else `BURAAQ_DATABASE_URL` (process env, `~/.buraaq/dock/env`, or `forge.env` next to the binary)
3. Else `DATABASE_URL`
4. Else, on Windows only, `host=localhost` with the current user (SSPI attempt), then `127.0.0.1` / `buraaq_play`
5. Else, on Linux, only if `BURAAQ_PG_LOCAL=1`

Keel reconnects when Neon (or any pooler) drops an idle socket. `/api/health` is `SELECT 1`. libpq is loaded at runtime (`libpq.dll` / `libpq.so.5`). The runtime does not bake in a password. Linux hosts need `libpq5` (`land.sh` installs it). Do not point `BURAAQ_DATABASE_URL` at a random local Docker Postgres on a shared VPS.

Typical URL:

```text
host=127.0.0.1 port=5432 dbname=buraaq_play user=postgres password=...
```

If no API is registered, `run()` still serves pages and does not require Postgres. `/api/health` then reports `not connected`.

`std.db` is the extra-SQL hatch (`connect`, `exec`, `quote`, `disconnect` — `ok` is a language keyword, so the check is `connected()`). Prefer `api(...)` for ordinary CRUD.

---

## Calling other APIs

```buraaq
use std.keel.call

fn main() {
    body = call("https://example.com")
}
```

`call` is a GET of the response body. For client-only programs that never `run()`, use `std.http.get`.

`std.http` is **not** the server. There is no application-level `listen` / `accept` in `std.http`.

---

## HTTP behavior (all replies)

Every response includes:

```text
Access-Control-Allow-Origin: *   (or a listed origin)
Access-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS
Access-Control-Allow-Headers: Content-Type, Authorization, X-Api-Key
```

Requests are HTTP/1.1. The server uses `select` on the listen sockets, then reads one request per connection and closes. There is no HTTP/2, no keep-alive reuse, and no request pipelining in this version.

Unknown paths return `404` with `text/plain`. Unsupported methods on a resource return `405`.

---

## Environment

| Variable | Meaning |
|----------|---------|
| `BURAAQ_DATABASE_URL` | libpq conninfo (preferred) |
| `DATABASE_URL` | Fallback conninfo |
| `BURAAQ_TLS_CERT` | PEM certificate path |
| `BURAAQ_TLS_KEY` | PEM private key path |
| `BURAAQ_API_KEY` | Override generated API key |
| `BURAAQ_API_LOCK` | `1` — require the key on all `/api/*` except health |
| `BURAAQ_CORS_ORIGIN` | `*` or comma-separated origins |
| `BURAAQ_HTTP_PORT` | Cleartext port (default 8080) |
| `BURAAQ_TLS_PORT` | TLS port (default 8443) |
| `USERNAME` | Windows SSPI user fallback inside libpq connect |

Working directory matters: `page` paths and default `cert.pem` / `key.pem` are relative to it. Run from the project root (`buraaq run` or `run.ps1`).

---

## C runtime symbols

Linked automatically. Do not call these from Buraaq unless you are extending the stdlib.

| Symbol | Role |
|--------|------|
| `buraaq_svc_page` | Register a file route |
| `buraaq_svc_api` | Register a REST table |
| `buraaq_svc_store` | Set conninfo |
| `buraaq_svc_key` | Set API key |
| `buraaq_svc_origin` | Set CORS allow-list |
| `buraaq_svc_run` | Bind and loop |
| `buraaq_http_listen` / `accept` / `reply` / `close` | Used by `run`, not by app code |
| `buraaq_pg_connect` / `exec` / `quote` | Postgres via libpq |

Headers: `stdlib/runtime/buraaq_std.h`.

---

## Security notes (this version)

- Table and column names are allowlisted identifiers; they are not taken from request URLs except the numeric id.
- Field values are escaped with libpq `PQescapeLiteral`.
- TLS is real OpenSSL when certs load; default play certs are self-signed.
- The API has no authentication. Put it behind your edge if it must not be public.
- CORS is origin-aware. Default allows `*`. Set `origin("https://app.example")` or `BURAAQ_CORS_ORIGIN` to a comma list. Unknown origins are not reflected.
- Every API gets a key (`.buraaq/api.key`, `BURAAQ_API_KEY`, or `key("…")`). Cross-origin and curl **writes** need `X-Api-Key` / `Authorization: Bearer`. Same-origin browser UI from this process still posts without a key. `BURAAQ_API_LOCK=1` requires the key on all `/api/*` except health.
- If `cert.pem` / `key.pem` are missing, Keel tries to generate a localhost TLS pair with `openssl`.
- Page files cannot contain `..`.
- Table and column names are allowlisted identifiers; they are not taken from request URLs except the numeric id.
- One accept loop, blocking read/write — high performance relative to interpreters, not a full HTTP/2 thread pool.

---

## Related modules

| Module | Use when |
|--------|----------|
| `std.keel` | You are **running** a service (`std.service` is the old name) |
| `std.http` | You are **calling** an HTTP/HTTPS URL |
| `std.db` | Extra SQL beyond `api(...)` |
| `std.fs` | Read/write files yourself |
| `std.json` | Parse JSON in application code |

---

## Play project

The Desktop play app `buraaq-play/webapi` is this module in a real tree:

```text
buraaq-play/webapi/
  src/main.bq          # page + api + run (Keel)
  public/index.html    # browser UI for /api/items
  cert.pem, key.pem    # localhost TLS
  run.ps1              # PATH, Postgres 5433, buraaq run
```

```powershell
cd ...\buraaq-play\webapi
.\run.ps1
```

Then open `https://127.0.0.1:8443` or `http://127.0.0.1:8080`.
