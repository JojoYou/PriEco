document.addEventListener("DOMContentLoaded", () => {
  document.addEventListener("click", async (e) => {
    const button = e.target.closest(".vote-btn");
    if (!button) return;

    const isLike = button.classList.contains("like-btn");

    const parentDiv = button.closest(".vote-buttons");
    const sectionWrapper = button.closest(".center-wrapper");

    const h2 = sectionWrapper.querySelector("h2");
    const featureName = h2 ? h2.textContent.trim() : "Unknown Feature";

    const allButtons = parentDiv.querySelectorAll("button");
    allButtons.forEach((b) => (b.disabled = true));

    try {
      const response = await fetch("/roadmap/vote", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          feature: featureName,
          is_like: isLike,
        }),
      });

      if (response.ok) {
        parentDiv.innerHTML = "<span class='vote-thanks'>Thank you!</span>";
      } else {
        console.error("Failed to send vote. Status:", response.status);
        allButtons.forEach((b) => (b.disabled = false));
      }
    } catch (error) {
      console.error("Network error:", error);
      allButtons.forEach((b) => (b.disabled = false));
    }
  });
});
