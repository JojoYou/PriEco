<?php
ini_set('session.cookie_domain', '.jojoyou.org');
session_start();
include 'Controller/simple_html_dom.php';

$gTime = microtime(true);

//Development mode (Get search results from json files in ./Controller/dev folder)
$dev = false;
//CSS version
$cssver = 63;
//Variable, controls reloading on settings change
$reload = false;

if(file_exists('disGoogle.txt')){
  $tmp = file_get_contents('disGoogle.txt');
  if($tmp < date('d') && date('H') >= 7){
    unlink('disGoogle.txt');
  }
}
if(file_exists('disImg.txt')){
  $tmp = explode(' ', file_get_contents('disImg.txt'));
  if($tmp[0] < date('d') && date('H') >= $tmp[1]){
    unlink('disImg.txt');
  }
}

  //Get data from $PromoFile
  $promoobj = json_decode(file_get_contents('./Controller/value/data.json'), true);


//Prepare for search request with search engine APIs
include './Controller/database.php';
##
#Protection
##
if(!$dev){
include 'Controller/functions/protection/cookie.php';
include 'Controller/functions/protection/ip.php';}

//Function to get string between characters
include 'Controller/functions/getsearch.php';
//Check for img search and for empty query
$urlSet = explode('&', $urlSet);
for ($i = 1; $i < count($urlSet); $i++) {
  if(strpos($urlSet[$i], 'q=') !== false && !isset($_COOKIE['hQuery'])){
    $purl = urldecode(str_replace('q=', '', $urlSet[$i]));
  }
  if(!isset($_COOKIE['hQuery'])){
  if ($urlSet[$i] == 'image') {
    $type = 'image';
  }
  if ($urlSet[$i] == 'video') {
    $type = 'video';
  }
  if ($urlSet[$i] == 'news') {
    $type = 'news';
  }
  }
  
  if(strpos($urlSet[$i], 'page=') !== false){
    $page = str_replace('page=', '', $urlSet[$i]);
  }
  if (strpos($urlSet[$i], 'imgsize=') !== false) {
    $imgsize = str_replace('imgsize=', '', $urlSet[$i]);
  }
  if (strpos($urlSet[$i], 'imgcolor=') !== false) {
    $imgcolor = str_replace('imgcolor=', '', $urlSet[$i]);
  }
  if (strpos($urlSet[$i], 'imgtype=') !== false) {
    $imgtype = str_replace('imgtype=', '', $urlSet[$i]);
  }
  if (strpos($urlSet[$i], 'imgtime=') !== false) {
    $imgtime = str_replace('imgtime=', '', $urlSet[$i]);
  }
  if (strpos($urlSet[$i], 'imglicence=') !== false) {
    $imgright = str_replace('imglicence=', '', $urlSet[$i]);
  }
  if (strpos($urlSet[$i], 'pixabay') !== false) {
    $pixabay = true;
  }
}

if(!isset($page)){
  $page = 0;
}

if ((isset($_POST['q']) && $_POST['q'] != $purl) && !isset($_COOKIE['hQuery'])) {
  header('Location: ./?' . $type . '&q=' . urlencode($_POST['q']));
  exit();
}

//Bangs
if (strpos($purl, '!') !== false) {
  $bangData = file_get_contents('Controller/value/bangs.json');
  $bangObj = json_decode($bangData, true);
  $bangsSplit = explode(' ', $purl);
  foreach ($bangsSplit as $bgS) {
    if (strpos($bgS, '!') !== false) {
      $bgS = str_replace('!', '', $bgS);
      $i = 0;
      foreach ($bangObj['bangs'] as $name => $value) {
        if ($name == $bgS) {
          $j = 0;
          foreach ($bangObj['bangs'] as $pBang) {
            if ($j == $i) {
              $Rurl = str_replace('!' . $name . ' ', '', $purl);
              header('Location: ' . $pBang . $Rurl);
              exit();
            } else {
              $j++;
            }
          }
        }
        $i++;
      }
    }
  }
}

    //Set parameters for search request
    $lang = 'all'; $loc = 'all';
    if (isset($_COOKIE['Language'])) {
      $lang = $_COOKIE['Language'];
    }
    if (isset($_COOKIE['Location'])) {
      $loc = $_COOKIE['Location'];
    }
    
    if (!$dev && !str_starts_with(getenv('REMOTE_ADDR'), '192.') && !str_starts_with(getenv('REMOTE_ADDR'), '127.') && (!isset($_COOKIE['Location']) || !isset($_COOKIE['Language']))) {
     
    $ch = curl_init();

    curl_setopt($ch, CURLOPT_RETURNTRANSFER, 1);

    curl_setopt($ch, CURLOPT_URL, 'https://ipapi.co/'.getenv('REMOTE_ADDR').'/json/');
    curl_setopt($ch, CURLOPT_USERAGENT, 'Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13');

    $geo =json_decode(curl_exec($ch), true);

      if(!isset($_COOKIE['Location'])){
        if(isset($geo['country'])){
        setcookie('Location', strtolower(strtolower($geo['country'])), time() + 604800, '/');
        }
        else{
          setcookie('Location', 'all', time() + 604800, '/');
        }
      }
      if(!isset($_COOKIE['Language'])){
        $ch = curl_init();

        curl_setopt($ch, CURLOPT_RETURNTRANSFER, 1);
    
        curl_setopt($ch, CURLOPT_URL, 'https://restcountries.com/v3.1/alpha/'.$geo['country']);
        curl_setopt($ch, CURLOPT_USERAGENT, 'Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13');
    
        $newlang = json_decode(curl_exec($ch), true);
        if(isset($newlang[0]['languages'])){
        $newlang =$promoobj['lang'][0][reset($newlang[0]['languages'])];
        }
        else{
          unset($newlang);
        }
        if(isset($newlang)){
          setcookie('Language', strtolower($newlang), time() + 604800, '/');
        }
          else{
            setcookie('Language', 'all', time() + 604800, '/');
          }
      }
      $reload = true;
    }

    $fahrenheitCountries = ['as', 'bs', 'bz', 'ca', 'fm', 'gu', 'mh', 'mp', 'pr', 'pw', 'tc', 'us', 'vi'];
    if(!isset($_COOKIE['temp']) && in_array($_COOKIE['Location'], $fahrenheitCountries)){setcookie('temp', 'f', time() + 604800, '/');}

    if(isset($_COOKIE['safe'])){$safe = $_COOKIE['safe'];}
    else{$safe = 'active';}

    if(isset($_COOKIE['new'])){$new = $_COOKIE['new'];}
    else{$new = 'off';}

    if (!isset($imgsize)){$imgsize = 'all';}
    if (!isset($imgcolor)){$imgcolor = 'all';}
    if (!isset($imgtime)){$imgtime = 'all';}
    if (!isset($imgtype)){$imgtype = 'all';}
    if (!isset($imgright)){$imgright = 'all';}

    if(isset($_COOKIE['time'])){
    switch ($_COOKIE['time']):
      case 'day':
        $date = 'd1';
        break;
      case 'week':
        $date = 'w1';
        break;
      case 'month':
        $date = 'm1';
        break;
      case 'year':
        $date = 'y1';
        break;
    endswitch;
  }
  else{
    $date = '';
  }

    include 'Model/header.php';
