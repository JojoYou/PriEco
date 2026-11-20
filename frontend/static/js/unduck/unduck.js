if ("serviceWorker" in navigator) {
  window.addEventListener("load", async () => {
    try {
      const registration = await navigator.serviceWorker.register("/sw.js");
      console.log("ServiceWorker registered:", registration.scope);
    } catch (err) {
      console.warn("ServiceWorker registration failed:", err);
      return;
    }

    // Listen for SW messages
    navigator.serviceWorker.addEventListener("message", (event) => {
      if (event.data.action === "cacheInvalidated") {
        console.log("Cache invalidated, new version:", event.data.newVersion);
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
function getBangRedirectUrls() {
  var url = new URL(window.location.href);
  var query = url.searchParams.get("q");
  if (!query) return [];

  var words = query.split(/\s+/);
  var bangTokens = words.filter((w) => w.startsWith("!"));
  if (bangTokens.length === 0) return [];

  var selectedBangs = [];
  var bangCandidates = bangTokens.map((b) => b.substring(1).toLowerCase());

  for (var i = 0; i < bangCandidates.length; i++) {
    var candidate = bangCandidates[i];
    for (var j = 0; j < bangs.length; j++) {
      if (bangs[j].t === candidate) {
        selectedBangs.push(bangs[j]);
        break;
      }
    }
  }

  if (selectedBangs.length === 0) return [];

  var cleanQuery = query;
  bangTokens.forEach((token) => {
    var regex = new RegExp(
      token.replace(/[.*+?^${}()|[\]\\]/g, "\\$&") + "\\s*",
      "gi",
    );
    cleanQuery = cleanQuery.replace(regex, "");
  });
  cleanQuery = cleanQuery.trim();

  return selectedBangs
    .map((bang) => {
      if (cleanQuery === "") {
        return "https://" + bang.d;
      }
      return bang.u
        ? bang.u.replace(
            "{{{s}}}",
            encodeURIComponent(cleanQuery).replace(/%2F/g, "/"),
          )
        : null;
    })
    .filter((u) => u !== null);
}

function doRedirect() {
  var searchUrls = getBangRedirectUrls();
  if (!searchUrls || searchUrls.length === 0) return;

  if (searchUrls.length === 1) {
    window.location.replace(searchUrls[0]);
    return;
  }

  var blockedCount = 0;

  for (var i = 0; i < searchUrls.length - 1; i++) {
    var win = window.open(searchUrls[i], "_blank");
    if (!win || win.closed || typeof win.closed === "undefined") {
      blockedCount++;
    }
  }

  if (blockedCount > 0) {
    return;
  }

  window.location.replace(searchUrls[searchUrls.length - 1]);
}

doRedirect();
