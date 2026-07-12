const COOKIE_NAME = "prieco_qt_prefs";

function getPrefs() {
  console.log(`[Quick Tune] Attempting to read cookie: ${COOKIE_NAME}`);
  const match = document.cookie.match(
    new RegExp("(^| )" + COOKIE_NAME + "=([^;]+)"),
  );

  if (match) {
    try {
      const parsed = JSON.parse(decodeURIComponent(match[2]));
      console.log("[Quick Tune] Successfully parsed existing cookie:", parsed);
      return parsed;
    } catch (e) {
      console.error("[Quick Tune] Failed to parse cookie JSON:", e);
    }
  } else {
    console.log(
      "[Quick Tune] No existing cookie found. Returning default state.",
    );
  }

  if ("serviceWorker" in navigator && navigator.serviceWorker.controller) {
    navigator.serviceWorker.controller.postMessage({ action: "clearCache" });
  }

  return { boost: [], downrank: [], discard: [] };
}

function savePrefs(prefs) {
  console.log("[Quick Tune] Preparing to save preferences:", prefs);
  const expiry = new Date(Date.now() + 31536000000).toUTCString(); // 1 year
  const encodedData = encodeURIComponent(JSON.stringify(prefs));

  const cookieString = `${COOKIE_NAME}=${encodedData}; expires=${expiry}; path=/; SameSite=Lax`;
  document.cookie = cookieString;
  console.log("[Quick Tune] Cookie successfully written to document.");
}

document.addEventListener("click", function (e) {
  const toggleBtn = e.target.closest(".quick_tune");
  if (toggleBtn) {
    console.log("[Quick Tune] Menu toggle button clicked.");
    const menu = document.querySelector(".qt_menu");
    if (menu) {
      const isNowActive = menu.classList.toggle("active");
      console.log(`[Quick Tune] Menu active state is now: ${isNowActive}`);
    } else {
      console.error("[Quick Tune] Could not find .qt_menu element in the DOM.");
    }
    return;
  }

  const actionBtn = e.target.closest(".qt-btn");
  if (actionBtn) {
    const domainItem = actionBtn.closest(".qt_item");
    if (!domainItem) {
      console.error(
        "[Quick Tune] Clicked an action button, but could not find parent .qt_item.",
      );
      return;
    }

    const domain = domainItem.dataset.domain;
    const action = actionBtn.dataset.action;
    const isTurningOff = actionBtn.classList.contains("active");

    console.log(
      `[Quick Tune] Action button clicked -> Domain: "${domain}", Action: "${action}", Toggling Off: ${isTurningOff}`,
    );

    const allBtnsInRow = domainItem.querySelectorAll(".qt-btn");
    allBtnsInRow.forEach((btn) => btn.classList.remove("active"));
    console.log(
      `[Quick Tune] Removed 'active' class from all buttons for ${domain}.`,
    );

    if (!isTurningOff) {
      actionBtn.classList.add("active");
      console.log(`[Quick Tune] Added 'active' class to ${action} button.`);
    }

    let prefs = getPrefs();
    console.log(
      "[Quick Tune] State before modifications:",
      JSON.parse(JSON.stringify(prefs)),
    );

    prefs.boost = prefs.boost.filter((d) => d !== domain);
    prefs.downrank = prefs.downrank.filter((d) => d !== domain);
    prefs.discard = prefs.discard.filter((d) => d !== domain);

    console.log(`[Quick Tune] Cleared "${domain}" from all arrays.`);

    if (!isTurningOff) {
      prefs[action].push(domain);
      console.log(`[Quick Tune] Pushed "${domain}" to "${action}" array.`);
    } else {
      console.log(
        `[Quick Tune] Neutral state selected. "${domain}" remains cleared.`,
      );
    }

    console.log("[Quick Tune] Final state to be saved:", prefs);

    savePrefs(prefs);
  }
});
