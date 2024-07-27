<?php 
echo'<div class="wrapper">
<input type="checkbox" id="btn" hidden /><label
  for="btn"
  class="menu-btn">
  <img
    alt="icnSet"
    class="icnSet"
    src="./View/icon/gear.svg"
/></label>
<nav id="sidebar">
  <label for="btn"><img class="filterImage close-menu-btn"
    alt="icnCross" src="./View/icon/cross.svg"></label
  >
  <div class="sidebarTitle flex"><img src="./View/icon/gear.svg" alt="Settings icon">Settings</div>
  <div class="list-items">
    <form method="post" class="form-items" action="">
      <div class="setGroup">
      <div class="setListItem">
        <h3>General</h3>
      </div>
      <br />
      <div class="setListItem">
        <p>Language:</p>
        <div>
        <select class="setSelect" name="LangDropdown">
          <option disabled hidden selected>
';
if (isset($lang) && $lang !== 'all')
{
    $i = array_search($lang, $langVal);
    echo $langName[$i];
}
echo '</option><option value="all"';
if (!isset($lang) or $lang == 'all')
{
    echo 'selected';
}
echo '>All languages</option>
<option name="ar" value="ar">Arabic</option>
<option name="bg" value="bg">Bulgarian</option>
<option name="ca" value="ca">Catalan</option>
<option name="hr" value="hr">Croatia</option>
<option name="cs" value="cs">Czech</option>
<option name="da" value="da">Danish</option>
<option name="nl" value="nl">Dutch</option>
<option name="en" value="en">English</option>
<option name="et" value="et">Estonian</option>
<option name="fi" value="fi">Finnish</option>
<option name="fr" value="fr">French</option>
<option name="de" value="de">German</option>
<option name="el" value="el">Greek</option>
<option name="iw" value="iw">Hebrew</option>
<option name="hu" value="hu">Hungarian</option>
<option name="is" value="is">Icelandic</option>
<option name="id" value="id">Indonesian</option>
<option name="it" value="it">Italian</option>
<option name="ja" value="ja">Japanese</option>
<option name="ko" value="ko">Korean</option>
<option name="lv" value="lv">Latvian</option>
<option name="lt" value="lt">Lithuanian</option>
<option name="no" value="no">Norwegian</option>
<option name="pl" value="pl">Polish</option>
<option name="pt" value="pt">Portuguese</option>
<option name="ro" value="ro">Romanian</option>
<option name="ru" value="ru">Russian</option>
<option name="sr" value="sr">Serbian</option>
<option name="sk" value="sk">Slovak</option>
<option name="sl" value="sl">Slovenian</option>
<option name="es" value="es">Spanish</option>
<option name="sv" value="sv">Swedish</option>
<option name="tr" value="tr">Turkish</option>
</select> <input type="submit" value="Save" class="langSave" name="langSave"></div></div><br><div class="setListItem"><label for="newtab">Open links in New tab:</label><label class="switch">
<input type="submit" name="newtab" id="newtab" value="newtab';if (isset($_COOKIE['new'])){echo 'On';}else{echo 'Off';}echo '">
<input type="checkbox" class="setCheck" ';
if (isset($_COOKIE['new']))
{
    echo 'checked';
}
echo '>
<span class="slider round"></span>
</label></div><br><div class="setListItem"><p>Theme:</p>
<div class="themeMenu">
<div>


<input type="radio" class="none"';
if(!isset($_COOKIE['mode'])){
echo ' checked ';
}
echo '>
<input type="submit" id="systemTheme" name="systemTheme" class="none">
<label class="';if(!isset($_COOKIE['mode'])){echo 'setChooseChecked ';}echo 'unitTemp" for="systemTheme">🌓</label>

<input type="submit" id="light" name="light" class="none">
<label class="';if(isset($_COOKIE['mode']) && $_COOKIE['mode'] == 1){echo 'setChooseChecked ';}echo 'unitTemp" for="light">☀️</label>

<input type="submit" id="dark" name="dark" class="none">
<label class="';if(isset($_COOKIE['mode']) && $_COOKIE['mode'] == 2){echo 'setChooseChecked ';}echo 'unitTemp" for="dark">🌑</label>

