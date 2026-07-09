document.addEventListener("click", async (e) => {
  const button = e.target.closest(".copy_btn");
  if (!button) return;

  const text = button.dataset.copy;

  try {
    await navigator.clipboard.writeText(text);

    const original = button.textContent;
    button.textContent = "Copied!";
    button.disabled = true;

    setTimeout(() => {
      button.textContent = original;
      button.disabled = false;
    }, 1500);
  } catch (err) {
    console.error("Failed to copy:", err);
    alert("Copy failed. Please copy the address manually.");
  }
});
