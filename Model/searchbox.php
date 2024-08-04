<?php
$langName = [""];
$langVal = [""];
foreach ($promoobj['lang'][0] as $location) {
  array_push($langVal, $location);
}
foreach ($promoobj['lang'][0] as $name => $value) {
  array_push($langName, $name);
}
$locName = [""];
$locVal = [""];
foreach ($promoobj['loc'][0] as $location) {
  array_push($locVal, $location);
}
foreach ($promoobj['loc'][0] as $name => $value) {
  array_push($locName, $name);
}


echo '
<div class="topbg" id="topbg">
<a href="/" class="link">', (isset($_COOKIE['datasave']) ? '<h1 class="sLogo">P</h1>' : '<img class="sLogo" alt="TreeLogo" src="./View/img/PriEco.webp">'),'</a>
  <div class="searchM">
  <form class="searchForm" id="searchForm" method="post" >

  <div class="flex">
    <button id="searchButton" class="searchButton">', (isset($_COOKIE['datasave']) ? '<p>➤</p>' : '<img alt="icnSearch" src="./View/icon/search.webp" width="13" height="13">'),'</button>
    <input type="hidden" name="search_type" value="'.$type.'">
      <input list="suggestions"
       id="searchBox"
       value="' , htmlspecialchars($purl, ENT_QUOTES | ENT_HTML5, 'UTF-8'), '"
       class="searchBox"
       name="q"
       size="21"
       placeholder="PriEco"
       autocomplete="off">
    ', (isset($_COOKIE['DisQue']) ? '' : '<button class="delQueryBtn" type="button"><img src="View/icon/cross.svg" class="width10 height10 opacity7"></button>'),
  '</div>
  <div class="autocom-box"></div>
  </form>
<div class="settingButtons">
<form method="POST" action="">
<input type="hidden" name="search_type" value="">
<input type="hidden" name="q" value="', $purl ,'">
<button class="';
if ($type != 'image' && $type != 'video' && $type !='news' && $type != 'shop') {
  echo 'settingBtnsActive ';
}
echo 'allBut settingButton" ';
if(!isset($_COOKIE['hQuery'])){
  echo 'name="allBut" id="allbut"';
}
echo '>';
  if(!isset($_COOKIE['datasave'])){
    echo '<img src="./View/icon/earth.svg" class="setBut">';
  }
  echo '<p>';
  if($lang == null || $lang == 'en' || $lang == 'all'){echo 'All';}
  else{echo $promoobj['All'][0][$lang];}
  echo '</p></button>
  </form>


  <form method="POST" action="">
<input type="hidden" name="search_type" value="image">
<input type="hidden" name="q" value="', $purl ,'">
<button class="';
if ($type == 'image') {
  echo 'settingBtnsActive ';
}
echo 'settingButton" ';
if(!isset($_COOKIE['hQuery'])){
  echo 'name="imgBut" id="imgbut"';
}
echo '>';
if(!isset($_COOKIE['datasave'])){
  echo '<img src="./View/icon/image.svg" alt="" class="setBut">';
}
echo '<p>';
  if($lang == null || $lang == 'en' || $lang == 'all'){echo 'Images';}
  else{echo $promoobj['Images'][0][$lang];}
  echo '</p></button></form>

  <form method="POST" action="">
