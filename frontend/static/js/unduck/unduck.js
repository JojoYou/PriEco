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
var unduckSearch = document.currentScript ? new URL(document.currentScript.src).search : "";

function getBangRedirectUrls() {
  var url = new URL(window.location.href);
  var queryMeta = document.querySelector('meta[name="prieco-query"]');
  var query = url.searchParams.get("q") || (queryMeta && queryMeta.content);
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
  if (!searchUrls || searchUrls.length === 0) return false;

  if (searchUrls.length === 1) {
    window.location.replace(searchUrls[0]);
    return true;
  }

  var blockedCount = 0;

  for (var i = 0; i < searchUrls.length - 1; i++) {
    var win = window.open(searchUrls[i], "_blank");
    if (!win || win.closed || typeof win.closed === "undefined") {
      blockedCount++;
    }
  }

  if (blockedCount > 0) {
    return false;
  }

  window.location.replace(searchUrls[searchUrls.length - 1]);
  return true;
}

var unduckQuery =
  new URL(window.location.href).searchParams.get("q") ||
  document.querySelector('meta[name="prieco-query"]')?.content ||
  "";

if (location.pathname === "/search" && unduckQuery) {
  document.title = unduckQuery + " | PriEco";
  var fillQuery = new MutationObserver(function () {
    document.querySelectorAll('input[name="q"]').forEach(function (input) {
      if (!input.value) input.value = unduckQuery;
    });
  });
  fillQuery.observe(document.documentElement, { childList: true, subtree: true });
  document.addEventListener("DOMContentLoaded", function () {
    fillQuery.disconnect();
  });
}

if (unduckQuery.split(/\s+/).some((w) => w.length > 1 && w.startsWith("!"))) {
  document.documentElement.style.visibility = "hidden";
  window.priecoBangCheck = new Promise(function (resolve) {
    var script = document.createElement("script");
    script.src = "/static/js/unduck/bangs.js" + unduckSearch;
    script.onload = function () {
      resolve(doRedirect());
    };
    script.onerror = function () {
      resolve(false);
    };
    document.head.appendChild(script);
  }).then(function (redirected) {
    if (!redirected) document.documentElement.style.visibility = "";
    return redirected;
  });
}
