const UI_CACHE = "prieco-ui";
const RESULTS_CACHE = "prieco-results";
const MEDIA_CACHE = "prieco-media";
const LIMITS = { [RESULTS_CACHE]: 200, [MEDIA_CACHE]: 300 };
const CACHE_VER_URL = "/cache-ver";
const VERSION_KEY = "/__sw-version__";

const NETWORK_ONLY = [
  "/",
  "/sw.js",
  CACHE_VER_URL,
  "/settings_html",
  "/stats",
  "/pv",
  "/goggles",
  "/goggles/load",
  "/quick_tune_update",
];

function swLog(...args) {
  console.log("[SW]", ...args);
}

async function getStoredVersion() {
  try {
    const cache = await caches.open(UI_CACHE);
    const res = await cache.match(VERSION_KEY);
    return res ? await res.text() : null;
  } catch {
    return null;
  }
}

async function storeVersion(version) {
  const cache = await caches.open(UI_CACHE);
  await cache.put(
    VERSION_KEY,
    new Response(version, {
      headers: { "Content-Type": "text/plain" },
    }),
  );
}

async function notifyClients(message) {
  const clients = await self.clients.matchAll();
  clients.forEach((c) => c.postMessage(message));
}

async function trim(cacheName) {
  const cache = await caches.open(cacheName);
  const keys = await cache.keys();
  const extra = keys.slice(0, Math.max(0, keys.length - LIMITS[cacheName]));
  await Promise.all(extra.map((req) => cache.delete(req)));
}

async function checkVersion() {
  try {
    const res = await fetch(CACHE_VER_URL);
    if (!res.ok) return;
    const latest = (await res.text()).trim();
    const stored = await getStoredVersion();

    swLog("Version check — stored:", stored, "latest:", latest);

    if (stored === null) {
      await storeVersion(latest);
      return;
    }

    if (stored !== latest) {
      swLog("Version changed, invalidating cache...");
      await Promise.all([UI_CACHE, RESULTS_CACHE, MEDIA_CACHE].map((n) => caches.delete(n)));
      await storeVersion(latest);
      await notifyClients({ action: "cacheInvalidated", newVersion: latest });
    }
  } catch (err) {
    console.warn("[SW] Version check failed:", err);
  }
}

function cacheFor(url) {
  const path = url.pathname;
  if (NETWORK_ONLY.includes(path) || path.startsWith("/api")) return null;
  if (path === "/proxy") {
    const target = url.searchParams.get("u") || "";
    if (target.startsWith("https://duckduckgo.com/ac/")) return null;
    if (target.startsWith("https://cdn.searchexpander.com/")) return UI_CACHE;
    if (target.startsWith("https://api.searchexpander.com/")) return RESULTS_CACHE;
    return MEDIA_CACHE;
  }
  if (path.startsWith("/static/js/hbs/") && url.searchParams.get("q")) return RESULTS_CACHE;
  if (path.startsWith("/static/") || path === "/unduck.js") return UI_CACHE;
  return RESULTS_CACHE;
}

async function fetchShell(key) {
  const res = await fetch(key);
  if (!res.ok || res.redirected) return null;
  const text = await res.clone().text();
  return text.includes("load_html.js.hbs") ? { res, text } : null;
}

async function clearSearchData() {
  await Promise.all([caches.delete(RESULTS_CACHE), caches.delete(MEDIA_CACHE)]);
  const ui = await caches.open(UI_CACHE);
  for (const req of await ui.keys()) {
    if (new URL(req.url).pathname === "/search") await ui.delete(req);
  }
}

async function revalidateShell(key, cachedText, type) {
  const fresh = await fetchShell(key);
  if (fresh && fresh.text === cachedText) return;
  const cache = await caches.open(UI_CACHE);
  if (fresh) await cache.put(key, fresh.res);
  else await cache.delete(key);
  await notifyClients({ action: "shellUpdated", type });
}

async function handleSearch(event, type) {
  const key = new URL("/search?t=" + encodeURIComponent(type) + "&q=", self.location.origin).href;
  const cache = await caches.open(UI_CACHE);
  const cached = await cache.match(key);

  if (cached) {
    event.waitUntil(cached.clone().text().then((text) => revalidateShell(key, text, type)).catch(() => {}));
    return cached;
  }

  try {
    const fresh = await fetchShell(key);
    if (!fresh) return await fetch(event.request);
    await cache.put(key, fresh.res.clone());
    return fresh.res;
  } catch {
    return new Response("You're offline 😥", { status: 404 });
  }
}

async function handleCached(event, cacheName) {
  const cache = await caches.open(cacheName);
  const cached = await cache.match(event.request.url);
  if (cached) return cached;

  try {
    const res = await fetch(event.request);
    if (res.ok) {
      await cache.put(event.request.url, res.clone());
      if (LIMITS[cacheName]) event.waitUntil(trim(cacheName));
    }
    return res;
  } catch {
    return new Response("You're offline 😥", { status: 404 });
  }
}

self.addEventListener("install", (event) => {
  event.waitUntil(self.skipWaiting());
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    (async () => {
      await self.clients.claim();
      const keep = [UI_CACHE, RESULTS_CACHE, MEDIA_CACHE];
      const allCaches = await caches.keys();
      await Promise.all(
        allCaches.filter((name) => !keep.includes(name)).map((name) => caches.delete(name)),
      );
      swLog("Activated and ready.");
    })(),
  );
});

self.addEventListener("fetch", (event) => {
  const url = new URL(event.request.url);
  if (event.request.method !== "GET" || url.origin !== self.location.origin) return;

  const type = url.searchParams.get("t");
  if (url.pathname === "/search" && type && event.request.mode === "navigate") {
    event.respondWith(handleSearch(event, type));
    return;
  }

  const cacheName = cacheFor(url);
  if (cacheName) event.respondWith(handleCached(event, cacheName));
});

self.addEventListener("message", (event) => {
  if (!event.data?.action) return;

  if (event.data.action === "checkVersion") {
    event.waitUntil(checkVersion());
  }

  if (event.data.action === "clearCache") {
    event.waitUntil(
      clearSearchData().then(() => notifyClients({ action: "cacheCleared" })),
    );
  }
});
