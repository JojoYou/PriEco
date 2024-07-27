const revGl = document.querySelector("#revertGlass"); 
const whiteImg = document.querySelector("#whiteImg"); 
revGl.addEventListener('mouseover', (event) => {
  revGl.addEventListener('mousemove', (event) => {
    const newX = event.clientX + revGl.offsetLeft;
    const newY = event.clientY + window.pageYOffset;

    whiteImg.style.mask = `radial-gradient(circle farthest-corner at ${ event.clientX-whiteImg.offsetLeft}px ${event.clientY+ window.pageYOffset-whiteImg.offsetTop}px, transparent 50px, #fff 50px)`;
    whiteImg.style.webkitMask = `radial-gradient(circle farthest-corner at ${ event.clientX-whiteImg.offsetLeft}px ${event.clientY+ window.pageYOffset-whiteImg.offsetTop}px, transparent 50px, #fff 50px)`;

  });
});