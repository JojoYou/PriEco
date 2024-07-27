function animTypeAdd(sumResOutElement){
    if(!sumResOutElement.classList.contains('typeAnim')){
        sumResOutElement.classList.add('typeAnim');

        setTimeout(() => {
            sumResOutElement.classList.remove('typeAnim');
        }, 1000);
          
    }
}

document.addEventListener('DOMContentLoaded', () => {
const sumResElements = document.querySelectorAll('img#sumRes');

sumResElements.forEach(element => {
  element.addEventListener('click', () => {
    const parent = element.parentElement.parentElement;
    console.log(parent);
    const sumResOutElement = parent.querySelector("#sumResOut");
    const sumOutElement = sumResOutElement.querySelector('#sumOut');
    console.log(sumResOutElement);

    if (sumOutElement.textContent === '') {
        sumOutElement.textContent = 'Loading...';
      animTypeAdd(sumResOutElement);
    const url = element.getAttribute('data-url');
    const apiUrl = "api/summarize.php/?url=" + url;
    const text = fetch(apiUrl)
    .then(responseData => responseData.json())
    .then(data => {
      if (data.summary === '') {
        sumOutElement.textContent = 'Couldn\'t summarize';
      } else {
let sl = false;
if(data.ssl == '443'){
    sl = true;
}
if (window.screen.width <= 890 && data.summary.length > 86) {
  data.summary = data.summary.substring(0, 86) + '...';
}
        sumOutElement.textContent = data.summary;
      }
      animTypeAdd(sumResOutElement);
    });
    
  }

  const sibling = parent.querySelector('#snippet');

if (sibling.style.display == 'none') {
    sibling.style.display = 'block';
    sumResOutElement.style.display = 'none';
} else {
  
    sumResOutElement.style.display = 'block';
    sibling.style.display = 'none';
    animTypeAdd(sumResOutElement);

}
   
  });
});
});