(function () {
  const { el, offer } = window.PriEcoWidgets;

  function randomInt(min, max) {
    if (min > max) [min, max] = [max, min];
    const range = max - min + 1;
    const buf = new Uint32Array(1);
    const limit = Math.floor(4294967296 / range) * range;
    let x;
    do {
      crypto.getRandomValues(buf);
      x = buf[0];
    } while (x >= limit);
    return min + (x % range);
  }

  function render({ min, max, prefill }) {
    const output = el("p", { class: "iw_rn_num" });
    const from = el("input", { type: "number", class: "iw_rn_inp", placeholder: "From", step: "1", min: "0" });
    const to = el("input", { type: "number", class: "iw_rn_inp", placeholder: "To", step: "1", min: "0" });
    const roll = el("button", { type: "button", class: "iw_rn_inp iw_rn_btn", text: "🎲", "aria-label": "Generate" });
    if (prefill) {
      from.value = String(min);
      to.value = String(max);
    }

    function generate() {
      const a = from.value === "" ? 1 : Math.trunc(Number(from.value));
      const b = to.value === "" ? 10 : Math.trunc(Number(to.value));
      if (!isFinite(a) || !isFinite(b) || Math.abs(b - a) >= 4294967296) {
        output.textContent = "Invalid input";
        return;
      }
      output.textContent = String(randomInt(a, b));
    }

    roll.addEventListener("click", generate);
    for (const field of [from, to]) {
      field.addEventListener("keydown", (e) => {
        if (e.key === "Enter") generate();
      });
    }
    output.textContent = String(randomInt(min, max));

    return el("div", { class: "iw_card iw_random" }, [output, el("div", { class: "iw_rn_row" }, [from, to, roll])]);
  }

  offer(function (q, lower) {
    const m = lower.match(
      /^(?:random|rand|rng)(?: number| integer| int)?(?: generator)?(?:\s+(?:between|from)?\s*(-?\d+)\s*(?:-|to|and|,)?\s*(-?\d+))?$/
    );
    if (m) {
      const ranged = m[1] !== undefined;
      return { min: ranged ? parseInt(m[1], 10) : 1, max: ranged ? parseInt(m[2], 10) : 10, prefill: ranged };
    }
    if (/\brandom number\b/.test(lower)) return { min: 1, max: 10, prefill: false };
    return null;
  }, render);
})();
