<?php
  //Get data from $PromoFile
  $promodata = file_get_contents('./Controller/value/data.json');
  $promoobj = json_decode($promodata, true);
$langName = [""];
$langVal = [""];
foreach ($promoobj['lang'][0] as $location) {
  array_push($langVal, $location);
}
foreach ($promoobj['lang'][0] as $name => $value) {
  array_push($langName, $name);
}

if (isset($_COOKIE['auth']) && !$dev) {
  ini_set('session.cookie_domain', '.jojoyou.org');
  session_start();

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

if (isset($_COOKIE['auth'])) {
  if (!isset($_SESSION)) {
  ini_set('session.cookie_domain', '.jojoyou.org');
  session_start();
  }

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
include './Controller/functions/indexLogic.php';

echo '

<button class="tree-btn" id="tree-btn"><img alt="icntree" src="./View/icon/user.svg" style="width:30px;height:30px;"><p style="font-weight:bold;">'
, $usr , '</p></button>
  <br>
  ';
  include 'settings.php';
?>






<div class="autocomplete">
<div style="margin: 0;
    position: absolute;
    top: 43%;
    left: 50%;
    transform: translate(-50%, -125px);z-index:10;width:clamp(0px, 559px, 100%);text-align:center;">

    <img id="LandSLogo" alt="PriEcoLogo" style="height:100px;width: 83.33px;"
      src="./View/img/PriEco.webp" />
    <span style="font-size: 500%;font-weight:bold;width:40%;padding-right:30px;  background: -webkit-linear-gradient(#03D781, #3EDCE2);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;">PriEco</span>

  <form id="searchForm" class="searchBarMain" method="post" style="margin-left:15px;">
    <input
    list="suggestions"
    id="searchBox"
    placeholder="PriEco"
      value=""
      class="searchBox"
      name="q"
      size="21"
      autofocus autocomplete="off"
      style="height: 50px;
width: calc(90% - 10px);
padding-left: 10px;margin-left:unset;
min-width: 200px;box-shadow: 0 2px 6px rgba(0, 0, 0, 0.1);"
    /><button id="searchButton" aria-label="search button" style="width:10%;height:50px;box-shadow: 0 2px 6px rgba(0, 0, 0, 0.1);" class="searchButtonMain searchButton">
      <img src="./View/icon/search.webp" alt="‎" style="width:10px;height:10px;"></button
    >
  </form>
  <div class="autoboxMain autocom-box" style="width: calc(90% - 30px);min-width: 200px;position:absolute;margin-left: 25px;margin-top:0;"></div>
  <p style="padding-top: 10px;
  text-align: center;
  width: 95%;font-style:italic;">Search 🙈 privately and 🌲 ecofriendly</p>

<div class="shortcuts">
<?php
if(isset($_COOKIE['shortcuts'])){
$shortcuts = explode(',', urldecode($_COOKIE['shortcuts']));
$i =0;
foreach($shortcuts as &$shs){
  ++$i;
  if($i == 1){continue;}
$sh = explode('=',$shs);
echo '<div>

<div>
<button class="shortcutEditBtn',$i,' shortcutEditBtn">•</button>
<a href="',$sh[1],'" class="shortcutBtn',$i,'" style="text-decoration:none;color:black;">
<div tabindex="0" class="shortcutElemenet" style="background-image: url(/Controller/functions/proxy.php?q=https://judicial-peach-octopus.b-cdn.net/'. parse_url($sh[1])['host']. ');"></div>
</a>
<div class="shortcutEditForm',$i,' shortcutEditForm shortcutForm">
<form method="post">
<input type="hidden" name="shortcutID" value="',$i,'">
<input type="submit" name="shortcutDelete" value="Delete" style="background-color:red;margin-bottom:20px;cursor:pointer;">
</form>
<p><b>Edit</b></p>
<p>',$sh[0],'</p>
<form method="post">
<input type="hidden" name="shortcutID" value="',$i,'">
<input type="text" name="shortcutName" placeholder="Name" value="',$sh[0],'" required>
<input type="text" name="shortcutURL" placeholder="URL" value="',$sh[1],'" required>
<input type="submit" name="shortcutEdit" value="Edit" style="cursor:pointer;">
</form>
</div>
<div>
<p>',$sh[0],'</p>
</div>
</div>
</div>

<style>
.shortcutEditBtn',$i,':focus ~ .shortcutEditForm',$i,'{
  display: flex;
}
</style>';

}
}
?>

<button id="addShortcutBtn" class="shortcutElemenet">+</button>
<div class="addShortcut shortcutForm">
<p><b>Add shortcut</b></p>
<form method="POST">
<input type="text" name="shortcutName" placeholder="Name" required>
<input type="text" name="shortcutURL" placeholder="URL" required>
<input type="submit" name="shortcutSubmit" value="Add" style="cursor:pointer;">
</form>
</div>
  </div>
</div>
</div>

  <div style="height: 98vh;width:auto; background-image: url('/View/img/wbg.webp');background-repeat: no-repeat;
background-position: center;
background-size: cover;"></div>

<div style="width:auto;height:<?php echo ($i-1)/4*111;?>px"></div>

<style>
  @media (max-width: 890px) {
    .autocomplete{
      margin-top: -80px;
        }
  }
  .menu-btn{
    position: absolute !important;
    top: 14px !important;
  }
</style>