<label class="labCustom" for="custom">🌈</label></div>
<input type="checkbox" class="showCustomThemeBox" aria-label="Open custom editor" class="float-right"><textarea name="customTheme" class="';
if (isset($_COOKIE['mode']) && $_COOKIE['mode'] == 3)
{
    echo 'CustomThemeShow';
}
echo 'CustomThemeBox" placeholder="Custom theme(Css injection)">';
if (isset($_COOKIE['theme']))
{
    echo $_COOKIE['theme'];
}
echo '</textarea>
<div class="themeSetText">
<label for="systemTheme">System</label>
<label for="light">Light</label>
<label for="dark">Dark</label>
</div>

</div>
</div>

<br><div class="setListItem"><p>Units:</p>
<div class="themeMenu">
<div>


<input type="submit" id="tempC" name="tempC" class="none">
<label class="';if(!isset($_COOKIE['temp'])){echo 'setChooseChecked ';}echo 'unitC unitTemp" for="tempC">°C</label>

<input type="submit" id="tempF" name="tempF" class="none">
<label class="';if(isset($_COOKIE['temp']) && $_COOKIE['temp'] == 'f'){echo 'setChooseChecked ';}echo 'unitF unitTemp" for="tempF">°F</label>

<input type="submit" id="tempK" name="tempK" class="none">
<label class="';if(isset($_COOKIE['temp']) && $_COOKIE['temp'] == 'k'){echo 'setChooseChecked ';}echo 'unitK unitTemp" for="tempK">K</label>
</div>


</div>
</div>

<br><div class="setListItem"><label for="datasaver">Cellular data saver:</label><label class="switch">
<input type="submit" class="none" name="datasave" id="datasaver" value="datasave';if (isset($_COOKIE['datasave'])){echo 'On';}else{echo 'Off';}echo '">
<input type="checkbox" class="setCheck" ';
if (isset($_COOKIE['datasave']))
{
    echo 'checked';
}
echo '><span class="slider round"></span>
</label></div>

<br><div class="setListItem"><p>Suggestions:</p>
<div>
<select name="sugPDropdown">

<option value="d"';if (!isset($_COOKIE['sugProvider'])){echo 'selected';}echo '>DuckDuckGo</option>
<option value="g"';if (isset($_COOKIE['sugProvider']) && $_COOKIE['sugProvider'] == 'g'){echo 'selected';}echo '>Google</option>
<option value="p"';if (isset($_COOKIE['sugProvider']) && $_COOKIE['sugProvider'] == 'p'){echo 'selected';}echo '>PriEco</option>
</select> <input type="submit" value="Save" class="langSave" name="sugPSave">
</div>
</div>


</div>';
/*
<div class="setGroup"><div class="setListItem"><h3>Data</h3></div>
<br>
<div class="setListItem"><label for="aCou"><p>User counter:</p></label><label class="switch">
<input type="submit" class="none" name="aCou" id="aCou" value="aCou';if (isset($_COOKIE['userid'])){echo 'On';}else{echo 'Off';}echo '">
<input type="checkbox" class="setCheck" ';
if (isset($_COOKIE['userid']))
{
    echo 'checked';
}
echo '>
<span class="slider round"></span>
</label></div>
</div>
*/
echo '<div class="setGroup"><div class="setListItem"><h3>Privacy</h3></div>
<br>
<div class="setListItem"><a href="web/privacypolicy.php#telemetry" target="_blank" class="link">Telemetry:</a><label class="switch">
<input type="submit" class="none" name="telemetry" id="telemetry" value="telemetry';if (isset($_COOKIE['telemetry']) && $_COOKIE['telemetry'] == 'on'){echo 'On';}else{echo 'Off';}echo '">
<input type="checkbox" class="setCheck" ';
if (isset($_COOKIE['telemetry']) && $_COOKIE['telemetry'] == 'on')
{
    echo 'checked';
}
echo '>
<span class="slider round"></span>
</label></div>
<br>
<div class="setListItem"><label for="hQuery">Hide query:</label><label class="switch">
<input type="submit" class="none" name="hQuery" id="hQuery" value="hQuery';if (isset($_COOKIE['hQuery'])){echo 'On';}else{echo 'Off';}echo '">
<input type="checkbox" class="setCheck" ';
if (isset($_COOKIE['hQuery']))
{
    echo 'checked';
}
echo '>
<span class="slider round"></span>
</label></div>

