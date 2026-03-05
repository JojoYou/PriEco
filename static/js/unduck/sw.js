const CACHE_NAME = "prieco-cache";
const CACHE_VER_URL = "/cache-ver";
const MAX_CACHE_ITEMS = 50;

const STATIC_ASSETS = ["/", "/bangs.js"];

function swLog(...args) {
    console.log("[SW]", ...args);
}

const VERSION_KEY = "__cache-version__";

async function getStoredVersion() {
    try {
        const cache = await caches.open(CACHE_NAME);
        const res = await cache.match(VERSION_KEY);
        return res ? await res.text() : null;
    } catch {
        return null;
    }
}

async function storeVersion(version) {
    const cache = await caches.open(CACHE_NAME);
    await cache.put(
        VERSION_KEY,
        new Response(version, {
            headers: { "Content-Type": "text/plain" },
        }),
    );
}

const METADATA_KEY = "__cache-metadata__";

async function getMetadata() {
    try {
        const cache = await caches.open(CACHE_NAME);
        const res = await cache.match(METADATA_KEY);
        return res ? await res.json() : { urls: [] };
    } catch {
        return { urls: [] };
    }
}

async function saveMetadata(metadata) {
    const cache = await caches.open(CACHE_NAME);
    await cache.put(
        METADATA_KEY,
        new Response(JSON.stringify(metadata), {
            headers: { "Content-Type": "application/json" },
        }),
    );
}

async function trackAndEvict(url) {
    const reserved = [VERSION_KEY, METADATA_KEY, ...STATIC_ASSETS];
    if (reserved.includes(url)) return;

    const cache = await caches.open(CACHE_NAME);
    const metadata = await getMetadata();

    const idx = metadata.urls.indexOf(url);
    if (idx > -1) metadata.urls.splice(idx, 1);
    metadata.urls.push(url); // most recent at end

    while (metadata.urls.length > MAX_CACHE_ITEMS) {
        const evicted = metadata.urls.shift();
        await cache.delete(evicted);
        swLog("Evicted:", evicted);
    }

    await saveMetadata(metadata);
}

// Cache wipe + rebuild
async function rebuildCache() {
    await caches.delete(CACHE_NAME);
    swLog("Cache wiped, rebuilding static assets...");
    const cache = await caches.open(CACHE_NAME);
    await Promise.all(
        STATIC_ASSETS.map(async (url) => {
            try {
                const res = await fetch(url);
                if (res.ok) await cache.put(url, res);
            } catch (err) {
                console.warn("[SW] Failed to cache asset:", url, err);
            }
        }),
    );
}

// Version check
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
            await rebuildCache();
            await storeVersion(latest);

            const clients = await self.clients.matchAll();
            clients.forEach((c) =>
                c.postMessage({
                    action: "cacheInvalidated",
                    newVersion: latest,
                }),
            );
        }
    } catch (err) {
        console.warn("[SW] Version check failed:", err);
    }
}

// Lifecycle
self.addEventListener("install", (event) => {
    event.waitUntil(self.skipWaiting());
});

self.addEventListener("activate", (event) => {
    event.waitUntil(
        (async () => {
            await self.clients.claim();

            // Wipe ALL old caches
            const allCaches = await caches.keys();
            await Promise.all(
                allCaches.map((name) => {
                    if (name !== CACHE_NAME) {
                        swLog("Deleting old cache:", name);
                        return caches.delete(name);
                    }
                }),
            );

            // Pre-cache static assets on first install
            const cache = await caches.open(CACHE_NAME);
            await Promise.all(
                STATIC_ASSETS.map(async (url) => {
                    try {
                        const res = await fetch(url);
                        if (res.ok) await cache.put(url, res);
                    } catch (err) {
                        console.warn("[SW] Failed to pre-cache:", url, err);
                    }
                }),
            );

            swLog("Activated and ready.");
        })(),
    );
});

self.addEventListener("fetch", (event) => {
    const url = new URL(event.request.url);

    if (
        url.pathname === "/sw.js" ||
        url.pathname === CACHE_VER_URL ||
        url.pathname == "/settings_html"
    )
        return;
    if (event.request.method !== "GET" || url.origin !== self.location.origin)
        return;

    const swCookie = event.request.headers.get("cookie") ?? "";

    const swMatch = swCookie.match(/(?:^|;\s*)screen_width=([^;]*)/);
    const screenWidth = swMatch ? swMatch[1] : "default";

    const langMatch = swCookie.match(/(?:^|;\s*)lang=([^;]*)/);
    const lang = langMatch ? langMatch[1] : "default";

    const locMatch = swCookie.match(/(?:^|;\s*)loc=([^;]*)/);
    const loc = locMatch ? locMatch[1] : "default";

    const cacheKey =
        url.href + "__sw=" + screenWidth + "__lang=" + lang + "__loc=" + loc;

    event.respondWith(
        caches.match(cacheKey).then(async (cached) => {
            if (cached) return cached;

            try {
                const res = await fetch(event.request);
                if (res.ok) {
                    const cache = await caches.open(CACHE_NAME);
                    await cache.put(cacheKey, res.clone());
                    await trackAndEvict(cacheKey);
                }
                return res;
            } catch {
                return new Response("You're offline 😥", { status: 404 });
            }
        }),
    );
});

// Messages from main thread
self.addEventListener("message", (event) => {
    if (!event.data?.action) return;

    if (event.data.action === "checkVersion") {
        event.waitUntil(checkVersion());
    }

    if (event.data.action === "clearCache") {
        event.waitUntil(
            caches.delete(CACHE_NAME).then(() => {
                self.clients
                    .matchAll()
                    .then((clients) =>
                        clients.forEach((c) =>
                            c.postMessage({ action: "cacheCleared" }),
                        ),
                    );
            }),
        );
    }
});
