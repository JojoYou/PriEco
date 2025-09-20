if ("serviceWorker" in navigator) {
  window.addEventListener("load", () => {
    navigator.serviceWorker
      .register("sw.js")
      .then((registration) => {
        console.log("ServiceWorker registered with scope:", registration.scope);
      })
      .catch((error) => {
        console.log("ServiceWorker registration failed:", error);
      });
  });
}

function getBangredirectUrl() {
  var url = new URL(window.location.href);
  var query = url.searchParams.get("q");
  query = query ? query.trim() : "";

  if (!query) {
    return null;
  }

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

  // If no bang is provided, do not use defaultBang and return null
  if (!selectedBang) return null;

  // Remove the first bang from the query
  var cleanQuery = query.replace(/!\S+\s*/i, "").trim();

  // If the query is just `!gh`, use `github.com` instead of `github.com/search?q=`
  if (cleanQuery === "") {
    return selectedBang ? "https://" + selectedBang.d : null;
  }

  // Format of the url is:
  // https://www.google.com/search?q={{{s}}}
  var searchUrl =
    selectedBang && selectedBang.u
      ? selectedBang.u.replace(
          "{{{s}}}",
          encodeURIComponent(cleanQuery).replace(/%2F/g, "/"),
        )
      : null;

  if (!searchUrl) return null;

  return searchUrl;
}

function doRedirect() {
  var searchUrl = getBangredirectUrl();
  if (!searchUrl) return; // Do nothing if no valid bang or search URL
  window.location.replace(searchUrl);
}

doRedirect();
