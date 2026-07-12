const form = document.getElementById("goggles-form");

if (form) {
  function updateCookieLocally() {
    const ids = Array.from(
      form.querySelectorAll('input[name="ids"]:checked'),
    ).map((cb) => cb.value);

    if (ids.length === 0) {
      document.cookie = "active_goggles=; path=/; max-age=0";
    } else {
      const joined = ids.join(",");
      document.cookie = `active_goggles=${joined}; path=/; max-age=31536000`;
    }
  }

  form.querySelectorAll('input[name="ids"]').forEach((checkbox) => {
    checkbox.addEventListener("change", updateCookieLocally);
  });

  form.addEventListener("submit", (e) => {
    e.preventDefault();
    updateCookieLocally();
  });
}
