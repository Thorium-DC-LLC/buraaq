# Create a CRUD API and run the server

Stack names: [STACK.md](STACK.md). Deploy: [SHIP.md](SHIP.md).

You do **not** write sockets, TLS handshakes, SQL, or route tables. You declare a page and a resource, then `run()`.

---

## 1. Project

```powershell
buraaq new notes
cd notes
```

That creates a **Keel** API (`std.keel`, `public/index.html`, tests). Then:

```powershell
$env:BURAAQ_DATABASE_URL = "host=127.0.0.1 port=5433 dbname=buraaq_play user=postgres"
buraaq up
```

---

## 2. The server program

`buraaq new` already wrote this. You only change the resource name if you want:

```buraaq
module main

use std.keel.{page, api, run}

fn main() {
    page("/", "public/index.html")
    api("items", "title, body")
    run()
}
```

What that means:

| Line | Effect |
|------|--------|
| `page("/", "public/index.html")` | Browser UI at `/` |
| `api("items", "title, body")` | Postgres table `items` + REST at `/api/items` |
| `run()` | TLS on **8443**, HTTP on **8080**, forever |

API routes you get for free:

| Method | Path |
|--------|------|
| `GET` | `/api/health` |
| `GET` `POST` | `/api/items` |
| `GET` `PUT` `DELETE` | `/api/items/:id` |

JSON body for create/update:

```json
{"title":"First note","body":"Hello from Buraaq"}
```

---

## 3. A page to browse

Create `public/index.html`. Keep HTML in a **file**, not in a Buraaq `"..."` string (`{` starts interpolation).

Minimal page:

```html
<!DOCTYPE html>
<html>
<head><meta charset="utf-8"><title>Notes</title></head>
<body>
  <h1>Notes</h1>
  <pre id="out">loading</pre>
  <script>
    fetch("/api/items")
      .then(r => r.json())
      .then(j => { document.getElementById("out").textContent = JSON.stringify(j, null, 2); });
  </script>
</body>
</html>
```

A fuller UI (create / update / delete buttons) lives in the play app: `buraaq-play/webapi/public/index.html`.

---

## 4. TLS (default)

From the project root:

```powershell
& "C:\Program Files\Git\usr\bin\openssl.exe" req -x509 -newkey rsa:2048 -keyout key.pem -out cert.pem -days 365 -nodes -subj "/CN=localhost"
```

`run()` loads `cert.pem` and `key.pem` in the working directory, or `BURAAQ_TLS_CERT` / `BURAAQ_TLS_KEY`.

Without certs, HTTP still binds on 8080 and TLS is skipped.

---

## 5. Postgres

`api(...)` needs a database. Set a libpq URL, then make sure the database exists:

```powershell
$env:BURAAQ_DATABASE_URL = "host=127.0.0.1 port=5432 dbname=buraaq_play user=postgres password=YOUR_PASSWORD"
```

Create the database once (psql):

```sql
CREATE DATABASE buraaq_play;
```

The table itself is created on startup (`CREATE TABLE IF NOT EXISTS items ...`). You only create the **database**.

On this machine a local play cluster can run on port **5433** without a password; `buraaq-play/webapi/run.ps1` sets that URL for you.

---

## 6. Run the server

From the project directory (so `public/` and `cert.pem` resolve):

```powershell
$env:PATH = "C:\Users\w3asi\bin;C:\Program Files\PostgreSQL\17\bin;" + $env:PATH
buraaq run
```

You should see:

```text
postgres connected
https://127.0.0.1:8443
http://127.0.0.1:8080
service ready
```

Open [http://127.0.0.1:8080](http://127.0.0.1:8080) or [https://127.0.0.1:8443](https://127.0.0.1:8443) (accept the self-signed cert).

Play app shortcut:

```powershell
cd C:\Users\w3asi\Desktop\buraaq-play\webapi
.\run.ps1
```

---

## 7. Exercise CRUD

Write JSON to a file. PowerShell strips quotes if you pass JSON on the command line.

```powershell
[System.IO.File]::WriteAllText("create.json", '{"title":"First note","body":"Hello from Buraaq"}')
[System.IO.File]::WriteAllText("update.json", '{"title":"Updated","body":"Changed"}')

curl.exe -s http://127.0.0.1:8080/api/health
curl.exe -s http://127.0.0.1:8080/api/items --data-binary "@create.json" -H "Content-Type: application/json"
curl.exe -s http://127.0.0.1:8080/api/items
curl.exe -s http://127.0.0.1:8080/api/items/1
curl.exe -s http://127.0.0.1:8080/api/items/1 -X PUT --data-binary "@update.json" -H "Content-Type: application/json"
curl.exe -s http://127.0.0.1:8080/api/items/1 -X DELETE
curl.exe -sk https://127.0.0.1:8443/api/items
```

Responses look like:

```json
{"ok":true,"error":"","rows":[{"id":"1","title":"First note","body":"Hello from Buraaq","created_at":"..."}]}
```

---

## 8. Another resource

Add a second line. No new server code.

```buraaq
api("items", "title, body")
api("users", "name, email")
```

That is `/api/users` with the same REST shape.

---

## What not to do

- Do not write `listen` / `accept` / `while true` for this. That is inside `run()`.
- Do not put HTML or JSON with `{` inside Buraaq string literals.
- Do not concatenate user input into SQL. `api(...)` quotes values for you.
- `std.http.get` is for **calling** other APIs, not for serving.

More detail: [SERVICE.md](SERVICE.md).
