const COOKIE_NAME = "prieco_qt_prefs";

function getPrefs() {
  const match = document.cookie.match(
    new RegExp("(^| )" + COOKIE_NAME + "=([^;]+)"),
  );

  if (match) {
    try {
      return JSON.parse(decodeURIComponent(match[2]));
    } catch (e) {}
  }

  if ("serviceWorker" in navigator && navigator.serviceWorker.controller) {
    navigator.serviceWorker.controller.postMessage({ action: "clearCache" });
  }

  return { boost: [], downrank: [], discard: [] };
}

function savePrefs(prefs) {
  const expiry = new Date(Date.now() + 31536000000).toUTCString();
  const encodedData = encodeURIComponent(JSON.stringify(prefs));
  document.cookie = `${COOKIE_NAME}=${encodedData}; expires=${expiry}; path=/; SameSite=Lax`;
  if ("serviceWorker" in navigator && navigator.serviceWorker.controller) {
    navigator.serviceWorker.controller.postMessage({ action: "clearCache" });
  }
}

document.addEventListener("click", function (e) {
  const toggleBtn = e.target.closest(".quick_tune");
  if (toggleBtn) {
    const menu = document.querySelector(".qt_menu");
    if (menu) menu.classList.toggle("active");
    return;
  }

  const actionBtn = e.target.closest(".qt-btn");
  if (actionBtn) {
    const action = actionBtn.dataset.action;
    if (!action) return;

    e.preventDefault();

    const domainItem = actionBtn.closest(".qt_item");
    if (!domainItem) return;

    const domain = domainItem.dataset.domain;
    const isTurningOff = actionBtn.classList.contains("active");

    domainItem
      .querySelectorAll(".qt-btn")
      .forEach((btn) => btn.classList.remove("active"));

    let prefs = getPrefs();

    prefs.boost = prefs.boost.filter((d) => d !== domain);
    prefs.downrank = prefs.downrank.filter((d) => d !== domain);
    prefs.discard = prefs.discard.filter((d) => d !== domain);

    if (action === "none" || isTurningOff) {
      const noneBtn = domainItem.querySelector('[data-action="none"]');
      if (noneBtn) noneBtn.classList.add("active");
    } else {
      actionBtn.classList.add("active");
      prefs[action].push(domain);
    }

    savePrefs(prefs);
  }
});
