<?php
//Homepage
if (isset($_POST['shortcutSubmit'])) {
  $shURL = $_POST['shortcutURL'];
  if (strpos($shURL, "http://") !== 0 && strpos($shURL, "https://") !== 0 && strpos($shURL, "file://") !== 0) {
    $shURL = 'https://' . $shURL;
  }
  setcookie('shortcuts', $_COOKIE['shortcuts'] . ',' . $_POST['shortcutName'] . '=' . $shURL, time() + 31536000, '/',null,true,true);
  $reload = true;
}
if (isset($_POST['shortcutDelete'])) {
  $shortcutCookie = explode(',', urldecode($_COOKIE['shortcuts']));
  $pasteCookie = '';
  $i = 0;
  foreach ($shortcutCookie as &$sc) {
    if ($i != $_POST['shortcutID'] - 1) {
      $pasteCookie .= $sc . ',';
    }
    ++$i;
  }
  $pasteCookie = substr($pasteCookie, 0, -1);
  setcookie('shortcuts', $pasteCookie, time() + 31536000, '/',null,true,true);
  $reload = true;
}
if (isset($_POST['shortcutEdit'])) {
  $shortcutCookie = explode(',', urldecode($_COOKIE['shortcuts']));

  $pasteCookie = '';
  $shURL = $_POST['shortcutURL'];
  if (strpos($shURL, "http://") !== 0 && strpos($shURL, "https://") !== 0) {
    $shURL = 'https://' . $shURL;
  }

  $i = 0;
  foreach ($shortcutCookie as &$sc) {
    if ($i != $_POST['shortcutID'] - 1) {
      $pasteCookie .= $sc . ',';
    } else {
      $pasteCookie .= $_POST['shortcutName'] . '=' . $shURL . ',';
    }
    ++$i;
  }
  $pasteCookie = substr($pasteCookie, 0, -1);

  setcookie('shortcuts', $pasteCookie, time() + 31536000, '/',null,true,true);
  $reload = true;
}
//Quick Settings Buttons
if (isset($_POST['revetToGlobal'])) {
  setcookie('Language', 'all', time() + 604800, '/',null,true,true);
  setcookie('Location', 'all', time() + 604800, '/',null,true,true);
  $reload = true;
}
if (isset($_POST['allBut'])) {
  header("Location: ./?q=" . urlencode($_POST['q']), true);
  exit();
}
if (isset($_POST['imgBut'])) {
  header('Location: ./?image&q=' . urlencode($_POST['q']), true);
  exit();
}
if (isset($_POST['videoBut'])) {
  header('Location: ./?video&q=' . urlencode($_POST['q']), true);
  exit();
}
if (isset($_POST['newsBut'])) {
  header('Location: ./?news&q=' . urlencode($_POST['q']), true);
  exit();
}
if (isset($_POST['shopBut'])) {
  header('Location: ./?shop&q=' . urlencode($_POST['q']), true);
  exit();
}
if (isset($_POST['mapBut'])) {
  header("Location: https://www.openstreetmap.org/search?query=" . urlencode($_POST['q']));
  exit();
}

//Save Settings

if (isset($_POST['systemTheme'])) {
  setcookie('mode', null, -1, '/',null,true,true);
  $reload = true;
}
if (isset($_POST['light'])) {
  setcookie('mode', '1', time() + 31536000, '/',null,true,true);
  $reload = true;
}
if (isset($_POST['dark'])) {
  setcookie('mode', '2', time() + 31536000, '/',null,true,true);
  $reload = true;
}

if ($_POST['customTheme'] && $_POST['customTheme'] != "") {
  setcookie('theme', $_POST['customTheme'], time() + 31536000, '/',null,true,true);
}
if (isset($_POST['langSave'])) {
  setcookie('Language', $_POST['LangDropdown'], time() + 31536000, '/',null,true,true);
  $reload = true;
}

if (isset($_POST['tempC'])) {
  setcookie('temp', null, -1, '/',null,true,true);
  $reload = true;
}
if (isset($_POST['tempF'])) {
  setcookie('temp', 'f', time() + 31536000, '/',null,true,true);
  $reload = true;
}
if (isset($_POST['tempK'])) {
  setcookie('temp', 'k', time() + 31536000, '/',null,true,true);
  $reload = true;
}

if (isset($_POST['newtab'])) {
  if ($_POST['newtab'] == 'newtabOff') {
    setcookie('new', 'on', time() + 31536000, '/',null,true,true);
    $reload = true;
  } else {
    setcookie('new', null, -1, '/',null,true,true);
    $reload = true;
  }
}
if (isset($_POST['index'])) {
  if ($_POST['index'] == 'indexOff') {
    setcookie('index', 'on', time() + 31536000, '/',null,true,true);
    $reload = true;
  } else {
    setcookie('index', null, -1, '/',null,true,true);
    $reload = true;
  }
}
if (isset($_POST['providers'])) {
  if ($_POST['providers'] == 'providersOff') {
    setcookie('providers', 'on', time() + 31536000, '/',null,true,true);
    $reload = true;
  } else {
    setcookie('providers', null, -1, '/',null,true,true);
    $reload = true;
  }
}