<input type="hidden" name="search_type" value="video">
<input type="hidden" name="q" value="', $purl ,'">
  <button class="';
  if ($type == 'video') {
    echo 'settingBtnsActive ';
  }
  echo ' settingButton"';
  if(!isset($_COOKIE['hQuery'])){
    echo 'name="videoBut"';
  }
  echo '>';
  if(!isset($_COOKIE['datasave'])){
    echo '<img src="./View/icon/video.svg" alt="" class="setBut">';
  }
  echo '<p>';
    if($lang == null || $lang == 'en' || $lang == 'all'){echo 'Videos';}
    else{echo $promoobj['Videos'][0][$lang];}
    echo '</p></button></form>

    <form method="POST" action="">
    <input type="hidden" name="search_type" value="news">
    <input type="hidden" name="q" value="', $purl ,'">
      <button class="';
      if ($type == 'news') {
        echo 'settingBtnsActive ';
      }
      echo 'settingButton"';
      if(!isset($_COOKIE['hQuery'])){
        echo 'name="newsBut"';
      }
      echo '>';
      if(!isset($_COOKIE['datasave'])){
        echo '<img src="./View/icon/news.svg" alt="" class="setBut">';
      }
        if($lang == null || $lang == 'en' || $lang == 'all'){echo 'News';}
        else{echo $promoobj['News'][0][$lang];}
        echo '</button></form>

    <form method="POST" action="">
    <input type="hidden" name="search_type" value="shop">
    <input type="hidden" name="q" value="', $purl ,'">
  <button class="';
  if ($type == 'shop') {
    echo 'settingBtnsActive ';
  }
  echo 'settingButton"';
  if(!isset($_COOKIE['hQuery'])){
    echo 'name="shopBut"';
  }
  echo '>';
  if(!isset($_COOKIE['datasave'])){
    echo '<img src="./View/icon/shop.svg" alt="" class="setBut">';
  }
    if($lang == null || $lang == 'en' || $lang == 'all'){echo 'Shopping';}
    else{echo $promoobj['Shopping'][0][$lang];}
    echo '</button></form>
<a class="link" href="map.php?q=',$purl,'"><button class="settingButton" id="mapbut">';
if(!isset($_COOKIE['datasave'])){
  echo '<img src="./View/icon/map.svg" alt="" class="setBut">';
}
  if($lang == null || $lang == 'en' || $lang == 'all'){echo 'Map';}
  else{echo $promoobj['Map'][0][$lang];}
  echo '</button></a>
  </form>

<a href="/feedback.php" target="_blank" class="feedbackMenuLabel">';
if(!isset($_COOKIE['datasave'])){
echo '<img src="./View/icon/feedback.webp" alt="feedback" >';
}
else{
  echo '<p class="settingButton">Feedback</p>';
}

$notifications = json_decode(file_get_contents('notify.json'), true);
$j = count($notifications);

$t = isset($_COOKIE['notify']) ? $_COOKIE['notify'] : 0;

$not = false;
foreach($notifications as $date){
  if (is_numeric($date['date']) && $date['date'] > $t) {
    $t = $date['date'];
    $not = true;
    break;
  }
}

echo '</a>

<label for="threedotsquick" class="labelforcheckquick feedbackMenuLabel"><img alt="quick settings" src="./View/icon/sliders.svg"></label>
  </div>
  </div>

  <label for="notify" class="notify">
  <div class="Pointer filterImage width15 height15 absolute right60 top10">
  <img src="View/icon/bell.svg" class="width100P height100P">';
  if($not){echo '<section></section>';}
  echo '</div>
  </label>
</div>
</form>
';

include 'settings.php';

echo '
<input type="checkbox" id="threedotsquick" hidden>
<div class="topspace"></div>
<form method="post" class="quickSettingButtons">
<select name="LocDropDown" class="left9vw quickSet">
<option disabled selected hidden>';
if ($loc !== null) {
  $i = array_search($loc, $locVal);
  echo $locName[$i];
}
echo '
  </option>
  <option value="all"';
