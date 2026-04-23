# AQI — Setup Guide

## Prerequisites

### 1. Install Rust
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

### 2. Add the WASM compile target (needed for the frontend)
```bash
rustup target add wasm32-unknown-unknown
```

### 3. Install Trunk (WASM bundler / dev server for the frontend)
```bash
cargo install trunk
```

### 4. Get a free WAQI API token
Register at: https://aqicn.org/data-platform/token/
A "demo" token works for testing but is rate-limited to a single test station.

---

## Configuration

Copy `.env.example` to `backend/.env` and fill in your token:
```bash
cp .env.example backend/.env
# then edit backend/.env:
WAQI_API_TOKEN=your_token_here
# Optional:
# CORS_ALLOWED_ORIGINS=http://localhost:8080,http://127.0.0.1:8080
# HOST=0.0.0.0
# PORT=3000
```

---

## Running in development (two terminals)

**Terminal 1 — backend (Axum API server on :3000)**
```bash
cd backend
cargo run
```

**Terminal 2 — frontend (Trunk dev server on :8080, proxies /api → :3000)**
```bash
cd frontend
trunk serve --open
```

Open http://localhost:8080 in your browser.
The browser will ask for location permission; allow it for automatic detection.

---

## Production build

```bash
# Build the frontend WASM bundle into frontend/dist/
cd frontend && trunk build --release

# Run the backend — it serves frontend/dist/ as static files
cd ../backend && cargo run --release
# App is now available at http://localhost:3000
```

---

## Project structure

```
air-quality-app/
├── Cargo.toml              # Workspace — ties backend + frontend together
├── backend/
│   ├── Cargo.toml
│   └── src/main.rs         # Axum server: /api/aqi/geo  /api/aqi/search
│                           # Also serves ../frontend/dist/ in production
└── frontend/
    ├── Cargo.toml
    ├── Trunk.toml          # Trunk config: proxies /api/* → backend in dev
    ├── index.html          # Shell HTML (Trunk injects WASM here)
    ├── style.css
    └── src/
        ├── lib.rs          # Leptos App component + geolocation logic
        ├── api.rs          # WAQI response types + async fetch helpers
        └── components/
            ├── aqi_card.rs     # Large AQI display + colour-coded scale bar
            ├── forecast.rs     # Daily forecast cards + tooltip detail
            ├── favorites.rs    # Saved locations dropdown + localStorage
            ├── pollutants.rs   # Per-pollutant grid + weather conditions
            └── search.rs       # Debounced search input + dropdown
```

---

## How it works (learning notes)

| Layer | Crate | Why |
|-------|-------|-----|
| Backend HTTP | **axum** | Tower-based, ergonomic, async-first |
| Async runtime | **tokio** | De-facto standard async runtime |
| Static files | **tower-http** `ServeDir` | Serves Trunk-built WASM assets |
| WASM framework | **leptos** (CSR mode) | React-like reactive UI, compiles to WASM |
| WASM bundler | **trunk** | Handles WASM compilation + asset pipeline |
| Browser HTTP | **gloo-net** | Thin wrapper around the browser Fetch API |
| DOM bindings | **web-sys** | Auto-generated bindings to every browser API |
| JS interop | **wasm-bindgen** | The bridge between Rust/WASM and JavaScript |
| Geolocation | **web-sys** `Geolocation` | Browser GPS/IP location API |

### Reactive data flow (frontend)
1. `App` holds a single `RwSignal<AppView>` — the entire UI state
2. On mount, `Effect::new` fires once → triggers `get_browser_location()` 
3. Location resolves → `fetch_aqi_by_geo` → `view_state` updated → Leptos re-renders
4. `SearchBar` debounces keystrokes (400 ms) then calls `fetch_aqi_search`
5. Selecting a result calls the `on_select` callback with coordinates → same flow

### Why the backend proxies the API
The WAQI token must not be embedded in WASM (it's visible to anyone who downloads the binary).
The backend keeps it in an environment variable and proxies all requests server-side.
