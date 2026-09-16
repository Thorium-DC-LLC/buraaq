# std.lumen — native HD UI

**Lumen** is the one obvious way to draw a Buraaq desktop window. Not HTML, not Electron, not a guest OS. A native window, DPI-aware, ClearType text, talking to a **Keel** API when you want data.

```buraaq
use std.lumen.{app, heading, note, field, button, bind, show}

fn main() {
    app("Harbor", 1280, 800)
    heading("Deploy jobs")
    note("Native Lumen console for a Keel API")
    field("title", "Title")
    field("body", "Details")
    button("create", "Create job")
    bind("http://127.0.0.1:8080", "jobs")
    show()
}
```

Stack: [STACK.md](STACK.md). Keel APIs: [SERVICE.md](SERVICE.md).

---

## API

Call these before `show()`. `show()` opens the window and does not return until it closes.

| Function | Role |
|----------|------|
| `app(title, width, height)` | Window title and size (logical pixels) |
| `heading(text)` | Large title |
| `note(text)` | Supporting line |
| `field(id, label)` | Labeled text field |
| `button(id, label)` | Action |
| `bind(url, resource)` | Keel REST at `{url}/api/{resource}` |
| `keep(path)` | Local JSON file instead of a server |
| `show()` | Paint, input, loop |
| `value(id)` | Field text (hatch) |
| `clicked(id)` | Last button id match (hatch) |

`bind` lists rows as cards (title, body, delete) and posts `title` + `body` field ids when you click a button named `create` or `save`, or press Enter. F5 refreshes.

`keep("desk.json")` is the local file instead of a Keel API — a small Windows app with no server.

Limits in this runtime: 48 widgets, 64 rows, ASCII/UTF-8 field input, one window.

---

## Look

Flat, teal, no drop shadows. Segoe UI with ClearType. Per-monitor DPI. Double-buffered paint.

This is a first native surface — not a full design-system kit (no charts, no nav split, no dark theme yet). It is meant to replace “open a browser and hope” for local operator consoles.

---

## Platforms

| OS | Status |
|----|--------|
| Windows | Native Win32 + GDI, HD/DPI |
| Linux / macOS | Compiles; `show()` reports that the blit backend is not wired yet |

Same Buraaq source. The C runtime picks the windowing path.

---

## With Keel

Terminal 1: `buraaq up` on the API project.  
Terminal 2: `buraaq run` on the Lumen project.

They share `/api/{resource}`. CORS does not matter — Lumen is not a browser.

Create: `buraaq new desk --ui`