if ($loc == 'all' || $loc == null) {
  echo 'selected';
}
echo '>All regions</option>

  <option value="af">Afghanistan</option>
  <option value="al">Albania</option>
  <option value="dz">Algeria</option>
  <option value="as">American Samoa</option>
  <option value="ad">Andorra</option>
  <option value="ao">Angola</option>
  <option value="ai">Anguilla</option>
  <option value="aq">Antarctica</option>
  <option value="ag">Antigua and Barbuda</option>
  <option value="ar">Argentina</option>
  <option value="am">Armenia</option>
  <option value="aw">Aruba</option>
  <option value="au">Australia</option>
  <option value="at">Austria</option>
  <option value="az">Azerbaijan</option>
  <option value="bs">Bahamas</option>
  <option value="bh">Bahrain</option>
  <option value="bd">Bangladesh</option>
  <option value="bb">Barbados</option>
  <option value="by">Belarus</option>
  <option value="be">Belgium</option>
  <option value="bz">Belize</option>
  <option value="bj">Benin</option>
  <option value="bm">Bermuda</option>
  <option value="bt">Bhutan</option>
  <option value="bo">Bolivia</option>
  <option value="ba">Bosnia and Herzegovina</option>
  <option value="bw">Botswana</option>
  <option value="bv">Bouvet Island</option>
  <option value="br">Brazil</option>
  <option value="io">British Indian Ocean Territory</option>
  <option value="bn">Brunei Darussalam</option>
  <option value="bg">Bulgaria</option>
  <option value="bf">Burkina Faso</option>
  <option value="bi">Burundi</option>
  <option value="kh">Cambodia</option>
  <option value="cm">Cameroon</option>
  <option value="ca">Canada</option>
  <option value="cv">Cape Verde</option>
  <option value="ky">Cayman Islands</option>
  <option value="cf">Central African Republic</option>
  <option value="td">Chad</option>
  <option value="cl">Chile</option>
  <option value="cn">China</option>
  <option value="cx">Christmas Island</option>
  <option value="cc">Cocos (Keeling) Islands</option>
  <option value="co">Colombia</option>
  <option value="km">Comoros</option>
  <option value="cg">Congo</option>
  <option value="cd">Congo, the Democratic Republic of the</option>
  <option value="ck">Cook Islands</option>
  <option value="cr">Costa Rica</option>
  <option value="ci">Cote D`ivoire</option>
  <option value="hr">Croatia</option>
  <option value="cu">Cuba</option>
  <option value="cy">Cyprus</option>
  <option value="cz">Czech Republic</option>
  <option value="dk">Denmark</option>
  <option value="dj">Djibouti</option>
  <option value="dm">Dominica</option>
  <option value="do">Dominican Republic</option>
  <option value="ec">Ecuador</option>
  <option value="eg">Egypt</option>
  <option value="sv">El Salvador</option>
  <option value="gq">Equatorial Guinea</option>
  <option value="er">Eritrea</option>
  <option value="ee">Estonia</option>
  <option value="et">Ethiopia</option>
  <option value="fk">Falkland Islands (Malvinas)</option>
  <option value="fo">Faroe Islands</option>
  <option value="fj">Fiji</option>
  <option value="fi">Finland</option>
  <option value="fr">France</option>
  <option value="gf">French Guiana</option>
  <option value="pf">French Polynesia</option>
  <option value="tf">French Southern Territories</option>
  <option value="ga">Gabon</option>
  <option value="gm">Gambia</option>
  <option value="ge">Georgia</option>
  <option value="de">Germany</option>
  <option value="gh">Ghana</option>
  <option value="gi">Gibraltar</option>
  <option value="gr">Greece</option>
  <option value="gl">Greenland</option>
  <option value="gd">Grenada</option>
  <option value="gp">Guadeloupe</option>
  <option value="gu">Guam</option>
  <option value="gt">Guatemala</option>
  <option value="gn">Guinea</option>
  <option value="gw">Guinea-Bissau</option>
  <option value="gy">Guyana</option>
  <option value="ht">Haiti</option>
  <option value="hm">Heard Island and Mcdonald Islands</option>
  <option value="va">Holy See (Vatican City State)</option>
  <option value="hn">Honduras</option>
  <option value="hk">Hong Kong</option>
  <option value="hu">Hungary</option>
  <option value="is">Iceland</option>
  <option value="in">India</option>
  <option value="id">Indonesia</option>
  <option value="ir">Iran, Islamic Republic of</option>
  <option value="iq">Iraq</option>
  <option value="ie">Ireland</option>
  <option value="il">Israel</option>
  <option value="it">Italy</option>
  <option value="jm">Jamaica</option>
  <option value="jp">Japan</option>
  <option value="jo">Jordan</option>
  <option value="kz">Kazakhstan</option>
  <option value="ke">Kenya</option>
  <option value="ki">Kiribati</option>
  <option value="kp">Korea, Democratic People`s Republic of</option>
  <option value="kr">Korea, Republic of</option>
  <option value="kw">Kuwait</option>
  <option value="kg">Kyrgyzstan</option>
  <option value="la">Lao People`s Democratic Republic</option>
  <option value="lv">Latvia</option>
  <option value="lb">Lebanon</option>
  <option value="ls">Lesotho</option>
  <option value="lr">Liberia</option>
  <option value="ly">Libyan Arab Jamahiriya</option>
  <option value="li">Liechtenstein</option>
  <option value="lt">Lithuania</option>
  <option value="lu">Luxembourg</option>
  <option value="mo">Macao</option>
  <option value="mk">Macedonia, the Former Yugosalv Republic of</option>
  <option value="mg">Madagascar</option>
  <option value="mw">Malawi</option>
  <option value="my">Malaysia</option>
  <option value="mv">Maldives</option>
  <option value="ml">Mali</option>
  <option value="mt">Malta</option>
  <option value="mh">Marshall Islands</option>
  <option value="mq">Martinique</option>
  <option value="mr">Mauritania</option>
  <option value="mu">Mauritius</option>
  <option value="yt">Mayotte</option>
  <option value="mx">Mexico</option>
  <option value="fm">Micronesia, Federated States of</option>
  <option value="md">Moldova, Republic of</option>
  <option value="mc">Monaco</option>
  <option value="mn">Mongolia</option>
  <option value="ms">Montserrat</option>
  <option value="ma">Morocco</option>
  <option value="mz">Mozambique</option>
  <option value="mm">Myanmar</option>
  <option value="na">Namibia</option>
  <option value="nr">Nauru</option>
  <option value="np">Nepal</option>
  <option value="nl">Netherlands</option>
  <option value="an">Netherlands Antilles</option>
  <option value="nc">New Caledonia</option>
  <option value="nz">New Zealand</option>
  <option value="ni">Nicaragua</option>
  <option value="ne">Niger</option>
  <option value="ng">Nigeria</option>
  <option value="nu">Niue</option>
  <option value="nf">Norfolk Island</option>
  <option value="mp">Northern Mariana Islands</option>
  <option value="no">Norway</option>
  <option value="om">Oman</option>
  <option value="pk">Pakistan</option>
  <option value="pw">Palau</option>
  <option value="ps">Palestinian Territory, Occupied</option>
  <option value="pa">Panama</option>
  <option value="pg">Papua New Guinea</option>
  <option value="py">Paraguay</option>
  <option value="pe">Peru</option>
  <option value="ph">Philippines</option>
  <option value="pn">Pitcairn</option>
  <option value="pl">Poland</option>
  <option value="pt">Portugal</option>
  <option value="pr">Puerto Rico</option>
  <option value="qa">Qatar</option>
  <option value="re">Reunion</option>
  <option value="ro">Romania</option>
  <option value="ru">Russian Federation</option>
  <option value="rw">Rwanda</option>
  <option value="sh">Saint Helena</option>
  <option value="kn">Saint Kitts and Nevis</option>
  <option value="lc">Saint Lucia</option>
  <option value="pm">Saint Pierre and Miquelon</option>
  <option value="vc">Saint Vincent and the Grenadines</option>
  <option value="ws">Samoa</option>
  <option value="sm">San Marino</option>
  <option value="st">Sao Tome and Principe</option>
  <option value="sa">Saudi Arabia</option>
  <option value="sn">Senegal</option>
  <option value="cs">Serbia and Montenegro</option>
  <option value="sc">Seychelles</option>
  <option value="sl">Sierra Leone</option>
  <option value="sg">Singapore</option>
  <option value="sk">Slovakia</option>
  <option value="si">Slovenia</option>
  <option value="sb">Solomon Islands</option>
  <option value="so">Somalia</option>
  <option value="za">South Africa</option>
  <option value="gs">South Georgia and the South Sandwich Islands</option>
  <option value="es">Spain</option>
  <option value="lk">Sri Lanka</option>
  <option value="sd">Sudan</option>
  <option value="sr">Suriname</option>
  <option value="sj">Svalbard and Jan Mayen</option>
  <option value="sz">Swaziland</option>
  <option value="se">Sweden</option>
  <option value="ch">Switzerland</option>
  <option value="sy">Syrian Arab Republic</option>
  <option value="tw">Taiwan, Province of China</option>
  <option value="tj">Tajikistan</option>
  <option value="tz">Tanzania, United Republic of</option>
  <option value="th">Thailand</option>
  <option value="tl">Timor-Leste</option>
  <option value="tg">Togo</option>
  <option value="tk">Tokelau</option>
  <option value="to">Tonga</option>
  <option value="tt">Trinidad and Tobago</option>
  <option value="tn">Tunisia</option>
  <option value="tr">Turkey</option>
  <option value="tm">Turkmenistan</option>
  <option value="tc">Turks and Caicos Islands</option>
  <option value="tv">Tuvalu</option>
  <option value="ug">Uganda</option>
  <option value="ua">Ukraine</option>
  <option value="ae">United Arab Emirates</option>
  <option value="uk">United Kingdom</option>
  <option value="us">United States</option>
  <option value="um">United States Minor Outlying Islands</option>
  <option value="uy">Uruguay</option>
  <option value="uz">Uzbekistan</option>
  <option value="vu">Vanuatu</option>
  <option value="ve">Venezuela</option>
  <option value="vn">Viet Nam</option>
  <option value="vg">Virgin Islands, British</option>
  <option value="vi">Virgin Islands, U.S.</option>
  <option value="wf">Wallis and Futuna</option>
  <option value="eh">Western Sahara</option>
  <option value="ye">Yemen</option>
  <option value="zm">Zambia</option>
  <option value="zw">Zimbabwe</option>
  </select>

  <select name="SafeDropDown" class="quickSet">
  <option disabled selected hidden>';
  if(isset($_COOKIE['safe'])){
    switch ($_COOKIE['safe']) {
      case "active":
        echo "SafeSearch: On";
        break;
      case "off":
        echo "SafeSearch: Off";
        break;
    }
  }
  else{echo "SafeSearch: On";}
echo '
    </option>
    <option value="active">On</option>
    <option value="off">Off</option>
</select>

<select name="TimeDropDown" class="quickSet">
<option disabled selected hidden>';
if(isset($_COOKIE['time'])){
switch ($_COOKIE['time']):
  case "day":
    echo "Past day";
    break;
  case "week":
    echo "Past week";
    break;
  case "month":
    echo "Past month";
    break;
  case "year":
    echo "Past year";
    break;
endswitch;
}
else{echo "Any time";}
echo '
    </option>
    <option value="any">Any time</option>
    <option value="day">Past day</option>
    <option value="week">Past week</option>
    <option value="month">Past month</option>
    <option value="year">Past year</option>

</select>

<input type="submit" name="savequicksetting" class="right9vw quickSet" value="Save">
</form>
<div class="quickSettingsSpace"></div>

<input type="checkbox" id="notify" class="notificationCheckbox none" '; if(!isset($_COOKIE['telemetry'])){echo 'checked';}echo '>';

echo '<div class="notifications scrollDown flexDColumn max-height250 fixed right60 whiteAblackBg padding20 curve top30 ';
if(!isset($_COOKIE['telemetry'])){echo 'mobile_top80P';}

echo '">
<label for="notify" id="notifyHandle" class="width150 block height30 bgGray absolute curve top-15 left50PM74"></label>';

  if($not){
  echo '<form method="POST">
    <input type="text" value="',$t,'" class="none" name="markasreadValue">
    <input type="submit" name="markasread" value="mark as read" class="link float-right borderNone bgNone mt--10">
  </form>';
  }
  $i = 1;
  foreach($notifications as $notify) {
    if(!is_numeric($notify['date']) && isset($_COOKIE[$notify['date']])) {--$j;continue;}

    if(!isset($_COOKIE['telemetry']) && $notify['date'] != 'telemetry') {continue;}

    echo '<div class="max-width500';
    if($i < $j && isset($_COOKIE['telemetry'])) {echo ' borderBottom';}
    echo '">';
    if(!empty($notify['url'])) {
      echo '<a href="', $notify['url'], '" target="_blank"';
    }
    else{
      echo '<div ';
    }
    echo ' class="clearLink flex flexDRow alignC">
    <img src="',$notify['icon'],'" class="width80 padding10 mr-10">
    <div>
    <p>',$notify['body'],'</p></div>';
    if(!empty($notify['url'])) {echo '</a>';}
    else{echo '</div>';}
    echo '</div>';
    ++$i;
  }

echo '</div>';
if(!isset($_COOKIE['telemetry'])){
echo '<div class="none" data="NoTelemetry"></div>';
}
