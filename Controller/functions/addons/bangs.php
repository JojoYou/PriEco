<?php
if(strpos($purl, '!') !== false){
    $bangObj = json_decode(file_get_contents('Controller/value/bangs.json'), true);
    $tmp = explode(' ', $purl);
    $bangs = array();

    $bangs_Sites= '<div id="bangs" bangs="';

    foreach($tmp as &$t){
      $tmp2 = str_replace('!','',$t);
      if(isset($bangObj['bangs'][$tmp2]) && strpos($t, '!') !== false){
        $tmps = str_replace($t,'', $purl);

        if(isset($_COOKIE['DisMul'])){header('Location: ' . $bangObj['bangs'][$tmp2] . urldecode($tmps));exit();}
        else{$bangs[] = $bangObj['bangs'][$tmp2]; $bangs_Sites .= $bangObj['bangs'][$tmp2].';';}
      }
      else{
        $bangQuery .= $t.' ';
      }
    }

    if(count($bangs) == 1){header("Location: " . $bangs[0].$bangQuery, true); exit();}
    elseif(count($bangs) > 1) {
      echo $bangs_Sites,'" query="',urlencode(substr($bangQuery, 0, -1)),'"></div><script src="View/js/bangs.js"></script>
      <h1>If you haven\'t already, please allow PriEco to open multi  ple tabs in your web browser.</h1>';
      exit();
    }
    else{
      echo '"></div>';
    }

}
