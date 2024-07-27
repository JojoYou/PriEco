    function debounce(func, wait) {
  let timeout;
  return function() {
    const context = this;
    const args = arguments;
    clearTimeout(timeout);
    timeout = setTimeout(function() {
      func.apply(context, args);
    }, wait);
  };
}

    <?php 
    header('Content-Type: application/javascript');
    $loc = 'all';
    if(isset($_COOKIE['Location']) && $_COOKIE['Location'] != 'all'){
      $loc = $_COOKIE['Location'];
    }
    if(!isset($_COOKIE['index'])){
      echo 'const sugUrl ="Controller/functions/addons/sug.php/?t='.(isset($_COOKIE['sugProvider']) ? $_COOKIE['sugProvider'] : 'd').'&c='.$loc.'&q=";';
    }
    else{
      echo 'const sugUrl ="Controller/functions/addons/sug.php/?t=p&c='.$loc.'&q=";';
    }
    ?> 
  // Getting all required elements
const searchWrapper = document.querySelector(".searchForm");
const inputBox = searchWrapper.querySelector(".searchBox");
const inputBtn = searchWrapper.querySelector(".searchButton");
const suggBox = searchWrapper.querySelector(".autocom-box");

let backUpInp = "";

// Function to fetch suggestions from the server
async function fetchSuggestions(userData) {
  const response = await fetch(
    sugUrl + userData
  );

  let data = await response.json();

  let emptyArray = [];

  <?php
    echo '
    let w = 0;
    let ar = [];

    let z = 0;
    let adDown = "";
    if (Array.isArray(data)) {
      data.forEach((item) => {
        if (item.ad) {
          if(typeof item.ad === "object"){
          Object.values(item.ad).forEach((ad) => {  
            if(typeof ad === "object"){
            if(z == 1){
              adDown = `<a href="${ad.rurl}" class="clearLink"><li class="suggestion"><img loading="lazy" class="nofilterImage sugImg" src="/Controller/functions/proxy.php?q=${ad.iurl}"><img class="none" loading="lazy" src="/Controller/functions/proxy.php?q=${ad.impurl}">${ad.brand}<img loading="lazy" class="ml-20" src="/View/icon/arrow-trend-up.svg"><img loading="lazy" class="ml-10" src="/View/icon/ad1.svg"></li></a>`;
            }
            else if(z == 0){
            emptyArray.push(
              `<a href="${ad.rurl}" class="clearLink"><li class="suggestion"><img loading="lazy" class="nofilterImage sugImg" src="/Controller/functions/proxy.php?q=${ad.iurl}"><img class="none" loading="lazy" src="/Controller/functions/proxy.php?q=${ad.impurl}">${ad.brand}<img loading="lazy" class="ml-20" src="/View/icon/arrow-trend-up.svg"><img loading="lazy" class="ml-10" src="/View/icon/ad1.svg"></li></a>`   
            );
          }
          z++; 
        }          
        else{
          if(w==4){
            if(z == 1){
              adDown = `<a href="${ar[2]}" class="clearLink"><li class="suggestion"><img loading="lazy" class="nofilterImage sugImg" src="/Controller/functions/proxy.php?q=${ar[3]}"><img class="none" loading="lazy" src="/Controller/functions/proxy.php?q=${ar[4]}">${ar[0]}<img loading="lazy" class="ml-20" src="/View/icon/arrow-trend-up.svg"><img loading="lazy" class="ml-10" src="/View/icon/ad1.svg"></li></a>`;
            }
            else if(z == 0){
            emptyArray.push(
              `<a href="${ar[2]}" class="clearLink"><li class="suggestion"><img loading="lazy" class="nofilterImage sugImg" src="/Controller/functions/proxy.php?q=${ar[3]}"><img class="none" loading="lazy" src="/Controller/functions/proxy.php?q=${ar[4]}">${ar[0]}<img loading="lazy" class="ml-20" src="/View/icon/arrow-trend-up.svg"><img loading="lazy" class="ml-10" src="/View/icon/ad1.svg"></li></a>`   
            );
          }
          z++; 
            w=0;
            ar.length = 0;
          }
         ar.push(ad);
         w++;
        } 
          });
        }
      }
        if (Array.isArray(item)) {';
          if((isset($_COOKIE['sugProvider']) && $_COOKIE['sugProvider'] == 'p') || isset($_COOKIE['index'])){
            echo 'item.slice(0, 10).forEach((element) => {
              var sug =  `<li class="suggestion `;
              if (element.img !== "") { sug += `height-70`;}
              sug = sug + `">`;
              if (element.img !== "") {
                sug = sug + `<img loading="lazy" class="nofilterImage sugImg" src="/Controller/functions/proxy.php?q=${element.img}">`;
              } else {
                sug = sug + `<img loading="lazy" src="/View/icon/search.webp">`;
              }
              sug = sug + `<p>${element.name}</p></li>`;
      
              emptyArray.push(sug);
            });';
          }
          else{
            echo 'item.slice(0, 10).forEach((element) => {
              emptyArray.push(
                `<li class="suggestion"><img loading="lazy" src="/View/icon/search.webp">${element}</li>`
              );
            });';
          }
        echo '}
        });
      emptyArray.push(
        adDown
      );
    }';
  
