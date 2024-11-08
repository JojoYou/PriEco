const handleElement = document.getElementById("notifyHandle");
const parentDiv = document.getElementsByClassName("notifications")[0];
const noTelemetryElement = document.querySelector('.none[data="NoTelemetry"]');

let isDragging = false;
let startY = 0;

if (handleElement && parentDiv) {
  handleElement.addEventListener("touchstart", (e) => {
    isDragging = true;
    startY = e.touches[0].clientY - parentDiv.offsetTop;
    parentDiv.style.transition = "none";
  });

  document.addEventListener(
    "touchmove",
    (e) => {
      const notifyCheckbox = document.getElementById("notify");

      if (notifyCheckbox && !noTelemetryElement && notifyCheckbox.checked) {
        e.preventDefault();
      }
      if (isDragging) {
        const newY = e.touches[0].clientY - startY;
        parentDiv.style.top = `${Math.max(newY, window.innerHeight * 0.2)}px`;
      }
    },
    { passive: false },
  );

  document.addEventListener("touchend", () => {
    isDragging = false;
    parentDiv.style.transition = "all 0.3s ease-in-out";

    const notifyCheckbox = document.getElementById("notify");
    if (
      notifyCheckbox &&
      parseInt(parentDiv.style.top) > window.innerHeight / 2
    ) {
      notifyCheckbox.checked = false;
    }
    parentDiv.style.top = "";
  });
}
