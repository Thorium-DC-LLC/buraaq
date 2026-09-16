# Buraaq Standard Library

Version 1.0.0 · package `buraaq-std`

The Buraaq standard library follows a **small surface, one obvious way** design. For each common task there is typically a single canonical function or type — not dozens of overlapping APIs.

```
text = read("hello.txt")
write("hello.txt", "Hello")
response = get("https://example.com")
data = parse(response.body)
```

## Philosophy

| Principle | Implementation |
|-----------|----------------|
| One obvious way | `read(path)` not `FileOpenReadAll` / `slurp` / `load_text` |
| Zero GC | Ownership + `buraaq_std.c` explicit `malloc`/`free` |
| Low overhead | Thin FFI wrappers; stack paths where possible |
| C interop | `extern c { fn printf(...) }` + safe wrappers |
| Compiler ownership | Resources released at scope end / move |

## Module map

### `std.io`

Console output.

| API | Description |
|-----|-------------|
| `print(msg: text)` | Write text without newline |
| `println(msg: text)` | Write text with newline |
| `print_int(n: int)` | Print integer |

### `std.fs`

File system — **path-first** API.

| API | Description |
|-----|-------------|
| `read(path: text) -> text` | Read entire file |
| `write(path, contents) -> bool` | Write file |
| `exists(path) -> bool` | Check path |
| `open(path) -> File` | Open handle |
| `File.read(self) -> text` | Read via handle |
| `File.write(self, contents) -> bool` | Write via handle |

Types: `File`, `Path`

### `std.text`

| API | Description |
|-----|-------------|
| `len(s: text) -> int` | Byte length |
| `concat(a, b) -> text` | Allocate concatenation |
| `eq(a, b) -> bool` | Byte equality |
| `byte(s, i) -> int` | Byte at index, or `-1` |
| `slice(s, start, end) -> text` | Substring `[start, end)` |
| `String` | Owned string wrapper with `.len()` |

### `std.math`

Scalars. Arrays are `std.grid`.

| API | Description |
|-----|-------------|
| `sqrt`, `abs`, `abs_int`, `min`, `max` | Basics |
| `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2` | Trig |
| `sinh`, `cosh`, `tanh` | Hyperbolic |
| `exp`, `log`, `log10`, `log2`, `pow`, `hypot`, `cbrt` | Powers |
| `floor`, `ceil`, `trunc`, `round`, `fmod`, `copysign` | Rounding |
| `pi`, `euler`, `deg`, `rad`, `clamp`, `lerp`, `sign` | Constants and mix |

### `std.grid`

Numeric arrays — the Buraaq shape of NumPy. Handles are integers; `zeros(2, 2)` is the whole constructor.

```
g = zeros(2, 2)
put_at(g, 0, 0, 1.5)
println(at(g, 0, 0))
println(dot(row("1,2,3"), row("4,5,6")))
```

| API | Description |
|-----|-------------|
| `zeros` / `ones` / `fill` / `eye` / `linspace` / `row` | Build |
| `at` / `put_at` / `nrows` / `ncols` | Cells |
| `plus` / `times` / `scale` / `matmul` / `dot` / `transpose` | Linear algebra |
| `sum` / `mean` / `least` / `most` / `norm` | Reductions |
| `sin_all` / `cos_all` / `view` | Map and print |

### `std.hold`

Named columns of cargo — the Buraaq shape of Pandas. Stow rows; pick cells; keep matches.

```
h = hold("name,cents")
stow(h, "checking,1200")
println(col_mean(h, "cents"))
```

| API | Description |
|-----|-------------|
| `hold(cols)` / `from_csv` / `to_csv` | Create / files |
| `stow` / `rows` / `cols` / `pick` | Write and read |
| `keep` / `order` | Filter equal, sort |
| `col_mean` / `col_sum` / `hold_text` | Reduce / dump |

### `std.stream`

