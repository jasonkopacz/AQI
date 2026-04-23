// Service worker for AQI — Global Air Quality
// Strategy:
//   - App shell (HTML, WASM, CSS, JS): cache-first, update in background
//   - /api/* requests: network-first, fall back to cache (last known data)

const SHELL_CACHE = 'aqi-shell-v1';
const DATA_CACHE  = 'aqi-data-v1';

const SHELL_URLS = ['/', '/index.html'];

// ── Install: pre-cache the app shell ─────────────────────────────────────────
self.addEventListener('install', event => {
  event.waitUntil(
    caches.open(SHELL_CACHE).then(cache => cache.addAll(SHELL_URLS))
  );
  self.skipWaiting();
});

// ── Activate: clean up old caches ────────────────────────────────────────────
self.addEventListener('activate', event => {
  event.waitUntil(
    caches.keys().then(keys =>
      Promise.all(
        keys
          .filter(k => k !== SHELL_CACHE && k !== DATA_CACHE)
          .map(k => caches.delete(k))
      )
    )
  );
  self.clients.claim();
});

// ── Fetch: route requests ─────────────────────────────────────────────────────
self.addEventListener('fetch', event => {
  const { request } = event;
  const url = new URL(request.url);

  // API calls: network-first, stale fallback
  if (url.pathname.startsWith('/api/')) {
    if (request.method !== 'GET') {
      return;
    }
    event.respondWith(
      fetch(request)
        .then(response => {
          if (response.ok) {
            const clone = response.clone();
            caches.open(DATA_CACHE).then(cache => cache.put(request, clone));
          }
          return response;
        })
        .catch(() => caches.match(request))
    );
    return;
  }

  // Everything else: cache-first (app shell assets)
  event.respondWith(
    caches.match(request).then(cached => {
      if (cached) {
        // Refresh in background
        fetch(request).then(response => {
          if (response.ok) {
            caches.open(SHELL_CACHE).then(cache => cache.put(request, response));
          }
        }).catch(() => {});
        return cached;
      }
      return fetch(request).then(response => {
        if (response.ok) {
          caches.open(SHELL_CACHE).then(cache => cache.put(request, response.clone()));
        }
        return response;
      });
    })
  );
});
