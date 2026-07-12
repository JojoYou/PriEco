const form = document.getElementById("goggles-form");

if (form) {
  let pending = false;
  let queued = false;

  async function applyGoggles() {
    if (pending) {
      queued = true;
      return;
    }
    pending = true;

    const ids = Array.from(
      form.querySelectorAll('input[name="ids"]:checked'),
    ).map((cb) => cb.value);
    const params = new URLSearchParams();
    ids.forEach((id) => params.append("ids", id));

    try {
      const res = await fetch(`/goggles/apply?${params.toString()}`, {
        method: "GET",
        credentials: "same-origin",
      });
      if (!res.ok) {
        console.warn("Failed to apply goggles, server responded", res.status);
      }
    } catch (err) {
      console.warn("Failed to apply goggles:", err);
    } finally {
      pending = false;
      if (queued) {
        queued = false;
        applyGoggles();
      }
    }
  }

  form.addEventListener("submit", (e) => {
    e.preventDefault();
    applyGoggles();
  });

  form.querySelectorAll('input[name="ids"]').forEach((checkbox) => {
    checkbox.addEventListener("change", applyGoggles);
  });
}
