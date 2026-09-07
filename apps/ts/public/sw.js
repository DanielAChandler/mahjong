// Offline cache: app shell + assets, cache-first, base-path aware so the
// GitHub Pages subpath deployment works (scope derives from SW location).
const CACHE = "mahjong-v4";
const ASSETS = ["./", "./manifest.webmanifest"];

self.addEventListener("install", (e) => {
  e.waitUntil(caches.open(CACHE).then((c) => c.addAll(ASSETS)));
  self.skipWaiting();
});

self.addEventListener("activate", (e) => {
  e.waitUntil(
    (async () => {
      // hard purge: drop ALL old caches, including the bad cache-first v1/v2
      const keys = await caches.keys();
      await Promise.all(keys.filter((k) => k !== CACHE).map((k) => caches.delete(k)));
      // tell every open client a new SW is active
      const clients = await self.clients.matchAll({ includeUncontrolled: true });
      for (const c of clients) c.postMessage({ type: "SW_UPDATED" });
      await self.clients.claim();
    })(),
  );
});

self.addEventListener("fetch", (e) => {
  if (e.request.method !== "GET") return;
  const url = new URL(e.request.url);
  // navigations + hashed assets: network-first so deploys land immediately;
  // fall back to cache only when offline.
  if (e.request.mode === "navigate" || url.pathname.includes("/assets/")) {
    e.respondWith(
      fetch(e.request)
        .then((res) => {
          if (res.ok && url.origin === location.origin) {
            const copy = res.clone();
            caches.open(CACHE).then((c) => c.put(e.request, copy));
          }
          return res;
        })
        .catch(() => caches.match(e.request).then((hit) => hit || caches.match("./"))),
    );
    return;
  }
  // everything else: cache-first
  e.respondWith(
    caches.match(e.request).then(
      (hit) =>
        hit ??
        fetch(e.request).then((res) => {
          if (res.ok && url.origin === location.origin) {
            const copy = res.clone();
            caches.open(CACHE).then((c) => c.put(e.request, copy));
          }
          return res;
        }),
    ),
  );
});
