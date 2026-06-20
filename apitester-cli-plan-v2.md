# ApiTester CLI — Development Plan (Revised v2)

> Versi revisi dari `apitester-cli-plan.md` (Downloads). Perubahan utama: phasing MVP, arsitektur async yang konkret, multiline body editing, CLI flags, testing, security, dan risk register.

---

## 0. Ringkasan Perubahan dari v1

| Area | v1 | v2 |
|---|---|---|
| Phasing | Tidak ada — semua fitur dilist datar | MVP / v1.0 / Nice-to-have terpisah dengan acceptance criteria |
| Async architecture | Disinggung di "Catatan untuk Agent" | Dibuat eksplisit (event loop pattern, channel layout) |
| Body editing | Tidak dibahas — multi-line di TUI itu non-trivial | Pakai `tui-textarea` + opsi external `$EDITOR` |
| CLI args | Hanya `--collection` | Daftar lengkap dengan `clap` |
| Error handling | Matriks 6 baris | Tipe error sentral (`thiserror`) + propagasi ke status bar |
| Testing | Tidak disebut | Strategi unit + integration (`wiremock`) |
| Security | Tidak disebut | Token storage, env interpolation, `--insecure` UX |
| Cross-platform paths | `~/.config/...` (POSIX-only) | `directories` crate (XDG / Known Folders) |
| CI / release | Tidak disebut | GitHub Actions matrix build |
| Code samples | Ada `unwrap()` yang panic | Diganti dengan proper `Result` propagation |

---

## 1. Tujuan & Non-Tujuan

**Tujuan**
- TUI satu-binary untuk one-shot HTTP request
- Cross-platform (Linux + Windows) dari satu codebase
- Format collection human-readable (TOML), bisa di-version-control

**Non-Tujuan (eksplisit, supaya scope tidak melebar)**
- Bukan pengganti Postman/Insomnia — tidak ada GUI, tidak ada cloud sync
- Tidak ada scripting/test runner (chai-style assertions) di MVP
- Tidak ada WebSocket / gRPC / GraphQL playground khusus
- Tidak ada team collaboration / sharing built-in (cukup git)

---

## 2. Stack

| Komponen | Pilihan | Catatan |
|---|---|---|
| Language | Rust (edition 2021, MSRV 1.75+) | |
| TUI | **ratatui** `0.28` | v1 lama; cek crates.io saat scaffold |
| Terminal backend | **crossterm** `0.28` | Match dengan ratatui |
| Multi-line text input | **tui-textarea** `0.6` | **NEW** — body editor non-trivial tanpa ini |
| HTTP client | **reqwest** `0.12` (rustls) | |
| Async runtime | **tokio** `1` (`rt-multi-thread`, `macros`, `sync`) | |
| Args parser | **clap** `4` (derive) | **NEW** |
| Serialization | **serde** + **serde_json** + **toml** | |
| Highlighting | **syntect** `5` | Lazy-load syntax set |
| Error types | **thiserror** + **anyhow** | **NEW** — `thiserror` untuk lib, `anyhow` untuk main |
| Paths | **directories** `5` | **NEW** — XDG di Linux, Known Folders di Windows |
| Logging | **tracing** + **tracing-subscriber** | **NEW** — debug mode `RUST_LOG=debug` |
| Clipboard (opsional) | **arboard** `3` | Nice-to-have |

---

## 3. Struktur Project

