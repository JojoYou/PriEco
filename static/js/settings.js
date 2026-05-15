// Own index
const checkbox_index = document.getElementById("check_index");
checkbox_index.checked = document.cookie
  .split("; ")
  .some((s) => s === "index=1");
checkbox_index.addEventListener("change", () => {
  document.cookie =
    "index=" +
    (checkbox_index.checked ? "1; max-age=31536000" : "; max-age=0") +
    "; path=/; SameSite=Strict; Secure";
  location.reload();
});

// Language selection
document.getElementById("lang_select").addEventListener("change", function() {
  document.cookie =
    "lang=" +
    this.value +
    "; path=/; SameSite=Strict; Secure; max-age=31536000";
  location.reload();
});
// Location selection
document.getElementById("loc_select").addEventListener("change", function() {
  document.cookie =
    "loc=" + this.value + "; path=/; SameSite=Strict; Secure; max-age=31536000";
  location.reload();
});

// Open links in a new tab
const checkbox_newtab = document.getElementById("check_newtab");
checkbox_newtab.checked = document.cookie
  .split("; ")
  .some((s) => s === "newtab=1");
const updateLinks = () => {
  document.querySelectorAll("a.link").forEach((a) => {
    if (checkbox_newtab.checked) a.setAttribute("target", "_blank");
    else a.removeAttribute("target");
  });
};
updateLinks();
checkbox_newtab.addEventListener("change", () => {
  document.cookie =
    "newtab=" +
    (checkbox_newtab.checked ? "1; max-age=31536000" : "; max-age=0") +
    "; path=/; SameSite=Strict; Secure;";
  updateLinks();
});

// Theme
function setScreenWidthCookie() {
  const currentWidth = window.innerWidth;
  const cookieMatch = document.cookie.match(/screen_width=(\d+)/);
  const storedWidth = cookieMatch ? parseInt(cookieMatch[1], 10) : null;

  // Determine if breakpoint has changed
  const crossedThreshold =
    storedWidth === null || // first load
    (storedWidth < 890 && currentWidth >= 890) ||
    (storedWidth >= 890 && currentWidth < 890);

  // Only reload if breakpoint changed and not first load
  if (crossedThreshold || storedWidth == null) {
    document.cookie =
      "screen_width=" +
      currentWidth +
      "; path=/; SameSite=Strict; Secure; max-age=31536000";
    location.reload();
  }
}

// Initialize
setScreenWidthCookie();

// Add resize listener with debounce
window.addEventListener("resize", () => {
  clearTimeout(window._resizeTimeout);
  window._resizeTimeout = setTimeout(setScreenWidthCookie, 200);
});

// Theme
const r = document.querySelectorAll('input[name="theme"]'),
  cookie = (n, v) =>
    v !== undefined
      ? (document.cookie = `${n}=${v};path=/;SameSite=Lax;Secure;max-age=${30 * 24 * 60 * 60}`)
      : document.cookie
        .split("; ")
        .find((c) => c.startsWith(n + "="))
        ?.split("=")[1],
  del = (n) => (document.cookie = `${n}=;path=/;max-age=0`),
  swapCSS = (theme) =>
    document.querySelectorAll('link[rel="stylesheet"]').forEach((l) => {
      ["light", "dark", "system"].forEach((t) => {
        l.href = l.href.split(`/css/${t}/`).join(`/css/${theme}/`);
      });
    });

let t = cookie("theme") || "system";
r.forEach((x) => (x.checked = x.value === t));
r.forEach((x) =>
  x.addEventListener("change", (e) => {
    e.target.value === "system"
      ? del("theme")
      : cookie("theme", e.target.value);
    if ("serviceWorker" in navigator && navigator.serviceWorker.controller) { navigator.serviceWorker.controller.postMessage({ action: "clearCache" }); }
    swapCSS(e.target.value);
  }),
);

// No JS
let c = document.getElementById("check_js");
if (c) {
  c.checked = /\bjs=1/.test(document.cookie);
  c.onchange = async () => {
    document.cookie = `js=${c.checked ? "1;max-age=31536000" : ";max-age=0"};path=/;SameSite=Strict;Secure`;
    if (c.checked && "serviceWorker" in navigator) {
      try {
        await caches.delete("prieco-cache");
        const registrations = await navigator.serviceWorker.getRegistrations();
        for (let registration of registrations) {
          await registration.unregister();
        }
      } catch (err) {
        console.warn("Failed to clear SW:", err);
      }
    }
    location.reload();
  };
}

// POST
let check_post = document.getElementById("check_post");
if (check_post) {
  check_post.checked = /\bpost=1/.test(document.cookie);
  check_post.onchange = async () => {
    document.cookie = `post=${check_post.checked ? "1;max-age=31536000" : ";max-age=0"};path=/;SameSite=Strict;Secure`;
    if ("serviceWorker" in navigator) {
      try {
        await caches.delete("prieco-cache");
        const registrations = await navigator.serviceWorker.getRegistrations();
        for (let registration of registrations) {
          await registration.unregister();
        }
      } catch (err) {
        console.warn("Failed to clear SW:", err);
      }
    }

    location.reload();
  };
}
