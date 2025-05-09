const oldCacheName = 'at_2048_cache_v1'
const currentCacheName = 'at_2048_cache_v0.1'
const cacheAbleHosts = [
    '2048.blue',
    // '127.0.0.1'
];

/* Start the service worker and cache all of the app's content */
self.addEventListener('install', function (e) {
    //TODO add the acutal file names to pre cache since they dont have hashes now
    // e.waitUntil(precache());
});


async function clearOldCache() {
    const cache = await caches.open(oldCacheName);
    cache.keys().then(function (keys) {
        keys.forEach(function (key) {
            cache.delete(key);
        });
    });
}

self.addEventListener('activate', function (e) {
    e.waitUntil(clearOldCache());
});


async function networkFirst(request) {
    try {
        const networkResponse = await fetch(request);
        if (networkResponse.ok) {
            const cache = await caches.open(currentCacheName);
            cache.put(request, networkResponse.clone());
        }
        return networkResponse;
    } catch (error) {
        const cachedResponse = await caches.match(request);
        return cachedResponse || Response.error();
    }
}

async function cacheFirstWithRefresh(request) {
    const fetchResponsePromise = fetch(request).then(async (networkResponse) => {
        if (networkResponse.ok) {
            const cache = await caches.open(currentCacheName);
            cache.put(request, networkResponse.clone());
        }
        return networkResponse;
    });

    return (await caches.match(request)) || (await fetchResponsePromise);
}


async function cacheFirst(request) {
    const cachedResponse = await caches.match(request);
    if (cachedResponse) {
        return cachedResponse;
    }
    try {
        const networkResponse = await fetch(request);
        if (networkResponse.ok) {
            const cache = await caches.open(currentCacheName);
            cache.put(request, networkResponse.clone());
        }
        return networkResponse;
    } catch (error) {
        return Response.error();
    }
}


/* Serve cached content when offline */
self.addEventListener('fetch', function (e) {
    const url = new URL(e.request.url);

    if (cacheAbleHosts.includes(url.hostname) && !url.pathname.startsWith('/api')) {
        e.respondWith(cacheFirst(e.request));
    }

});