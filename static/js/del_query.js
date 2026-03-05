window.addEventListener("load", function () {
    var backButton = document.querySelector(".btn_del_query");
    backButton.addEventListener("click", function () {
        document.querySelector(".search_box").value = "";
        document.querySelector(".search_box").focus();
    });
});

// Prefetch results on user input pause
let prefetchTimer = null;
document.querySelector('[name="q"]').addEventListener("input", (e) => {
    clearTimeout(prefetchTimer);
    const query = e.target.value.trim();
    if (!query) return;

    prefetchTimer = setTimeout(() => {
        const t = document.querySelector('input[name="t"]').value;
        const lang =
            document.cookie.match(/(?:^|;\s*)lang=([^;]*)/)?.[1] ?? "en";
        const loc = document.cookie.match(/(?:^|;\s*)loc=([^;]*)/)?.[1] ?? "";
        const params = new URLSearchParams({ t, lang, loc, q: query });
        fetch(`/results_html?${params}`, {
            headers: { "requested-with": "js" },
        }).catch(() => {});
    }, 500);
});