```
apitester/
├── src/
│   ├── main.rs           # Entry: parse CLI, init tracing, run app
│   ├── lib.rs            # Re-export modul (memudahkan unit test)
│   ├── app/
│   │   ├── mod.rs        # State + reducer
│   │   ├── event.rs      # Event types (Key, Tick, RequestDone, ...)
│   │   └── input.rs      # Mode normal vs insert, key dispatch
│   ├── ui/
│   │   ├── mod.rs        # Top-level render
│   │   ├── collection_panel.rs
│   │   ├── request_panel.rs
│   │   ├── response_panel.rs
│   │   ├── status_bar.rs
│   │   └── help_modal.rs
│   ├── http/
│   │   ├── mod.rs        # Public API: send()
│   │   ├── client.rs     # reqwest Client builder (timeout, redirect, TLS)
│   │   └── response.rs   # Response struct + parsing helpers
│   ├── collection/
│   │   ├── mod.rs        # Load / save / validate
│   │   ├── model.rs      # Request, Collection structs
│   │   └── interpolate.rs # {{var}} substitution
│   ├── highlight.rs
│   ├── config.rs         # AppConfig (paths, theme, defaults)
│   └── error.rs          # ApiTesterError enum (thiserror)
├── tests/
│   ├── collection_load.rs
│   └── http_send.rs      # wiremock-based
├── collections/
│   └── example.toml
├── .github/workflows/
│   └── ci.yml            # build + test matrix (linux, windows)
├── Cargo.toml
├── Cargo.lock
└── README.md
```

Pemisahan `lib.rs` + `main.rs` membuat semua logic non-TUI bisa di-unit-test tanpa terminal.

---

## 4. Arsitektur Async (kunci yang hilang di v1)

**Problem:** `ratatui` sync; `reqwest` async. Naive call `block_on` di main loop = TUI freeze saat request berjalan.

**Pattern:**

```
┌─────────────── main thread (sync) ──────────────────┐
│  loop:                                              │
│    1. event = events_rx.recv_timeout(16ms)         │
│       (events_rx merge: keyboard + tick + result)  │
│    2. update(state, event)                         │
│    3. terminal.draw(|f| render(f, &state))         │
└─────────────────────────────────────────────────────┘
                       ▲
                       │ mpsc<AppEvent>
                       │
┌──── tokio runtime (background thread) ─────────────┐
│  task A: crossterm EventStream → AppEvent::Key     │
│  task B: tick interval 100ms → AppEvent::Tick      │
│  task C (spawned per request):                     │
│    http::send(...).await                           │
│    → AppEvent::RequestDone(Result<Response>)       │
└─────────────────────────────────────────────────────┘
```

`AppEvent` enum:
```rust
pub enum AppEvent {
    Key(crossterm::event::KeyEvent),
    Tick,
    RequestStarted,
    RequestDone(Result<Response, ApiTesterError>),
    Quit,
}
```

Kanal: `tokio::sync::mpsc::unbounded_channel::<AppEvent>()`. Main thread baca dengan `try_recv()` di dalam loop yang juga panggil `terminal.draw()` setiap tick.

Cancellation: simpan `tokio::task::JoinHandle` request aktif di state; tekan `Esc` saat loading → `handle.abort()`.

---

## 5. Format Collection (TOML, revisi)

Tambahan dari v1: collection-level metadata, environment block, query params terpisah, body content-type explicit.

```toml
# Collection-level metadata
name        = "Example API"
description = "Demo collection"
base_url    = "https://api.example.com"   # opsional, di-prefix ke url request

[env.default]
token = "Bearer dev-token"

[env.prod]
token = "Bearer prod-token"

[[requests]]
name   = "Get Users"
method = "GET"
url    = "/users"                         # diresolve ke {base_url}/users
headers = { Authorization = "{{token}}", Accept = "application/json" }
query   = { page = "1", limit = "20" }    # auto-encode ke URL

[[requests]]
name   = "Create User"
method = "POST"
url    = "/users"
headers = { Authorization = "{{token}}" }

[requests.body]
type = "json"                             # json | form | raw | multipart
content = '''
{ "name": "John", "email": "john@example.com" }
'''
```

**Aturan interpolasi:**
- `{{var}}` di-resolve dari `[env.<active>]`, fallback ke `[env.default]`, fallback ke env var OS (`$VAR` / `%VAR%`)
- Token sensitif sebaiknya pakai env var OS, bukan hardcoded di TOML
- Validasi: variabel undefined → tampilkan warning di status bar, jangan kirim request

**Schema validation:** parse → struct typed. Method invalid, URL kosong, atau type body tidak dikenal → error dengan baris TOML.

---

## 6. CLI Flags (clap)

