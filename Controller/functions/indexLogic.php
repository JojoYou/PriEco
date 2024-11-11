<?php
//Homepage
if (isset($_POST['shortcutSubmit'])) {
  $shURL = $_POST['shortcutURL'];
  if (strpos($shURL, "http://") !== 0 && strpos($shURL, "https://") !== 0 && strpos($shURL, "file://") !== 0) {
    $shURL = 'https://' . $shURL;
  }
  setcookie('shortcuts', $_COOKIE['shortcuts'] . ',' . $_POST['shortcutName'] . '=' . $shURL, ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Lax']);
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
  setcookie('shortcuts', $pasteCookie, ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Lax']);
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

  setcookie('shortcuts', $pasteCookie, ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Lax']);
  $reload = true;
}
//Quick Settings Buttons
if (isset($_POST['revetToGlobal'])) {
  setcookie('Language', 'all', ['expires' => time() + 604800,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
  setcookie('Location', 'all', ['expires' => time() + 604800,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
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
  setcookie('mode', '', ['expires' => time() - 3600,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Lax']);
  $reload = true;
}
if (isset($_POST['light'])) {
  setcookie('mode', '1', ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Lax']);
  $reload = true;
}
if (isset($_POST['dark'])) {
  setcookie('mode', '2', ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Lax']);
  $reload = true;
}

if ($_POST['customTheme'] && $_POST['customTheme'] != "") {
  setcookie('theme', $_POST['customTheme'], ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
}
if (isset($_POST['langSave'])) {
  setcookie('Language', $_POST['LangDropdown'], ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
  $reload = true;
}

if (isset($_POST['tempC'])) {
  setcookie('temp', '', ['expires' => time() - 3600,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
  $reload = true;
}
if (isset($_POST['tempF'])) {
  setcookie('temp', 'f', ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
  $reload = true;
}
if (isset($_POST['tempK'])) {
  setcookie('temp', 'k', ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
  $reload = true;
}

if (isset($_POST['newtab'])) {
  if ($_POST['newtab'] == 'newtabOff') {
    setcookie('new', 'on', ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
    $reload = true;
  } else {
    setcookie('new', '', ['expires' => time() - 3600,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
    $reload = true;
  }
}
if (isset($_POST['index'])) {
  if ($_POST['index'] == 'indexOff') {
    setcookie('index', 'on', ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
    $reload = true;
  } else {
    setcookie('index', '', ['expires' => time() - 3600,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
    $reload = true;
  }
}
if (isset($_POST['providers'])) {
  if ($_POST['providers'] == 'providersOff') {
    setcookie('providers', 'on', ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
    $reload = true;
  } else {
    setcookie('providers', '', ['expires' => time() - 3600,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
    $reload = true;
  }
}

if (isset($_POST['showtime'])) {
  if ($_POST['showtime'] == 'showtimeOff') {
    setcookie('showtime', 'on', ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
    $reload = true;
  } else {
    setcookie('showtime', '', ['expires' => time() - 3600,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
    $reload = true;
  }
}
if (isset($_POST['datasave'])) {
  if ($_POST['datasave'] == 'datasaveOff') {
    setcookie('datasave', 'on', ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
    $reload = true;
  } else {
    setcookie('datasave', '', ['expires' => time() - 3600,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
    $reload = true;
  }
}
if (isset($_POST['sugPSave'])) {
  if ($_POST['sugPDropdown'] == 'd') {
    setcookie('sugProvider', '', ['expires' => time() - 3600,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Lax']);
  } else {
    setcookie('sugProvider', $_POST['sugPDropdown'], ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Lax']);
  }
  $reload = true;
}

if (isset($_POST['lyrPSave'])) {
  if ($_POST['lyrPDropdown'] == 'sp') {
    setcookie('lyrProvider', '', ['expires' => time() - 3600,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
  } else {
    setcookie('lyrProvider', $_POST['lyrPDropdown'], ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
  }
  $reload = true;
}

if (isset($_POST['hQuery'])) {
  if ($_POST['hQuery'] == 'hQueryOff') {
    setcookie('hQuery', 'on', ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
    $reload = true;
  } else {
    setcookie('hQuery', '', ['expires' => time() - 3600,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
    $reload = true;
  }
}

if (isset($_POST['telemetry'])) {
  if ($_POST['telemetry'] == 'telemetryOff' || $_POST['telemetry'] == 'Accept') {
    setcookie('telemetry', 'on', ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
    $reload = true;
  } else {
    setcookie('telemetry', 'off', ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
    $reload = true;
  }
}

if (isset($_POST['vidSave'])) {
  if ($_POST['vidURL'] == '') {
    setcookie('vidURL', '', ['expires' => time() - 3600,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
  } else {
    setcookie('vidURL', $_POST['vidURL'], ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
  }
  $reload = true;
}
if (isset($_POST['redSave'])) {
  if ($_POST['redURL'] == '') {
    setcookie('redURL', '', ['expires' => time() - 3600,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
  } else {
    setcookie('redURL', $_POST['redURL'], ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
  }
  $reload = true;
}
if (isset($_POST['dSug'])) {
  if ($_POST['dSug'] == 'dSugOff') {
    setcookie('DisSugges', 'on', ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
    $reload = true;
  } else {
    setcookie('DisSugges', '', ['expires' => time() - 3600,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
    $reload = true;
  }
}
if (isset($_POST['dMul'])) {
  if ($_POST['dMul'] == 'dMulOff') {
    setcookie('DisMul', 'on', ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
    $reload = true;
  } else {
    setcookie('DisMul', '', ['expires' => time() - 3600,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
    $reload = true;
  }
}
if (isset($_POST['dQue'])) {
  if ($_POST['dQue'] == 'dQueOff') {
    setcookie('DisQue', 'on', ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
    $reload = true;
  } else {
    setcookie('DisQue', '', ['expires' => time() - 3600,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
    $reload = true;
  }
}
if (isset($_POST['dWid'])) {
  if ($_POST['dWid'] == 'dWidOff') {
    setcookie('DisWid', 'on', ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
    $reload = true;
  } else {
    setcookie('DisWid', '', ['expires' => time() - 3600,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
    $reload = true;
  }
}
if (isset($_POST['DisHImg'])) {
  if ($_POST['DisHImg'] == 'DisHImgOff') {
    setcookie('DisHImg', 'on', ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
    $reload = true;
  } else {
    setcookie('DisHImg', '', ['expires' => time() - 3600,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
    $reload = true;
  }
}

if(isset($_POST['rankSave'])){
  if ($_POST['rankTitle'] != 200) {setcookie('rankTitle', $_POST['rankTitle'], ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);}
  else{setcookie('rankTitle', '', ['expires' => time() - 3600,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);}

  if ($_POST['rankSecTitle'] != 100) {setcookie('rankSecTitle', $_POST['rankSecTitle'], ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);}
  else{setcookie('rankSecTitle', '', ['expires' => time() - 3600,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);}

  if ($_POST['rankDesc'] != 50) {setcookie('rankDesc', $_POST['rankDesc'], ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);}
  else{setcookie('rankDesc', '', ['expires' => time() - 3600,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);}

  if ($_POST['rankURL'] != 1000) {setcookie('rankURL', $_POST['rankURL'], ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);}
  else{setcookie('rankURL', '', ['expires' => time() - 3600,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);}

  if ($_POST['rankDomain'] != 50) {setcookie('rankDomain', $_POST['rankDomain'], ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);}
  else{setcookie('rankDomain', '', ['expires' => time() - 3600,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);}

  if ($_POST['rankLang'] != 100) {setcookie('rankLang', $_POST['rankLang'], ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);}
  else{setcookie('rankLang', '', ['expires' => time() - 3600,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);}

  if ($_POST['rankLoc'] != 100) {setcookie('rankLoc', $_POST['rankLoc'], ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);}
  else{setcookie('rankLoc', '', ['expires' => time() - 3600,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);}

  if ($_POST['rankMas'] != 250) {setcookie('rankMas', $_POST['rankMas'], ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);}
  else{setcookie('rankMas', '', ['expires' => time() - 3600,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);}
  $reload = true;
}

if (isset($_POST['savequicksetting'])) {

  if (isset($_POST['LocDropDown'])) {
    if ($_POST['LocDropDown'] == "all") {
      setcookie('Location', 'all', ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
    } else {
      setcookie('Location', $_POST['LocDropDown'], ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
    }
  }

  if (isset($_POST['SafeDropDown'])) {
    if ($_POST['SafeDropDown'] == "off") {
      setcookie('safe', 'off', ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
    } else {
      setcookie('safe', '', ['expires' => time() - 3600,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
    }
  }
  if (isset($_POST['TimeDropDown'])) {
    switch ($_POST['TimeDropDown']):

      case "day":
        setcookie('time', 'day', ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
        break;
      case "week":
        setcookie('time', 'week', ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
        break;
      case "month":
        setcookie('time', 'month', ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
        break;
      case "year":
        setcookie('time', 'year', ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
        break;
      default:
        setcookie('time', '', ['expires' => time() - 3600,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
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
  setcookie('tabs', $_COOKIE['tabs'].',', ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
  $reload = true;
}
elseif(isset($_POST['tabClose'])){
  $tabs = explode(',', $_COOKIE['tabs']);
  unset($tabs[$_POST['tabID']]);
  setcookie('tabs', implode(',', $tabs), ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
  $reload = true;
}

if(isset($_POST['markasread'])){
  setcookie('notify', $_POST['markasreadValue'], ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Strict']);
  $reload = true;
}

//Album
if (!isset($_GET['album'])) {
  if (strpos($purl, "album: ") !== false) {
      $album_title = substr($purl, strpos($purl, "album: ") + 7);
      header('Location: '.(isset($_SERVER['HTTPS']) && $_SERVER['HTTPS'] === 'on' ? "https://" : "http://") . $_SERVER['HTTP_HOST'] . $_SERVER['REQUEST_URI'] . '&album=true', true);
      exit();
    }
}

//Landing page
if(isset($_POST['hideDesc'])){
  if(isset($_COOKIE['hideDesc'])){
    setcookie('hideDesc', '', ['expires' => time() - 3600,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Lax']);
  }
  else{
    setcookie('hideDesc', 'true', ['expires' => time() + 31536000,'path' => '/','domain' => null,'secure' => true,'httponly' => true,'samesite' => 'Lax']);
  }
  $reload = true;
}
//Reload
if ($reload) {
  $reload = false;
  header("Refresh:0");
  exit();
}
