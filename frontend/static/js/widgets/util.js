(function () {
  if (window.PriEcoWidgets) return;

  function el(tag, attrs, children) {
    const node = document.createElement(tag);
    if (attrs) {
      for (const [key, value] of Object.entries(attrs)) {
        if (key === "text") node.textContent = value;
        else if (key === "class") node.className = value;
        else node.setAttribute(key, value);
      }
    }
    for (const child of children || []) node.appendChild(child);
    return node;
  }

  function getQuery() {
    const fromUrl = new URLSearchParams(window.location.search).get("q");
    const raw = fromUrl || (document.querySelector('input[name="q"]') || {}).value || "";
    return raw.trim().replace(/\s+/g, " ");
  }

  function formatNumber(x) {
    if (!isFinite(x)) return "Error";
    if (x === 0) return "0";
    const abs = Math.abs(x);
    if (abs < 1e-6 || abs >= 1e15) return x.toPrecision(8).replace(/\.?0+e/, "e");
    return String(parseFloat(x.toPrecision(12)));
  }

  let mounted = false;

  function offer(test, render) {
    if (mounted) return;
    const query = getQuery();
    if (!query) return;
    const state = test(query, query.toLowerCase());
    if (state === null || state === undefined || state === false) return;
    const results = document.querySelector(".results");
    if (!results) return;
    const card = render(state);
    const spell = results.querySelector(".spell_check");
    if (spell) spell.after(card);
    else results.prepend(card);
    mounted = true;
  }

  window.PriEcoWidgets = { el, formatNumber, offer };
})();