```
apitester [OPTIONS] [COLLECTION]

Args:
  [COLLECTION]                      Path ke file collection .toml

Options:
  -e, --env <NAME>                  Pilih environment (default: "default")
  -t, --timeout <SECS>              Request timeout (default: 30)
  -k, --insecure                    Skip TLS verification (warning di status bar)
      --no-redirect                 Disable follow redirects
      --proxy <URL>                 HTTP/HTTPS proxy
      --no-color                    Disable ANSI colors
      --config <PATH>               Path ke config file custom
      --headless <NAME>             Run request `NAME` non-interactive, print response, exit
  -h, --help
  -V, --version
```

`--headless` penting: bikin tool ini juga berguna di shell script / CI tanpa TUI.

---

## 7. Layout TUI (revisi)

Perbedaan dari v1: tambah **status bar** (1 baris di atas help bar) untuk pesan error/info, dan badge environment aktif di header.

```
┌─────────────────────────────────────────────────────────────────┐
│ ApiTester  [example.toml]  env=default               ● dirty    │ <- header
├──────────────────────┬──────────────────────────────────────────┤
│ collections          │ request                                  │
│ ▶ Get Users          │ Method  : [GET ▼]                        │
│   Create User        │ URL     : /users                         │
│   ...                │ Query   : page=1&limit=20                │
│                      │ Headers : Authorization: {{token}}       │
│                      │ Body    : <empty>                        │
│                      ├──────────────────────────────────────────┤
│                      │ response                                 │
│                      │ Status: 200 OK   Time: 142ms   Size: 1.2KB│
│                      │ {                                        │
│                      │   "users": [...]                         │
│                      │ }                                        │
├──────────────────────┴──────────────────────────────────────────┤
│ ⚠ Variable {{missing}} undefined                                │ <- status bar
│ [↑↓] nav [Enter] select [s] send [e] edit [?] help [q] quit     │ <- key hints
└─────────────────────────────────────────────────────────────────┘
```

**Modal**: `?` buka help screen full-screen dengan tabel keybinding lengkap.

---

## 8. Keybindings

**Mode normal** (Vim-flavored, lowercase = single key):

| Key | Action |
|---|---|
| `↑` `↓` / `j` `k` | Navigate list / scroll response |
| `Tab` / `Shift-Tab` | Cycle panel |
| `Enter` | Select request → load ke editor |
| `s` | Send request aktif |
| `e` | Enter insert mode untuk field aktif |
| `E` | Buka body di `$EDITOR` (vim/nano/notepad) |
| `a` | Add request baru |
| `d` | Delete request (konfirmasi y/n) |
| `c` | Open collection (file picker modal) |
| `w` | Write/save collection |
| `h` | Toggle response headers |
| `y` | Yank response body ke clipboard |
| `/` | Search di collection list |
| `?` | Help modal |
| `Esc` | Abort: cancel in-flight request, exit insert mode, close modal |
| `q` / `Ctrl-C` | Quit (prompt jika dirty) |

**Mode insert**: `Esc` untuk keluar; key lain langsung edit. Untuk multi-line body, `tui-textarea` handle Enter, arrow, paste.

---

## 9. Fitur — Phased

### MVP (P0 — wajib untuk v0.1)
Acceptance: bisa load collection → pilih request → kirim → lihat response.
- [ ] Parse TOML → struct typed dengan error message berbaris
- [ ] Render 3-panel layout
- [ ] Navigasi list dengan ↑↓
- [ ] Send request async tanpa freeze TUI
- [ ] Tampilkan status, time, body
- [ ] Method GET/POST/PUT/PATCH/DELETE
- [ ] Custom headers (single-line)
- [ ] Raw JSON body
- [ ] CLI flag `--collection`, `--timeout`, `--insecure`
- [ ] Keybinding subset: navigate, send, quit

### v1.0 (P1)
- [ ] Edit field inline (insert mode)
- [ ] Save collection (`w`)
- [ ] Add/delete request (`a` / `d`)
- [ ] Multi-line body editor (`tui-textarea`)
- [ ] External editor `$EDITOR` untuk body
- [ ] Syntax highlight JSON/XML
- [ ] Response headers toggle
- [ ] Status bar dengan error/info
- [ ] Help modal
- [ ] Env interpolation `{{var}}` dengan `--env` flag
- [ ] Headless mode (`--headless NAME`)

