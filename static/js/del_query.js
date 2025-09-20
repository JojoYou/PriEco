window.addEventListener("load", function () {
  var backButton = document.querySelector(".btn_del_query");
  backButton.addEventListener("click", function () {
    document.querySelector(".search_box").value = "";
    document.querySelector(".search_box").focus();
  });
});
