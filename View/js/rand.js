function generateRandomNumber() {
    var from = parseInt(document.getElementById("from").value);
    var to = parseInt(document.getElementById("to").value);
    
    if (isNaN(from) || isNaN(to)) {
      document.getElementById("rnNum").innerHTML = "Invalid input";
    } else if (from >= to) {
      document.getElementById("rnNum").innerHTML = "From value must be less than To value";
    } else {
      var randomNumber = Math.floor(Math.random() * (to - from + 1)) + from;
      document.getElementById("rnNum").innerHTML = randomNumber;
    }
  }

  button = document.querySelector("#rnGen");
  button.addEventListener('click', () => {
    generateRandomNumber();
});