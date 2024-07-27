const bangsElement = document.querySelector('#bangs');
if (bangsElement) {
  const bangsString = bangsElement.getAttribute('bangs').toString().split(';');
  const queryString = bangsElement.getAttribute('query');

for (let i = 0; i < bangsString.length-1; i++) {
    if(i == bangsString.length-2){
        setTimeout(() => {
            window.location.href = `${bangsString[i]}${queryString}`;
          }, 7000);
        continue;
    }
      window.open(`${bangsString[i]}${queryString}`, '_blank');
  }
}