<br>
<div class="setListItem">
<input type="checkbox" id="proxySet" class="none">
<label for="proxySet" class="flex alignC Pointer">Instances <img src="View/icon/dropdown.svg" class="width15 height15 filterImage ml-10" alt="Show instances"></label>
<br><br>
<div id="openCheckSettings">
<a href="https://docs.invidious.io/instances/" target="_blank" class="link">Video instance</a>
<div class="flex justContSpace-Between">
<input type="url" class="langSave" name="vidURL" placeholder="https://yewtu.be/watch?v="';if(isset($_COOKIE['vidURL'])){echo 'value="',$_COOKIE['vidURL'],'"';}echo'>
<input type="submit" value="Save" class="langSave" name="vidSave">
</div>

<br><br>
<a href="https://github.com/redlib-org/redlib-instances/blob/main/instances.md" target="_blank" class="link">Reddit instance:</a>
<div class="flex justContSpace-Between">
    <input type="url" class="langSave" name="redURL" placeholder="https://safereddit.com/"';if(isset($_COOKIE['redURL'])){echo 'value="',$_COOKIE['redURL'],'"';}echo'>
    <input type="submit" value="Save" class="langSave" name="redSave">
    </div>
</div>
</div>
<br>
<div class="setListItem">
<input type="checkbox" id="jsSet" class="none">
<label for="jsSet" class="flex alignC Pointer">JavaScript <img src="View/icon/dropdown.svg" class="width15 height15 filterImage ml-10" alt="Show JavaScript settings"></label>
<br><br>
<div id="openCheckSettings">
<div class="setListItem"><label for="dSug">Suggestions:</label><label class="switch">
<input type="submit" class="none" name="dSug" id="dSug" value="dSug';if (isset($_COOKIE['DisSugges'])){echo 'On';}else{echo 'Off';}echo '">
<input type="checkbox" class="setCheck" ';
if (!isset($_COOKIE['DisSugges']))
{
    echo 'checked';
}
echo '>
<span class="slider round"></span>
</label></div><br>
<div class="setListItem"><label for="dSug">Multi Bang:</label><label class="switch">
<input type="submit" class="none" name="dMul" id="dMul" value="dMul';if (isset($_COOKIE['DisMul'])){echo 'On';}else{echo 'Off';}echo '">
<input type="checkbox" class="setCheck" ';
if (!isset($_COOKIE['DisMul']))
{
    echo 'checked';
}
echo '>
<span class="slider round"></span>
</label></div><br>
<div class="setListItem"><label for="dQue">Clear Query:</label><label class="switch">
<input type="submit"class="none" name="dQue" id="dQue" value="dQue';if (isset($_COOKIE['DisQue'])){echo 'On';}else{echo 'Off';}echo '">
<input type="checkbox" class="setCheck" ';
if (!isset($_COOKIE['DisQue']))
{
    echo 'checked';
}
echo '>
<span class="slider round"></span>
</label></div>
<br><div class="setListItem"><label for="dWid">Widgets:</label><label class="switch">
<input type="submit" class="none" name="dWid" id="dWid" value="dWid';if (isset($_COOKIE['DisWid'])){echo 'On';}else{echo 'Off';}echo '">
<input type="checkbox" class="setCheck" ';
if (!isset($_COOKIE['DisWid']))
{
    echo 'checked';
}
echo '>
<span class="slider round"></span>
</label></div>