Live framed messages — the Buraaq name for WebSockets. Not a browser API.

```
stream(9420)
run_stream()
```

A client:

```
s = wire("ws://127.0.0.1:9420/")
say(s, "hello")
println(hear(s))
hangup(s)
```

| API | Description |
|-----|-------------|
| `stream(port)` | Bind |
| `run_stream()` | Accept forever and echo |
| `hail()` | Accept one client |
| `wire(url)` | Connect `ws://host:port/path` |
| `say` / `hear` / `hangup` | Text frames |

### `std.time`

| API | Description |
|-----|-------------|
| `now_ms() -> int` | Wall clock milliseconds |
| `now_sec() -> f64` | Monotonic seconds (QPC / `clock_gettime`) |
| `sleep(d: Duration)` | Block current thread |
| `ms(n)`, `sec(n) -> Duration` | Duration builders |

### `std.collections`

Submodules (one type each):

| Module | Type | Role |
|--------|------|------|
| `std.collections.list` | `List[T]` | Growable sequence |
| `std.collections.map` | `Map[K,V]` | Key/value map |
| `std.collections.set` | `Set[T]` | Unique values |
| `std.collections.buffer` | `Buffer` | Byte/text buffer |

### `std.net`

| API | Description |
|-----|-------------|
| `listen(port) -> TcpListener` | Bind TCP |
| `connect(host, port) -> TcpStream` | Connect TCP |
| `udp_bind(port) -> UdpSocket` | Bind UDP |

Types: `TcpListener`, `TcpStream`, `UdpSocket`

### `std.lumen`

Native HD windows — **the operator console**. Not a browser.

```
use std.lumen.{app, heading, note, field, button, bind, show}

fn main() {
    app("Harbor", 1280, 800)
    heading("Deploy jobs")
    field("title", "Title")
    button("create", "Create job")
    bind("http://127.0.0.1:8080", "jobs")
    show()
}
```

Full reference: [LUMEN.md](LUMEN.md).

| API | Description |
|-----|-------------|
| `app(title, w, h)` | Native window |
| `heading` / `note` | Title and supporting copy |
| `field(id, label)` | Text field |
| `button(id, label)` | Action |
| `bind(url, resource)` | Keel REST list + create + delete |
| `show()` | Event loop until the window closes |

### `std.keel`

APIs and TLS servers — **the hull of the app**. `std.service` is the same module under the old name.

```
use std.keel.{page, api, run}

fn main() {
    page("/", "public/index.html")
    api("items", "title, body")
    run()
}
```

TLS is the default (`8443`, or `443` when you pass that port). HTTP is bound beside it for local clients. Postgres, table schema, REST, CORS, and `/api/health` live in the runtime — not in application source.

Ship that binary with `buraaq pack` / `buraaq up` / `buraaq land`. Toolchain, not a std module. See [STACK.md](STACK.md).

Full reference: [SERVICE.md](SERVICE.md). Step-by-step: [CRUD_API.md](CRUD_API.md).

| API | Description |
|-----|-------------|
| `page(path, file)` | Serve a file at an exact path |
| `api(name, fields)` | Postgres table + REST at `/api/{name}` |
| `store(url)` | Optional database URL (else `BURAAQ_DATABASE_URL`) |
| `call(url) -> text` | Call another HTTP/HTTPS API |
| `run()` | Bind TLS (and HTTP) and serve forever |

`api("items", "title, body")` creates `items` and handles:

- `GET/POST /api/items`
- `GET/PUT/DELETE /api/items/:id`

### `std.db`

Postgres hatch when `api(...)` is not enough. `ok` is a keyword, so the live check is `connected()`.

| API | Description |
|-----|-------------|
| `connect(url) -> int` | libpq connect (`1` / `0`) |
| `connected() -> int` | `1` if a connection is open |
| `exec(sql) -> text` | Run SQL; JSON `{ok, error, rows}` |
| `quote(s) -> text` | `PQescapeLiteral` |
| `disconnect()` | Close |

