<?php
echo'
<link rel="stylesheet" href="',$beforePathStyle,'css/universal.css?v=',$cssver,'">
<link rel="stylesheet" href="',$beforePathStyle,'css/style.css?v=',$cssver,'">
<link rel="stylesheet" href="',$beforePathStyle,'css/mobileStyle.css?v=',$cssver,'">';

if((!isset($_GET['q']) || $_GET['q'] == '') && (!isset($_COOKIE['hQuery']) || !isset($_POST['q']))){
  echo '<link rel="stylesheet" href="',$beforePathStyle,'css/landing.css?v=',$cssver,'">';
}
if(!isset($_COOKIE['mode'])){
  //Auto mode
  echo '<link rel="stylesheet" href="',$beforePathStyle,'css/system.css?v=',$cssver,'">';
}
elseif($_COOKIE['mode'] == 1){
//Light mode
    echo '<link rel="stylesheet" href="',$beforePathStyle,'css/light.css?v=',$cssver,'">';
}
elseif($_COOKIE['mode'] == 2){
  //Dark mode
    echo '<link rel="stylesheet" href="',$beforePathStyle,'css/dark.css?v=',$cssver,'">';    
}
else{
  //Custom mode
      echo '<style>', $_COOKIE['theme'], '</style>';
}
?>