### Nice-to-Have (P2)
- [ ] Multiple collection tabs
- [ ] Request history (last N, persistent)
- [ ] Body search / filter
- [ ] Yank ke clipboard
- [ ] Export response ke file
- [ ] Form-data / multipart body
- [ ] Auth helpers (Basic, Bearer, API Key) — UI khusus
- [ ] Cookie jar persistent
- [ ] Theme (light/dark)
- [ ] Proxy from env (`HTTP_PROXY`, `HTTPS_PROXY`)

---

## 10. Code Skeletons (revisi)

### `error.rs`
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiTesterError {
    #[error("collection file not found: {0}")]
    CollectionNotFound(std::path::PathBuf),

    #[error("invalid TOML at line {line}: {msg}")]
    TomlParse { line: usize, msg: String },

    #[error("invalid HTTP method: {0}")]
    InvalidMethod(String),

    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    #[error("undefined variable: {{{0}}}")]
    UndefinedVar(String),

    #[error("request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
```

### `http/mod.rs` — fix `unwrap()` panic
```rust
use std::time::{Duration, Instant};
use reqwest::Method;
use crate::error::ApiTesterError;

pub struct Response {
    pub status: u16,
    pub elapsed: Duration,
    pub headers: reqwest::header::HeaderMap,
    pub body: String,
}

pub struct SendOpts {
    pub timeout: Duration,
    pub insecure: bool,
    pub follow_redirects: bool,
}

pub async fn send(
    method: &str,
    url: &str,
    headers: &[(String, String)],
    body: Option<&str>,
    opts: &SendOpts,
) -> Result<Response, ApiTesterError> {
    let method = Method::from_bytes(method.as_bytes())
        .map_err(|_| ApiTesterError::InvalidMethod(method.into()))?;

    let client = reqwest::Client::builder()
        .timeout(opts.timeout)
        .danger_accept_invalid_certs(opts.insecure)
        .redirect(if opts.follow_redirects {
            reqwest::redirect::Policy::default()
        } else {
            reqwest::redirect::Policy::none()
        })
        .build()?;

    let mut req = client.request(method, url);
    for (k, v) in headers {
        req = req.header(k, v);
    }
    if let Some(b) = body {
        req = req.body(b.to_owned());
    }

    let start = Instant::now();
    let res = req.send().await?;
    let elapsed = start.elapsed();
    let status = res.status().as_u16();
    let headers = res.headers().clone();
    let body = res.text().await?;

    Ok(Response { status, elapsed, headers, body })
}
```

### `app/mod.rs` — state lebih lengkap
```rust
pub enum ActivePanel { CollectionList, RequestEditor, ResponseViewer }
pub enum InputMode { Normal, Insert }
pub enum RequestField { Method, Url, Query, Headers, Body }

pub struct App {
    pub collection_path: Option<std::path::PathBuf>,
    pub collection: Collection,
    pub selected: usize,
    pub active_panel: ActivePanel,
    pub active_field: RequestField,
    pub input_mode: InputMode,
    pub response: Option<Response>,
    pub response_scroll: u16,
    pub show_response_headers: bool,
    pub status_message: Option<(StatusKind, String)>,
    pub is_loading: bool,
    pub in_flight: Option<tokio::task::JoinHandle<()>>,
    pub dirty: bool,
    pub env: String,
    pub help_open: bool,
}

pub enum StatusKind { Info, Warn, Error }
```

---

## 11. Testing Strategy (hilang di v1)

| Layer | Tool | Yang ditest |
|---|---|---|
| Unit | `#[test]` standar | TOML parse, interpolasi `{{var}}`, URL build, state reducer |
| Integration HTTP | `wiremock` | `http::send()` real-roundtrip ke mock server |
| TUI snapshot | `insta` + ratatui `TestBackend` | Render output untuk state tertentu |
| Headless e2e | `assert_cmd` | `apitester --headless ...` exit code & stdout |

CI matrix:
```yaml
# .github/workflows/ci.yml (sketsa)
jobs:
  test:
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --check
      - run: cargo clippy -- -D warnings
      - run: cargo test --all-features
```

---

## 12. Cross-Platform Notes (revisi)

| Aspek | Linux | Windows | Penanganan |
|---|---|---|---|
| Config dir | `~/.config/apitester` | `%APPDATA%\apitester` | `directories::ProjectDirs` |
| Collection default | `./collections/` | `./collections/` | Sama, fallback ke config dir |
| Line ending | `\n` | `\r\n` di file output | Tulis `\n` di TOML; biarkan reqwest handle body |
| Terminal | xterm/gnome-terminal/alacritty | Windows Terminal (recommend) / cmd / PowerShell | Test di **Windows Terminal**; cmd.exe legacy mungkin glitchy |
| `$EDITOR` | `$EDITOR` env var | `%EDITOR%`, fallback `notepad` | Resolve dengan crate `edit` |
| Clipboard | Wayland/X11 | Win32 API | `arboard` handle keduanya |
| Cleanup terminal | `disable_raw_mode()` di `Drop` | Sama + `LeaveAlternateScreen` | Pasang panic hook supaya tidak corrupt terminal saat panic |

**Panic hook wajib**:
```rust
fn install_panic_hook() {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
        hook(info);
    }));
}
```

---

## 13. Build & Distribusi

| Target | Command |
|---|---|
| Linux native | `cargo build --release` |
| Linux MUSL (portable) | `cargo build --release --target x86_64-unknown-linux-musl` |
| Windows native | `cargo build --release` (di Windows) |
| Windows cross dari Linux | `cargo build --release --target x86_64-pc-windows-gnu` (perlu `mingw-w64`) |

**Catatan v1 yang salah:** `rustup target add x86_64-pc-windows-gnu` saja **tidak cukup**. Linker MinGW harus terinstall di host Linux (`apt install mingw-w64`).

**Release otomatis** (opsional, tapi recommended): tag `v*` → GitHub Action build matrix → upload binary ke Releases dengan SHA256.

---

## 14. Cargo.toml

```toml
[package]
name = "apitester"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"

[dependencies]
ratatui      = "0.28"
crossterm    = "0.28"
tui-textarea = "0.6"
reqwest      = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }
tokio        = { version = "1", features = ["rt-multi-thread", "macros", "sync", "time"] }
clap         = { version = "4", features = ["derive"] }
serde        = { version = "1", features = ["derive"] }
serde_json   = "1"
toml         = "0.8"
syntect      = { version = "5", default-features = false, features = ["default-fancy"] }
thiserror    = "1"
anyhow       = "1"
directories  = "5"
tracing      = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[dev-dependencies]
wiremock     = "0.6"
insta        = "1"
assert_cmd   = "2"

[profile.release]
lto = true
codegen-units = 1
strip = true
```

`reqwest` dengan `default-features = false` + `rustls-tls` → **tidak butuh OpenSSL** di host build → cross-compile jauh lebih mulus.

---

## 15. Error Handling Matrix (revisi)

| Kondisi | Source | UI Behavior |
|---|---|---|
| Collection tidak ada | `Io(NotFound)` | Mulai collection kosong, status: "starting empty — press `a` to add" |
| TOML malformed | `TomlParse{line, msg}` | Status bar merah dengan baris error; collection tidak di-load |
| Method invalid | `InvalidMethod` | Highlight field method merah, blok send |
| URL invalid | `InvalidUrl` | Highlight field URL merah, blok send |
| Variable undefined | `UndefinedVar` | Status warn kuning; user pilih: send anyway / cancel |
| Timeout | `reqwest::Error::is_timeout()` | Response panel: "⏱ Timeout after Ns" |
| TLS error | `reqwest::Error` cert | "🔒 TLS error — coba `--insecure` jika trusted" |
| DNS / connect | `reqwest::Error::is_connect()` | "🌐 Cannot connect: <host>" |
| Body too large untuk highlight | `body.len() > 100KB` | Render plain text, status info: "highlight skipped: large body" |
| Panic | panic hook | Restore terminal, print backtrace ke stderr |

---

## 16. Security Considerations (NEW)

- **Token storage:** TOML plaintext = OK untuk dev, **tidak OK** untuk prod token. Solusi MVP: env var interpolation. Solusi v1.0+: integrasi OS keyring (`keyring` crate) — opsional.
- **`.gitignore`:** README harus mention agar user tambahkan `collections/*.private.toml` atau pakai `[env.*]` block dengan placeholder.
- **`--insecure` UX:** wajib tampilkan banner merah persistent di header saat aktif. Jangan diam-diam.
- **Redirect:** default follow, tapi tidak forward `Authorization` header ke origin berbeda (reqwest default behavior sudah aman, dokumentasikan).
- **Response body sanitization:** jangan render escape sequence dari body sebagai control char ke terminal — sanitize sebelum render (cegah ANSI injection dari malicious server).

---

## 17. Performance Considerations (NEW)

- **syntect startup cost:** loading default `SyntaxSet` ~50–100ms. Lazy-load di task background saat startup, jangan blokir first frame.
- **Response besar:** stream ke `String` OK untuk <10MB. Untuk >10MB, render hanya first 1MB + tombol "save full to file".
- **Render frequency:** target 60fps overkill; 30fps (33ms tick) cukup untuk TUI dan hemat CPU.
- **Allocation:** hindari `String::clone` di hot path render — pakai `&str` slices ke state.

---

## 18. Risk Register (NEW)

| Risiko | Likelihood | Impact | Mitigasi |
|---|---|---|---|
| Multi-line body editor di TUI sulit | High | Med | Pakai `tui-textarea` (proven), plus escape hatch ke `$EDITOR` |
| Async + ratatui integration bug (race, deadlock) | Med | High | Pattern channel single-direction; cancellation via `JoinHandle::abort` |
| Windows terminal rendering glitch (cmd.exe) | Med | Low | Dokumentasikan: rekomendasi Windows Terminal; tidak invest fix cmd.exe |
| TOML format ambigu untuk user | Med | Med | Skema typed + error berbaris + contoh lengkap di `collections/example.toml` |
| Cross-compile gagal karena OpenSSL | Low (pakai rustls) | Med | Fitur `rustls-tls` di Cargo.toml — sudah dimitigasi |
| Scope creep ke "Postman killer" | High | High | Non-tujuan eksplisit di Section 1; phasing P0/P1/P2 ditegakkan |
| User menyimpan token plaintext di repo public | Med | High | Dokumentasi `.gitignore`; warning di `--insecure`; prefer env var |

---

## 19. Roadmap (estimasi)

Asumsi solo developer, ~10 jam/minggu:

| Phase | Scope | Estimasi |
|---|---|---|
| M0 | Scaffold, Cargo.toml, error.rs, CI hijau | 1 minggu |
| M1 | TOML load + read-only collection viewer (panel kiri saja) | 1 minggu |
| M2 | Async send + response render (MVP done) | 2 minggu |
| M3 | Edit/save + multi-line body + help modal | 2 minggu |
| M4 | Env interpolation + headless mode + syntax highlight | 1–2 minggu |
| M5 | Polish, release v1.0, build artifacts | 1 minggu |

Total ke v1.0: **~8–9 minggu**.

---

## 20. Open Questions untuk Owner

Sebelum mulai coding, tolong konfirmasi:

1. **Target user**: developer pribadi, atau internal tim? (Mempengaruhi prioritas multi-collection & sharing.)
2. **Auth flow**: cukup raw header, atau perlu helper khusus OAuth2 / API Key? (P2 vs scope creep.)
3. **Body editing**: `tui-textarea` sudah cukup, atau wajib invoke `$EDITOR` dari awal? (Mempengaruhi MVP scope.)
4. **Distribusi**: cukup `cargo install` + binary di Releases, atau perlu paket distro (`.deb`, Chocolatey, Homebrew)?
5. **Headless mode**: prioritas tinggi (CI use case) atau opsional?
6. **Telemetry / crash report**: tidak ada (privacy-first), atau opt-in?

Jawaban ke pertanyaan ini akan menentukan cut MVP yang final.
