// Corvus service worker: cache-first, one cache per commit.
// The Pages build writes sw.js from this file with __VERSION__ = the commit.
// Plain ES2017 for iOS 12 Safari.
var VERSION = "__VERSION__";
var CACHE = "corvus-" + VERSION;
var CORE = [
  "./",
  "index.html",
  "manifest.json",
  "favicon.png",
  "apple-touch-icon.png",
  "social-preview.png",
  "icons/corvus-logo.png",
  "icons/icon-192.png",
  "icons/icon-512.png",
  "icons/apple-touch-icon-152x152.png",
  "icons/apple-touch-icon-167x167.png",
  "icons/apple-touch-icon-180x180.png",
  "landing.js",
  "guide/console.png",
  "guide/splash.png",
  "app/",
  "app/index.html",
  "app/app.js",
  "app/tour.js",
  "app/dio.js",
  "app/diogenes.md",
  "app/pkg/corvus_web.js",
  "app/pkg/corvus_web_bg.wasm",
];

self.addEventListener("install", function (event) {
  event.waitUntil(
    caches.open(CACHE)
      .then(function (cache) { return cache.addAll(CORE); })
      .then(function () { return self.skipWaiting(); })
  );
});

self.addEventListener("activate", function (event) {
  event.waitUntil(
    caches.keys()
      .then(function (keys) {
        return Promise.all(keys.filter(function (k) { return k.indexOf("corvus-") === 0 && k !== CACHE; })
          .map(function (k) { return caches.delete(k); }));
      })
      .then(function () { return self.clients.claim(); })
  );
});

self.addEventListener("fetch", function (event) {
  var request = event.request;
  if (request.method !== "GET" || new URL(request.url).origin !== self.location.origin) return;
  event.respondWith(
    caches.open(CACHE).then(function (cache) {
      // ignoreSearch: app/?profile=... is the same page as app/.
      return cache.match(request, { ignoreSearch: true }).then(function (hit) {
        if (hit) return hit;
        return fetch(request).then(function (response) {
          if (response.ok && response.type === "basic") cache.put(request, response.clone());
          return response;
        }).catch(function () {
          if (request.mode === "navigate") {
            var app = new URL(request.url).pathname.indexOf("/app") >= 0;
            return cache.match(app ? "app/index.html" : "index.html");
          }
          throw new Error("offline and not cached: " + request.url);
        });
      });
    })
  );
});
