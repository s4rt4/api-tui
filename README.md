# apitester

An interactive **TUI for HTTP API testing** — think Postman/Insomnia, but a single
cross-platform binary that lives in your terminal. Collections are plain TOML, so
they version-control cleanly alongside your code.

```
┌ ApiTester ─ Example API ─ env=default ───────────────────────────┐
│ requests          │ request                                       │
│ > Get Status      │ Method: GET   [m] cycle                       │
│   Echo JSON       │ URL    : /status/200                          │
│   Query Params    │ Query  : <empty>                              │
│                   │ response                                      │
│                   │ Status: 200 OK   Time: 142ms   Size: 0 B      │
└───────────────────┴───────────────────────────────────────────────┘
```

## Build

Requires Rust (edition 2021, MSRV 1.75+).

```sh
cargo build --release
# binary at target/release/apitester (.exe on Windows)
```

## Usage

### Interactive (TUI)

```sh
apitester collections/example.toml
```

Run without a file to start empty and add requests with `a`.

### Headless (scripting / CI)

Run a single named request, print the (pretty-printed) body to **stdout** and
diagnostics to **stderr**, then exit:

```sh
apitester --headless "Get Status" collections/example.toml
```

Exit codes:

| Code | Meaning                         |
|------|---------------------------------|
| 0    | response received, status < 400 |
| 1    | response received, status ≥ 400 |
| 2    | no collection file given        |
| 3    | request name not found          |
| 4    | transport error (DNS/TLS/refused/timeout) |

### CLI flags

| Flag | Description |
|------|-------------|
| `-e, --env <ENV>` | Environment for `{{var}}` interpolation (default `default`) |
| `-t, --timeout <SECS>` | Request timeout (default 30) |
| `-k, --insecure` | Skip TLS certificate verification |
| `--no-redirect` | Don't follow redirects |
| `--proxy <URL>` | HTTP/HTTPS proxy (otherwise `HTTP_PROXY`/`HTTPS_PROXY`/`NO_PROXY` env vars are honored) |
| `--no-color` | Disable ANSI colors |
| `--headless <NAME>` | Run one request non-interactively and exit |

## Keybindings

**Normal mode**

| Key | Action |
|-----|--------|
| `↑ ↓` / `j k` | Navigate list / cycle field / scroll response |
| `Tab` | Cycle panel (collection → request → response) |
| `s` | Send the selected request |
| `e` | Edit the active field (URL / body / headers / query) |
| `E` | Edit body in `$EDITOR` |
| `m` | Cycle method (GET → POST → PUT → PATCH → DELETE) |
| `a` | Add a request |
| `d` | Delete the selected request (confirm) |
| `w` | Save collection to its file |
| `o` | Export response body to `<request-name>.json\|txt` |
| `y` | Yank (copy) response body to the clipboard |
| `h` | Toggle response headers |
| `?` | Help |
| `q` / `Ctrl-C` | Quit (confirms if there are unsaved changes) |

**Insert mode** (field editor)

- `Esc` saves and exits for every field.
- For the URL, `Enter` also saves. For multi-line fields, `Enter` inserts a newline.
- **Headers** are edited as `Key: value`, one per line.
- **Query** params are edited as `key=value`, one per line.

## Collection format

```toml
name        = "Example API"
description = "Demo collection"
base_url    = "https://httpbin.org"

[env.default]
token = "Bearer dev-token"

[env.prod]
token = "Bearer prod-token"

[[requests]]
name   = "Get Status"
method = "GET"
url    = "/status/200"          # relative URLs join onto base_url

[[requests]]
name    = "Echo JSON"
method  = "POST"
url     = "/anything"
headers = { Authorization = "{{token}}", "Content-Type" = "application/json" }

[requests.body]
type    = "json"
content = """
{
  "hello": "world"
}
"""
```

- **Environments**: `[env.default]` is the base; `--env prod` overlays `[env.prod]`
  on top of it.
- **Interpolation**: `{{var}}` in URL, headers, query, and body is replaced from
  the resolved environment; an undefined variable is an error.

## Development

```sh
cargo test                       # unit + integration (wiremock)
cargo clippy --all-targets       # lint
cargo fmt                        # format
```

CI (GitHub Actions) runs `fmt --check`, `clippy -D warnings`, `test`, and a
release build on Ubuntu and Windows.

## License

MIT
