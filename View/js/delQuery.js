window.addEventListener('load', function() {
    var backButton = document.querySelector('.delQueryBtn');
    backButton.addEventListener('click', function() {
        document.querySelector('.searchBox').value="";
        document.querySelector('.searchBox').focus();
    });
  });
  