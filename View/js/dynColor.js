const elements = document.querySelectorAll('.dynamicBG, .dynamicColor');

for (const element of elements) {
  if (element.classList.contains('dynamicBG')) {
    element.style.backgroundColor = `${$dominantColor}78`;
  }

  if (element.classList.contains('dynamicColor')) {
    element.style.color = `${$negativeColor}`;
  }
}
