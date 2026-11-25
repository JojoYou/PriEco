(function () {
  const { el, offer } = window.PriEcoWidgets;

  function hexToRgb(hex) {
    let h = hex.replace("#", "");
    if (h.length === 3) h = h.split("").map((c) => c + c).join("");
    if (!/^[0-9a-f]{6}$/i.test(h)) return null;
    const n = parseInt(h, 16);
    return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
  }

  function rgbToHex(r, g, b) {
    return "#" + [r, g, b].map((v) => v.toString(16).padStart(2, "0")).join("");
  }

  function rgbToHsl(r, g, b) {
    r /= 255;
    g /= 255;
    b /= 255;
    const max = Math.max(r, g, b);
    const min = Math.min(r, g, b);
    const l = (max + min) / 2;
    let h = 0;
    let s = 0;
    if (max !== min) {
      const d = max - min;
      s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
      if (max === r) h = (g - b) / d + (g < b ? 6 : 0);
      else if (max === g) h = (b - r) / d + 2;
      else h = (r - g) / d + 4;
      h *= 60;
    }
    return [Math.round(h), Math.round(s * 100), Math.round(l * 100)];
  }

  function render(initial) {
    const picker = el("input", { type: "color", class: "iw_color_input", "aria-label": "Pick a color" });
    const hex = el("p", { class: "iw_col_val" });
    const rgb = el("p", { class: "iw_col_val" });
    const hsl = el("p", { class: "iw_col_val" });

    function set(hexValue) {
      const c = hexToRgb(hexValue);
      if (!c) return;
      const normalized = rgbToHex(c[0], c[1], c[2]);
      const [h, s, l] = rgbToHsl(c[0], c[1], c[2]);
      picker.value = normalized;
      hex.textContent = normalized;
      rgb.textContent = c.join(", ");
      hsl.textContent = h + ", " + s + "%, " + l + "%";
    }

    picker.addEventListener("input", () => set(picker.value));

    const box = (title, value) =>
      el("div", { class: "iw_col" }, [el("p", { class: "iw_col_tit", text: title }), value]);

    set(initial || "#0ed794");

    return el("div", { class: "iw_card iw_color" }, [
      el("div", { class: "iw_color_wrap" }, [
        el("div", {}, [picker]),
        el("div", {}, [box("Hex", hex), box("RGB", rgb), box("HSL", hsl)]),
      ]),
    ]);
  }

  offer(function (q, lower) {
    const hex = q.match(/^(?:#|hex\s+#?)([0-9a-f]{3}|[0-9a-f]{6})$/i);
    if (hex) return "#" + hex[1];
    if (/\b(colou?r ?picker|colou?r|hex colou?r|colou?r code)\b/.test(lower) && lower.split(" ").length <= 3) {
      return "";
    }
    return null;
  }, render);
})();
