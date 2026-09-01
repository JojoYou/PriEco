(function () {
  if (window.__fxWidgetInit) return;
  window.__fxWidgetInit = true;

  function autoGrow(input) {
    const len = Math.max(input.value.length + 1, 3);
    input.style.width = len + "ch";
  }

  document.addEventListener("input", function (e) {
    if (e.target.id !== "fx_input_amount") return;

    const input = e.target;
    autoGrow(input);

    const card = input.closest(".fx_card");
    if (!card) return;

    const rate = parseFloat(card.dataset.rate);
    const amount = parseFloat(input.value);
    const display = card.querySelector("#fx_display_converted");

    if (!display) return;

    if (isNaN(rate) || isNaN(amount)) {
      display.textContent = "0";
      return;
    }

    const converted = Math.round(amount * rate * 100) / 100;
    display.textContent = converted;
  });

  document.querySelectorAll("#fx_input_amount").forEach(autoGrow);
})();
