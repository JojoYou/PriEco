<?php
 if (preg_match('/\bip\b/i', $purl) && !str_starts_with($_SERVER['REMOTE_ADDR'], '192.') && !str_starts_with($_SERVER['REMOTE_ADDR'], '127.')) {
    echo '<div class="redditCon output">
      <p class="borderBottom centerTxt">Your Public IP Address is: <b>',$_SERVER['REMOTE_ADDR'],'</b></p><br>
      <div class="flexSpaceWrap">
      <div>';
      if($ipObj['country'] != '' || $ipObj['code'] != '' || $ipObj['lang'] != ''){echo '<p><b>Info</b></p>';}
      if($ipObj['country'] != ''){echo '<p class="ipInfo">Country: ',$ipObj['country'],'</p>';}
      if($ipObj['code'] != ''){echo '<p class="ipInfo">Code: ',$ipObj['code'],'</p>';}
      if($ipObj['lang'] != ''){echo '<p class="ipInfo">Language: ',$ipObj['lang'],'</p>';}
     
echo '</div>
     </div>
    </div>';
}