if (isset($_POST['showtime'])) {
  if ($_POST['showtime'] == 'showtimeOff') {
    setcookie('showtime', 'on', time() + 31536000, '/',null,true,true);
    $reload = true;
  } else {
    setcookie('showtime', null, -1, '/',null,true,true);
    $reload = true;
  }
}
if (isset($_POST['datasave'])) {
  if ($_POST['datasave'] == 'datasaveOff') {
    setcookie('datasave', 'on', time() + 31536000, '/',null,true,true);
    $reload = true;
  } else {
    setcookie('datasave', null, -1, '/',null,true,true);
    $reload = true;
  }
}
if (isset($_POST['sugPSave'])) {
  if ($_POST['sugPDropdown'] == 'd') {
    setcookie('sugProvider', null, -1, '/',null,true,true);
  } else {
    setcookie('sugProvider', $_POST['sugPDropdown'], time() + 31536000, '/',null,true,true);
  }
  $reload = true;
}

if (isset($_POST['hQuery'])) {
  if ($_POST['hQuery'] == 'hQueryOff') {
    setcookie('hQuery', 'on', time() + 31536000, '/',null,true,true);
    $reload = true;
  } else {
    setcookie('hQuery', null, -1, '/',null,true,true);
    $reload = true;
  }
}

if (isset($_POST['telemetry'])) {
  if ($_POST['telemetry'] == 'telemetryOff' || $_POST['telemetry'] == 'Accept') {
    setcookie('telemetry', 'on', time() + 31536000, '/',null,true,true);
    $reload = true;
  } else {
    setcookie('telemetry', 'off', time() + 31536000, '/',null,true,true);
    $reload = true;
  }
}

if (isset($_POST['vidSave'])) {
  if ($_POST['vidURL'] == '') {
    setcookie('vidURL', null, -1, '/',null,true,true);
  } else {
    setcookie('vidURL', $_POST['vidURL'], time() + 31536000, '/',null,true,true);
  }
  $reload = true;
}
if (isset($_POST['redSave'])) {
  if ($_POST['redURL'] == '') {
    setcookie('redURL', null, -1, '/',null,true,true);
  } else {
    setcookie('redURL', $_POST['redURL'], time() + 31536000, '/',null,true,true);
  }
  $reload = true;
}
if (isset($_POST['dSug'])) {
  if ($_POST['dSug'] == 'dSugOff') {
    setcookie('DisSugges', 'on', time() + 31536000, '/',null,true,true);
    $reload = true;
  } else {
    setcookie('DisSugges', null, -1, '/',null,true,true);
    $reload = true;
  }
}
if (isset($_POST['dMul'])) {
  if ($_POST['dMul'] == 'dMulOff') {
    setcookie('DisMul', 'on', time() + 31536000, '/',null,true,true);
    $reload = true;
  } else {
    setcookie('DisMul', null, -1, '/',null,true,true);
    $reload = true;
  }
}
if (isset($_POST['dQue'])) {
  if ($_POST['dQue'] == 'dQueOff') {
    setcookie('DisQue', 'on', time() + 31536000, '/',null,true,true);
    $reload = true;
  } else {
    setcookie('DisQue', null, -1, '/',null,true,true);
    $reload = true;
  }
}
if (isset($_POST['dWid'])) {
  if ($_POST['dWid'] == 'dWidOff') {
    setcookie('DisWid', 'on', time() + 31536000, '/',null,true,true);
    $reload = true;
  } else {
    setcookie('DisWid', null, -1, '/',null,true,true);
    $reload = true;
  }
}
if (isset($_POST['DisHImg'])) {
  if ($_POST['DisHImg'] == 'DisHImgOff') {
    setcookie('DisHImg', 'on', time() + 31536000, '/',null,true,true);
    $reload = true;
  } else {
    setcookie('DisHImg', null, -1, '/',null,true,true);
    $reload = true;
  }
}