##
#Analytics
##
if(!$dev){include('Controller/functions/analytics/analytics.php');}

#IndexLogic
include 'Controller/functions/indexLogic.php';

//Get style.css file and post custom settings for light and dark mode
  include 'Model/style.php';
  
  if ($purl != null or $purl != '') {
  include 'Controller/retrieve.php';
  //indexLogic is here for saving settings
    //Get user from cookie and compare it with database
    if (isset($_COOKIE['auth']) && !$dev) {  
      $auth = '▛' . $auth;
  
      $possibleUsr = strstr($auth, '▛');
      $possibleUsr = strstr($auth, ' ', true);
      $possibleUsr = substr($possibleUsr, 3);
  
      $authName = str_replace('▛' . $possibleUsr . ' ', '', $auth);
  
  
      $sql = "SELECT * FROM JYS WHERE Username='$possibleUsr'";
      $result = $conn->query($sql);
      if ($result->num_rows > 0) {
        $row = $result->fetch_assoc();
        $authDB = explode(',', $row['Auth']);
        foreach ($authDB as $a) {
          if ($a == $authName) {
            $usr = $row['Username'];
            $usrSearches = $row['Searches'];
            $_SESSION['usr'] = $usr;
            continue;
          }
        }
      }
      $usr = $row['Username'];
      $select_user = "SELECT * FROM JYS WHERE Username='$usr'";
      $run_qry = mysqli_query($conn, $select_user);
      if (mysqli_num_rows($run_qry) > 0) {
        if ($row = mysqli_fetch_assoc($run_qry)) {
          $usrSearches = $row['Searches'];
          $usrSearches++;
          $update_user = "UPDATE JYS SET Searches='$usrSearches' WHERE Username='$_SESSION[usr]'";
          $run_qry = mysqli_query($conn, $update_user);
        }
      }
    }
    if(!isset($usr)){
      $usr = 'Guest';
    }
  //Create inputbox with settings
  include 'Model/searchbox.php';
  //Print output
  $priecoTime = -1;
  $resultTime = microtime(true);
  include 'Controller/functions/output.php';
  $resultTime = microtime(true) - $resultTime;
  //Improve PriEco
  if(isset($_COOKIE['improvePriEco']) && !$dev){
  include 'Controller/functions/analytics/improve.php';
  }
  //Print footer, contains JS for Ads
  include 'Model/footer.php';
  
  if(isset($_COOKIE['showtime'])){
    echo '<div class="showtime">
    <h4>Time to load:</h4>
    <p>Full time: ', (microtime(true) - $gTime),' s<br>
    Together results time: ', $resultTime ,' s<br>
    PriEco results time: ', $priecoTime ,' s
    </p></div>';
  }

  //Increase daily searches
  if(!$dev){
  $currentDate = date('Y-m-d');
  $sql = "SELECT * FROM users WHERE day = '$currentDate'";
  $result = $conn->query($sql);
  
  if ($result->num_rows > 0) {
      $sql = "UPDATE users SET searches = searches + 1 WHERE day = '$currentDate'";
      $conn->query($sql);
  } else {
      $sql = "INSERT INTO users (day, searches) VALUES ('$currentDate', 1)";  
      $conn->query($sql);
  }
  }
} else {  
  //Print mainpage
  include 'Model/mainpage.php';
    //Print footer, contains JS for Ads
    include 'Model/footer.php';
}
?>