?>
  // Show the suggestions
  showSuggestions(emptyArray);
}

// Function to show the suggestions in the suggestion box
function showSuggestions(list) {
  let listData;
  if (!list.length) {
    const userValue = inputBox.value;
    listData = `<li class="suggestion"><img loading="lazy" src="/View/icon/search.webp">${userValue}</li>`;
  } else {
    listData = list.join("");
    
  }
  suggBox.innerHTML = listData;

  var suggestionBoxes = document.querySelectorAll('.suggestion');
  suggestionBoxes.forEach(suggBox => {
    suggBox.addEventListener('click', function(event) {
        inputBox.value = event.target.textContent;
        const form = document.getElementById("searchForm");
        form.submit();
    });
  }); 
  inputBox.classList.add("hasSuggestions");
  inputBtn.classList.add("hasSuggestions");
}

const debouncedFetchSuggestions = debounce(fetchSuggestions, 300);

// Event handler when the user releases a key in the input box
inputBox.onkeyup = (e) => {
  const keyCode = e.keyCode;
  const userData = e.target.value.trim();

  if (userData) {
    if (keyCode === 40 || keyCode === 38) {
      // Handle arrow key navigation
      const active = suggBox.querySelector(".active");
      if (active) {
        const next = keyCode === 40 ? active.nextElementSibling : active.previousElementSibling;
        if (next) {
          active.classList.remove("active");
          next.classList.add("active");
        } else {
          setTimeout(() => {
            inputBox.value = backUpInp;
          }, 200);
        }
      } else {
        const firstElement = keyCode === 40 ? suggBox.firstElementChild : suggBox.lastElementChild;
        firstElement.classList.add("active");
      }
      inputBox.value = suggBox.querySelector(".active").textContent.replace('<img loading="lazy" src="/View/icon/search.webp">', '');
    } else if (keyCode === 13) {
      // Handle enter key to submit the form
      const form = document.getElementById("searchForm");
      form.submit();
    } else {
      // Fetch suggestions from the server
      backUpInp = inputBox.value;
      fetchSuggestions(userData);
    }
  } else {
    // Delayed clearing of suggestion box
    setTimeout(() => {
      suggBox.innerHTML = "";
      inputBox.classList.remove("hasSuggestions");
    inputBtn.classList.remove("hasSuggestions");
    }, 400);
  }
};

// Event handler when the input box gets focus
inputBox.onfocus = () => {
  const inputValue = inputBox.value.trim();
  if (inputValue) {
    backUpInp = inputValue;
    fetchSuggestions(inputValue);
  }
};

// Event handler when the input box loses focus
inputBox.onblur = () => {
  setTimeout(() => {
    suggBox.innerHTML = "";
    inputBox.classList.remove("hasSuggestions");
    inputBtn.classList.remove("hasSuggestions");
  }, 200);
};

// Event handler when a suggestion is selected
function select(element) {
  const selectData = element.textContent;
  inputBox.value = selectData;
  const form = document.getElementById("searchForm");
  form.submit();
}