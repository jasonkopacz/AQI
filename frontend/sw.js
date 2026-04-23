// Service worker for AQI — Global Air Quality
// Strategy:
//   - App shell (HTML, WASM, CSS, JS): cache-first, update in background
//   - /api/* requests: network-first, fall back to cache (last known data)

const SHELL_CACHE = 'aqi-shell-v1';
const DATA_CACHE  = 'aqi-data-v1';
const DEBUG = true;

const SHELL_URLS = ['/', '/index.html'];

function log(...args) {
  if (DEBUG) {
    console.log('[sw]', ...args);
  }
}

function cacheResponse(cacheName, request, response) {
  if (!response || !response.ok) {
    return;
  }

  try {
    const responseClone = response.clone();
    caches.open(cacheName).then(cache => cache.put(request, responseClone)).catch(() => {});
  } catch (_) {
    log('Skipping cache write due to non-cloneable response', request.url);
  }
}

// ── Install: pre-cache the app shell ─────────────────────────────────────────
self.addEventListener('install', event => {
  log('Installing service worker');
  event.waitUntil(
    caches.open(SHELL_CACHE).then(cache => cache.addAll(SHELL_URLS))
  );
  self.skipWaiting();
});

// ── Activate: clean up old caches ────────────────────────────────────────────
self.addEventListener('activate', event => {
  log('Activating service worker');
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
          cacheResponse(DATA_CACHE, request, response);
          log('API network response', request.url, response.status);
          return response;
        })
        .catch(() => {
          log('API network failed, falling back to cache', request.url);
          return caches.match(request);
        })
    );
    return;
  }

  // Everything else: cache-first (app shell assets)
  event.respondWith(
    caches.match(request).then(cached => {
      if (cached) {
        // Refresh in background
        fetch(request).then(response => {
          cacheResponse(SHELL_CACHE, request, response);
        }).catch(() => {});
        return cached;
      }
      return fetch(request).then(response => {
        cacheResponse(SHELL_CACHE, request, response);
        return response;
      });
    })
  );
});
