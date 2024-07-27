<?php
if(strpos($purl, '!') !== false){
    $bangObj = json_decode(file_get_contents('Controller/value/bangs.json'), true);
    $tmp = explode(' ', $purl);
    $bangs = array();

    echo '<div id="bangs" bangs="';

    foreach($tmp as &$t){
      $tmp2 = str_replace('!','',$t);
      if(isset($bangObj['bangs'][$tmp2]) && strpos($t, '!') !== false){
        $tmps = str_replace($t,'', $purl);
        
        if(isset($_COOKIE['DisMul'])){header('Location: ' . $bangObj['bangs'][$tmp2] . urldecode($tmps));exit();}
        else{$bangs[] = $bangObj['bangs'][$tmp2]; echo $bangObj['bangs'][$tmp2],';';}
      }
      else{
        $bangQuery .= $t.' ';
      }
    }

    if(count($bangs) == 1){header("Location: " . $bangs[0].$bangQuery, true);exit();}

    echo '" query="',urlencode(substr($bangQuery, 0, -1)),'"></div><script src="View/js/bangs.js"></script>
    <h1>If you haven\'t already, please allow PriEco to open multiple tabs in your web browser.</h1>';
    exit();
}