if(isset($_POST['rankSave'])){
  if ($_POST['rankTitle'] != 200) {setcookie('rankTitle', $_POST['rankTitle'], time() + 31536000, '/',null,true,true);}
  else{setcookie('rankTitle', null, -1, '/',null,true,true);}

  if ($_POST['rankSecTitle'] != 100) {setcookie('rankSecTitle', $_POST['rankSecTitle'], time() + 31536000, '/',null,true,true);}
  else{setcookie('rankSecTitle', null, -1, '/',null,true,true);}

  if ($_POST['rankDesc'] != 50) {setcookie('rankDesc', $_POST['rankDesc'], time() + 31536000, '/',null,true,true);}
  else{setcookie('rankDesc', null, -1, '/',null,true,true);}

  if ($_POST['rankURL'] != 1000) {setcookie('rankURL', $_POST['rankURL'], time() + 31536000, '/',null,true,true);}
  else{setcookie('rankURL', null, -1, '/',null,true,true);}

  if ($_POST['rankDomain'] != 50) {setcookie('rankDomain', $_POST['rankDomain'], time() + 31536000, '/',null,true,true);}
  else{setcookie('rankDomain', null, -1, '/',null,true,true);}

  if ($_POST['rankLang'] != 100) {setcookie('rankLang', $_POST['rankLang'], time() + 31536000, '/',null,true,true);}
  else{setcookie('rankLang', null, -1, '/',null,true,true);}

  if ($_POST['rankLoc'] != 100) {setcookie('rankLoc', $_POST['rankLoc'], time() + 31536000, '/',null,true,true);}
  else{setcookie('rankLoc', null, -1, '/',null,true,true);}

  if ($_POST['rankMas'] != 250) {setcookie('rankMas', $_POST['rankMas'], time() + 31536000, '/',null,true,true);}
  else{setcookie('rankMas', null, -1, '/',null,true,true);}
  $reload = true;
}

if (isset($_POST['aCou'])) {
  if ($_POST['aCou'] == 'aCouOff') {
    setcookie('userid', rand(1000000, 1000000000000), time() + (86400 * 364), '/',null,true,true);
    setcookie('noanalytics', null, -1, '/',null,true,true);
    $reload = true;
  } else {
    setcookie('noanalytics', 'true', time() + (86400 * 364), '/',null,true,true);
    setcookie('userid', null, -1, '/',null,true,true);
    $reload = true;
  }
}

if (isset($_POST['savequicksetting'])) {

  if (isset($_POST['LocDropDown'])) {
    if ($_POST['LocDropDown'] == "all") {
      setcookie('Location', 'all', time() + 31536000, '/',null,true,true);
    } else {
      setcookie('Location', $_POST['LocDropDown'], time() + 31536000, '/',null,true,true);
    }
  }

  if (isset($_POST['SafeDropDown'])) {
    if ($_POST['SafeDropDown'] == "off") {
      setcookie('safe', 'off', time() + 31536000, '/',null,true,true);
    } else {
      setcookie('safe', null, -1, '/',null,true,true);
    }
  }
  if (isset($_POST['TimeDropDown'])) {
    switch ($_POST['TimeDropDown']):

      case "day":
        setcookie('time', 'day', time() + 31536000, '/',null,true,true);
        break;
      case "week":
        setcookie('time', 'week', time() + 31536000, '/',null,true,true);
        break;
      case "month":
        setcookie('time', 'month', time() + 31536000, '/',null,true,true);
        break;
      case "year":
        setcookie('time', 'year', time() + 31536000, '/',null,true,true);
        break;
      default:
        setcookie('time', null, -1, '/',null,true,true);
        break;
    endswitch;
  }
  $reload = true;
}
if (isset($_POST['imgtoolsSave'])) {
  $purl = urlencode($purl);
  header('Location: ./?image&imgsize=' . $_POST['imgtoolsSize'] . '&imgcolor=' . $_POST['imgtoolsColor'] . '&imgtype=' . $_POST['imgtoolsType'] . '&imgtime=' . $_POST['imgtoolsTime'] . '&imglicence=' . $_POST['imgtoolsRights'] . '&q=' . $purl, true);
  exit();
}
if (isset($_POST['shopToolsSave'])) {
  $purl = urlencode($purl);
  header('Location: ./?shop&shopMin=' . $_POST['shopPriceMin'] . '&shopMax=' . $_POST['shopPriceMax'] . '&q=' . $purl, true);
  exit();
}
if (isset($_POST['pixabayimg'])) {
  $purl = urlencode($purl);
  header('Location: ./?image&pixabay&q=' . $purl);
  exit();
}
if (isset($_POST['imgback'])) {
  $purl = urlencode($purl);
  header('Location: ./?image&q=' . $purl);
  exit();
}
if(isset($_POST['addTab'])){
  setcookie('tabs', $_COOKIE['tabs'].',', time() + 31536000, '/');
  $reload = true;
}
elseif(isset($_POST['tabClose'])){
  $tabs = explode(',', $_COOKIE['tabs']);
  unset($tabs[$_POST['tabID']]);
  setcookie('tabs', implode(',', $tabs),  time() + 31536000, '/');
  $reload = true;
}

if(isset($_POST['markasread'])){
  setcookie('notify', $_POST['markasreadValue'], time() + 31536000, '/',null,true,true);
  $reload = true;
}

//Reload
if ($reload) {
  $reload = false;
  header("Refresh:0");
  exit();
}
