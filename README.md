# AQI — Global Air Quality

A full-stack web application for viewing real-time air quality anywhere in the world. Built entirely in Rust — backend, frontend, and everything in between.

![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)
![Leptos](https://img.shields.io/badge/Leptos-EF3939?style=flat)
![Axum](https://img.shields.io/badge/Axum-000000?style=flat)
![WebAssembly](https://img.shields.io/badge/WebAssembly-654FF0?style=flat&logo=webassembly&logoColor=white)

## Features

- **Automatic location detection** — loads air quality for your current location on first visit
- **Global search** — search any city or monitoring station in the world
- **Real-time AQI** — live data from the [World Air Quality Index](https://waqi.info) network
- **Full pollutant breakdown** — PM₂.₅, PM₁₀, O₃, NO₂, SO₂, CO with units
- **Weather conditions** — temperature, humidity, pressure, and wind where available
- **Color-coded scale** — EPA standard AQI categories from Good → Hazardous

## Tech Stack

| Layer | Technology | Purpose |
|---|---|---|
| Backend | [Axum](https://github.com/tokio-rs/axum) | Async HTTP server, API proxy |
| Async runtime | [Tokio](https://tokio.rs) | Powers the backend async I/O |
| Frontend | [Leptos](https://leptos.dev) (CSR) | Reactive UI compiled to WebAssembly |
| WASM bundler | [Trunk](https://trunkrs.dev) | Builds and serves the frontend |
| Browser HTTP | [gloo-net](https://github.com/rustwasm/gloo) | Fetch API wrapper for WASM |
| DOM bindings | [web-sys](https://rustwasm.github.io/wasm-bindgen/api/web_sys/) | Browser Geolocation API |
| Data source | [WAQI API](https://aqicn.org/api/) | Global air quality data |

## Prerequisites

```bash
# 1. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 2. Add the WebAssembly compile target
rustup target add wasm32-unknown-unknown

# 3. Install Trunk (frontend bundler)
cargo install trunk
```

## Setup

Get a free API token at **https://aqicn.org/data-platform/token/** and create your env file:

```bash
cp .env.example backend/.env
# Edit backend/.env and set your token:
# WAQI_API_TOKEN=your_token_here
```

> The backend keeps your token server-side so it is never exposed in the compiled WASM binary.

## Running locally

Open two terminals:

```bash
# Terminal 1 — backend API server (http://localhost:3000)
cd backend
cargo run

# Terminal 2 — frontend dev server (http://localhost:8080)
cd frontend
trunk serve --open
```

Trunk automatically proxies `/api/*` requests to the backend, so no CORS configuration is needed during development.

## Production build

```bash
# Compile frontend to frontend/dist/
cd frontend && trunk build --release

# Run backend — it serves frontend/dist/ as static files
cd ../backend && cargo run --release
# App is available at http://localhost:3000
```

## Project structure

```
air-quality-app/
├── Cargo.toml                  # Workspace — links backend + frontend
├── .env.example                # Copy to backend/.env and add your token
│
├── backend/
│   └── src/main.rs             # Axum server
│                               #   GET /api/aqi/geo?lat=&lng=
│                               #   GET /api/aqi/search?q=
│                               #   Serves frontend/dist/ in production
│
└── frontend/
    ├── Trunk.toml              # Dev proxy config (/api → :3000)
    ├── index.html              # Shell HTML (Trunk injects WASM here)
    ├── style.css               # All styles
    └── src/
        ├── lib.rs              # Root App component + geolocation logic
        ├── api.rs              # WAQI types + async fetch helpers
        └── components/
            ├── aqi_card.rs     # AQI gauge, category, colour scale bar
            ├── pollutants.rs   # Per-pollutant grid + weather row
            └── search.rs       # Debounced search input + dropdown
```

## AQI Scale

| Range | Category | Meaning |
|---|---|---|
| 0–50 | **Good** | Air quality is satisfactory |
| 51–100 | **Moderate** | Acceptable; some pollutants may affect sensitive people |
| 101–150 | **Unhealthy for Sensitive Groups** | General public unlikely to be affected |
| 151–200 | **Unhealthy** | Everyone may begin to experience effects |
| 201–300 | **Very Unhealthy** | Health alert for everyone |
| 301+ | **Hazardous** | Emergency conditions |

---

Built by [Jason Kopacz](https://www.linkedin.com/in/jasonkopacz/) · [jtkopacz@gmail.com](mailto:jtkopacz@gmail.com) · [GitHub](https://github.com/jasonkopacz)
