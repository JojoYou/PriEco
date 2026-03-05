if ("serviceWorker" in navigator) {
    window.addEventListener("load", async () => {
        try {
            const registration =
                await navigator.serviceWorker.register("/sw.js");
            console.log("ServiceWorker registered:", registration.scope);
        } catch (err) {
            console.warn("ServiceWorker registration failed:", err);
            return;
        }

        // Listen for SW messages
        navigator.serviceWorker.addEventListener("message", (event) => {
            if (event.data.action === "cacheInvalidated") {
                console.log(
                    "Cache invalidated, new version:",
                    event.data.newVersion,
                );
            }
            if (event.data.action === "cacheCleared") {
                console.log("Cache manually cleared.");
            }
        });

        // Check version
        navigator.serviceWorker.ready.then((reg) => {
            if (reg.active) {
                reg.active.postMessage({ action: "checkVersion" });
            }
        });
    });
}

// Bangs
function getBangredirectUrl() {
    var url = new URL(window.location.href);
    var query = url.searchParams.get("q");
    query = query ? query.trim() : "";
    if (!query) return null;

    var match = query.match(/!(\S+)/i);
    var bangCandidate = match && match[1] ? match[1].toLowerCase() : null;
    var selectedBang = null;

    if (bangCandidate) {
        for (var i = 0; i < bangs.length; i++) {
            if (bangs[i].t === bangCandidate) {
                selectedBang = bangs[i];
                break;
            }
        }
    }

    if (!selectedBang) return null;

    var cleanQuery = query.replace(/!\S+\s*/i, "").trim();
    if (cleanQuery === "") {
        return "https://" + selectedBang.d;
    }

    var searchUrl = selectedBang.u
        ? selectedBang.u.replace(
              "{{{s}}}",
              encodeURIComponent(cleanQuery).replace(/%2F/g, "/"),
          )
        : null;

    return searchUrl;
}

function doRedirect() {
    var searchUrl = getBangredirectUrl();
    if (!searchUrl) return;
    window.location.replace(searchUrl);
}

doRedirect();
