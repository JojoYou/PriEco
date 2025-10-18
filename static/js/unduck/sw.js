const CACHE_NAME = "prieco-browser-cache?v=0.1.2";
const EXCLUDED_FILES = ["/settings_html", "/api"];
const MAX_CACHE_ITEMS = 2000;

// Key for storing cache metadata
const CACHE_METADATA_KEY = "cache-metadata";

self.addEventListener("activate", (event) => {
  event.waitUntil(
    (async () => {
      await self.clients.claim();

      // Clean up old caches
      const cacheNames = await caches.keys();
      await Promise.all(
        cacheNames.map((cacheName) => {
          if (cacheName !== CACHE_NAME) {
            swLog("Deleting old cache:", cacheName);
            return caches.delete(cacheName);
          }
        }),
      );

      // Pre-fetch root document
      const cache = await caches.open(CACHE_NAME);
      try {
        const response = await fetch("/");
        if (response.ok) {
          await cache.put("/", response.clone());
          swLog("Root / successfully pre-cached.");
        }
      } catch (err) {
        console.warn("Error fetching root / during activate:", err);
      }
    })(),
  );
});

// Helper functions for cache metadata management
async function getCacheMetadata() {
  try {
    const cache = await caches.open(CACHE_NAME);
    const response = await cache.match(CACHE_METADATA_KEY);
    if (response) {
      return await response.json();
    }
  } catch (err) {
    console.warn("Error reading cache metadata:", err);
  }
  return { urls: [], timestamps: {} };
}

async function setCacheMetadata(metadata) {
  try {
    const cache = await caches.open(CACHE_NAME);
    const response = new Response(JSON.stringify(metadata), {
      headers: { "Content-Type": "application/json" },
    });
    await cache.put(CACHE_METADATA_KEY, response);
  } catch (err) {
    console.warn("Error saving cache metadata:", err);
  }
}

async function addToCache(request, response) {
  if (request.method !== "GET") {
    return;
  }
  const cache = await caches.open(CACHE_NAME);
  const url = request.url;
  const pathname = new URL(url).pathname;

  // Store the response
  await cache.put(request, response.clone());

  // Update metadata for evictable items
  if (pathname.startsWith("/proxy") || pathname.startsWith("/results_html")) {
    const metadata = await getCacheMetadata();

    // Remove if already exists to update position
    const existingIndex = metadata.urls.indexOf(url);
    if (existingIndex > -1) {
      metadata.urls.splice(existingIndex, 1);
    }

    // Add to end (most recent)
    metadata.urls.push(url);
    metadata.timestamps[url] = Date.now();

    // Evict oldest items if over limit
    while (metadata.urls.length > MAX_CACHE_ITEMS) {
      const oldestUrl = metadata.urls.shift();
      delete metadata.timestamps[oldestUrl];
      try {
        await cache.delete(oldestUrl);
        swLog("Evicted from cache:", oldestUrl);
      } catch (err) {
        console.warn("Error evicting cache item:", err);
      }
    }

    await setCacheMetadata(metadata);
  }
}

self.addEventListener("fetch", (event) => {
  const url = new URL(event.request.url);

  // Skip excluded files
  if (EXCLUDED_FILES.some((excluded) => url.pathname.startsWith(excluded))) {
    return;
  }

  event.respondWith(
    caches.match(event.request).then(async (cached) => {
      if (cached) return cached;

      try {
        const res = await fetch(event.request);
        if (res.ok && event.request.method === "GET") {
          await addToCache(event.request, res);
        }
        return res;
      } catch (err) {
        return new Response("You're offline 😥", { status: 404 });
      }
    }),
  );
});