Prefer `std.keel.api` for ordinary CRUD.

### `std.http`

Call other APIs. Servers belong in `std.keel`.

| API | Description |
|-----|-------------|
| `get(url) -> Response` | HTTP GET |
| `post(url, body) -> Response` | HTTP POST (v1: GET-shaped client) |

`Response { status, body }`

### `std.json`

| API | Description |
|-----|-------------|
| `parse(text) -> Value` | Parse JSON document |
| `stringify(v) -> text` | Serialize |
| `Value.field(key) -> text` | Extract string field (v1) |

### `std.process`

| API | Description |
|-----|-------------|
| `cmd(program) -> Command` | Build command |
| `Command.run() -> int` | Run and return exit code |

### `std.thread`

| API | Description |
|-----|-------------|
| `spawn_worker() -> JoinHandle` | Start background thread |
| `JoinHandle.join()` | Wait for completion |

Spawn blocks use language `spawn { ... }` syntax directly.

### `std.sync`

| API | Description |
|-----|-------------|
| `new_mutex() -> Mutex` | OS mutex |
| `Mutex.lock/unlock` | Exclusive access |
| `Channel[T]` | Typed message channel |
| `atomic(n) -> AtomicInt` | Atomic integer |
| `AtomicInt.load/store/fetch_add` | Atomic ops |

### `std.async`

Feature-gated async tasks (`buraaq.pkg` feature `async`).

| API | Description |
|-----|-------------|
| `Task` | Async task handle |
| `block_on(task)` | Run task to completion |

### `std.os`

| API | Description |
|-----|-------------|
| `getenv(name) -> text` | Environment variable |
| `args() -> int` | Argument count |
| `arg(i) -> text` | Argument by index |

### `std.crypto`

| API | Description |
|-----|-------------|
| `sha256(data: text) -> text` | SHA-256 hex digest |
| `sha256_bytes(data: bytes) -> text` | Hash byte buffer |

## C runtime (`runtime/buraaq_std.c`)

Native implementations linked automatically by `buraaq build`. No garbage collection.

| Symbol | Module |
|--------|--------|
| `buraaq_file_read/write/exists` | fs |
| `buraaq_text_concat/len` | text |
| `buraaq_math_*` | math |
| `buraaq_time_now_ms/sleep_ms` | time |
| `buraaq_os_getenv/argc/argv` | os |
| `buraaq_json_parse_string_field` | json |
| `buraaq_http_get_body` | http client |
| `buraaq_svc_page/api/store/run` | service |
| `buraaq_pg_connect/exec/quote` | db |
| `buraaq_crypto_sha256_hex` | crypto |
| `buraaq_mutex_*` | sync |
| `buraaq_process_exit_code` | process |

## FFI example

```buraaq
extern c {
    fn printf(format: c.text) -> c.int
}

fn main() {
    unsafe {
        printf("Hello\n".to_c())
    }
}
```

Safe wrappers live in std modules; raw `extern c` calls stay in `unsafe` blocks.

## Allocation strategy

- **Heap**: `text` concat, file read, JSON field extract — one allocation per call
- **Stack**: primitives, struct literals, mutex handles (v1 stub)
- **Benchmark**: `cargo bench` in `stdlib/` measures concat allocation rate

## Testing

```bash
cd stdlib
cargo test     # 19 module parse tests + C runtime assertions
cargo bench    # alloc benchmark
```

Examples live in `stdlib/examples/` — one per major module group.

## Compiler integration status

| Feature | Status |
|---------|--------|
| Stdlib source modules | ✅ |
| C runtime linked on build | ✅ |
| `use std.*` import resolution | 🔄 in progress |
| User `extern c` codegen | 🔄 partial |
| Multi-module package build | 🔄 planned |

Imports parse today; full cross-module builds will land with the package resolver.