<br><div class="setListItem"><label for="DisHImg">High res img:</label><label class="switch">
<input type="submit" class="none" name="DisHImg" id="DisHImg" value="DisHImg';if (isset($_COOKIE['DisHImg'])){echo 'On';}else{echo 'Off';}echo '">
<input type="checkbox" class="setCheck" ';
if (!isset($_COOKIE['DisHImg']))
{
    echo 'checked';
}
echo '>
<span class="slider round"></span>
</label>
</div>
</div>
</div>
</div>

<div class="setGroup"><div class="setListItem"><h3>Dev</h3></div>
<br><div class="setListItem"><label for="index">Use PriEco index:</label> <label class="switch">
<input type="submit" class="none" name="index" id="index" value="index';if (isset($_COOKIE['index'])){echo 'On';}else{echo 'Off';}echo '">
<input type="checkbox" class="setCheck" ';
if (isset($_COOKIE['index']))
{
    echo 'checked';
}
echo '>
<span class="slider round"></span>
</label></div>

<br>
<div class="setListItem">
<input type="checkbox" id="rankingSet" class="none">
<label for="rankingSet" class="flex alignC Pointer">Ranking <img src="View/icon/dropdown.svg" class="width15 height15 filterImage ml-10" alt="Show JavaScript settings"></label>
<br><br>
<div id="openCheckSettings">
<input type="submit" value="Save" class="width100P langSave" name="rankSave">
<div class="setListItem">
<p class="mt-10">Title points</p>
<input type="number" name="rankTitle" class="width100P langSave" placeholder="Title points" value="', (isset($_COOKIE['rankTitle']) ? $_COOKIE['rankTitle'] : 200),'">

<p class="mt-10">H1 points</p>
<input type="number" name="rankSecTitle" class="width100P langSave" placeholder="H1 points" value="', (isset($_COOKIE['rankSecTitle']) ? $_COOKIE['rankSecTitle'] : 100),'">

<p class="mt-10">Description points</p>
<input type="number" name="rankDesc" class="width100P langSave" placeholder="Description points" value="', (isset($_COOKIE['rankDesc']) ? $_COOKIE['rankDesc'] : 50),'">

<p class="mt-10">URL points</p>
<input type="number" name="rankURL" class="width100P langSave" placeholder="URL points" value="', (isset($_COOKIE['rankURL']) ? $_COOKIE['rankURL'] : 100),'">

<p class="mt-10">Domain points</p>
<input type="number" name="rankDomain" class="width100P langSave" placeholder="URL points" value="', (isset($_COOKIE['rankDomain']) ? $_COOKIE['rankDomain'] : 50),'">

<p class="mt-10">Language points</p>
<input type="number" name="rankLang" class="width100P langSave" placeholder="URL points" value="', (isset($_COOKIE['rankLang']) ? $_COOKIE['rankLang'] : 100),'">

<p class="mt-10">Location points</p>
<input type="number" name="rankLoc" class="width100P langSave" placeholder="URL points" value="', (isset($_COOKIE['rankLoc']) ? $_COOKIE['rankLoc'] : 100),'">

<p class="mt-10">Most used keyword points</p>
<input type="number" name="rankMas" class="width100P langSave" placeholder="URL points" value="', (isset($_COOKIE['rankMas']) ? $_COOKIE['rankMas'] : 250),'">

</div>
</div>
</div>

<br><div class="setListItem"><label for="providers">Show providers:</label> <label class="switch">
<input type="submit" class="none" name="providers" id="providers" value="providers';if (isset($_COOKIE['providers'])){echo 'On';}else{echo 'Off';}echo '">
<input type="checkbox" class="setCheck" ';
if (isset($_COOKIE['providers']))
{
    echo 'checked';
}
echo '>
<span class="slider round"></span>
</label></div>

<br><div class="setListItem"><label for="showtime">Show time:</label> <label class="switch">
<input type="submit" class="none" name="showtime" id="showtime" value="showtime';if (isset($_COOKIE['showtime'])){echo 'On';}else{echo 'Off';}echo '">
<input type="checkbox" class="setCheck" ';
if (isset($_COOKIE['showtime']))
{
    echo 'checked';
}
echo '>
<span class="slider round"></span>
</label></div>
</div>
<div class="empty"></div>
</div></form></nav></div>';