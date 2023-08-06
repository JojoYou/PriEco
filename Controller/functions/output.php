<!--  SPDX-FileCopyrightText: 2022, 2022-2022 Roman  Láncoš <jojoyou@jojoyou.org> -->
<!-- -->
<!--  SPDX-License-Identifier: AGPL-3.0-or-later -->

<?php
$Bpurl = urlencode($purl);

if ($type != 'image' and $type != 'video' and $type != 'news') {
    //Get string between characters
    function get_string_betweens($string, $start, $end)
    {
        $string = ' ' . $string;
        $ini = strpos($string, $start);
        if ($ini === 0) return '';
        $ini += strlen($start);
        $len = strpos($string, $end, $ini) - $ini;
        return substr($string, $ini, $len);
    }
    
    //Initial call
    if(!$dev){
        $ImpProfiles = false;
        $ImpGoogle = true;
        
        //Profiles from PriEco
        $name = mysqli_real_escape_string($conn, strtolower($purl));  
        $sql = "SELECT * FROM `profiles` WHERE `Name` = '$name'";
        $tmp = $conn->query($sql);
        $tmp = $tmp->fetch_assoc();

        $i=1;
        $ddgObj='';
        foreach($tmp as &$ddg){
            if($i>2){
                $ddgObj .= $ddg. ' ';
            }
            ++$i;
        }

        if(isset($_SESSION[$purl]) && !isset($_COOKIE['safe']) && !isset($_COOKIE['time'])){
            if (strpos($_SESSION[$purl], 'obj +=+ ') !== false) {$obj = json_decode(str_replace('obj +=+ ', '', $_SESSION[$purl]), true);}
            if (strpos($_SESSION[$purl], 'g2obj +=+ ') !== false) {$g2obj = json_decode(str_replace('g2obj +=+ ', '', $_SESSION[$purl]), true);}
    
            if(isset($_SESSION[$purl.':-:news'])){$NewsObj = json_decode($_SESSION[$purl.':-:news'],true);}
            if(isset($_SESSION[$purl.':-:yt'])){$YoutubeObj = json_decode($_SESSION[$purl.':-:yt'], true);}
            if(isset($_SESSION[$purl.':-:red'])){$redditObj = json_decode($_SESSION[$purl.':-:red'], true);}
            if(isset($_SESSION[$purl.':-:wiki'])){$wikiobj =explode('--]|[--',$_SESSION[$purl.':-:wiki']);$wikiTxt = $wikiobj[1];$wikiobj = json_decode($wikiobj[0],true);}
            if(isset($_SESSION[$purl.':-:rel'])){$related = json_decode($_SESSION[$purl.':-:rel'],true);}
            
            if (preg_match('/\bip\b/i', $purl)) {
                $ipCh = curl_init('https://ipapi.co/'.$_SERVER['REMOTE_ADDR'].'/json/');
                curl_setopt($ipCh, CURLOPT_RETURNTRANSFER, true);
                curl_setopt($ipCh, CURLOPT_USERAGENT, 'Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13');
                curl_setopt($ipCh, CURLOPT_CONNECTTIMEOUT, 2);
                curl_setopt($ipCh, CURLOPT_TIMEOUT, 2.5);
                $ipObj = json_decode(curl_exec($ipCh),true);        
                curl_close($ipCh);
            }
            $cached = true;
            $ImpGoogle = false;
        }
        else{
        //Cached Google results
        $obj = '';
        if(!isset($_COOKIE['safe']) && !isset($_COOKIE['time'])){
        if(!isset($loc)){
            $loc = 'all';
        }
        if(!isset($lang)){
            $lang = 'all';
        }
        $sql = "SELECT * FROM `googleCache` WHERE `query` = '$name' AND `loc` = '$loc' AND `lang` = '$lang'";
        $tmp = $conn->query($sql);
        $tmp = $tmp->fetch_assoc();

        $obj = isset($tmp['results']) ? $tmp['results'] : '';

        if($obj != ''){
            if($tmp['official'] == 1){
            $ImpGoogle = false;
            $obj = json_decode($obj, true);
            }
            else{
                $ImpGoogle = false;
                $g2obj = json_decode($obj, true);
                $obj = '';
            }
        }
        }
    }

if(!isset($cached)){
        // Initialize multi-curl handle
$multiHandle = curl_multi_init();

// Initialize curl handles for each request
$curlHandles = array();
$curlCount = 0;
// Request 1: RedditFile
$ch1 = curl_init($RedditFile);
curl_setopt($ch1, CURLOPT_RETURNTRANSFER, true);
curl_setopt($ch1, CURLOPT_USERAGENT, 'Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13');
curl_setopt($ch1, CURLOPT_CONNECTTIMEOUT, 2);
curl_setopt($ch1, CURLOPT_TIMEOUT, 2.5);
$curlHandles[] = $ch1;
if(count($curlHandles) > $curlCount){
    $redditOn = true;
    $curlCount = count($curlHandles);
}

// Request 2: DdgFile
if($ddgObj == ''){
    $ImpProfiles = true;
    $ch2 = curl_init($DdgFile);
    curl_setopt($ch2, CURLOPT_RETURNTRANSFER, true);
    curl_setopt($ch2, CURLOPT_HTTPHEADER, array(
        'Origin: https://ac.duckduckgo.com',
        'Referer: https://ac.duckduckgo.com/',
        'Accept-Language: en-US,en;q=0.9'
    ));
    curl_setopt($ch2, CURLOPT_CONNECTTIMEOUT, 2.5);
    curl_setopt($ch2, CURLOPT_FOLLOWLOCATION, true);
    $curlHandles[] = $ch2;
    if(count($curlHandles) > $curlCount){
        $ddgOn = true;
        $curlCount = count($curlHandles);
    }
}

// Request 3: Googlefile, Brave or Qwant
if($obj == '' && !isset($g2obj)){
    if(!file_exists('disGoogle.txt')){
        $ch3 = curl_init($Googlefile);
    }
    else{
        $ch3 = curl_init('https://librex.jojoyou3.repl.co/api.php?p=0&t=0&q='.$Bpurl);
        $tmp = $lang;
        if($lang = 'all'){$tmp = 'en';}
        $cookies = 'google_language_results='.$tmp.';google_number_of_results=20;google_language_site='.$tmp.';';
        if(!isset($_COOKIE['safe'])){$cookies.='safe_search=on;';} 
        curl_setopt($ch3, CURLOPT_COOKIE, $cookies);
    }
    curl_setopt($ch3, CURLOPT_RETURNTRANSFER, true);
    curl_setopt($ch3, CURLOPT_CONNECTTIMEOUT, 2.5);
    $curlHandles[] = $ch3;
    if(count($curlHandles) > $curlCount){
        $searchOn = true;
        $curlCount = count($curlHandles);
    }
}

//Related searches
    $relatedCh = curl_init('https://ac.duckduckgo.com/ac/?type=list&callback=jsonCallback&_=1600956892202&q='.$Bpurl);
    curl_setopt($relatedCh, CURLOPT_RETURNTRANSFER, true);
    curl_setopt($relatedCh, CURLOPT_USERAGENT, 'Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13');
    curl_setopt($relatedCh, CURLOPT_CONNECTTIMEOUT, 2);
    curl_setopt($relatedCh, CURLOPT_TIMEOUT, 2.5);
    curl_setopt($relatedCh, CURLOPT_HTTPHEADER, array(
    'Origin: https://ac.duckduckgo.com'
));
   $curlHandles[] = $relatedCh;
   if(count($curlHandles) > $curlCount){
    $relatedOn = true;
    $curlCount = count($curlHandles);
}


$tmp = $_COOKIE['Language'];
    if($tmp == 'all' or $tmp == null){
        $tmp = 'en';
    }

//Wikipedia
$wikiCh = curl_init('https://search.jojoyou.org/Controller/functions/addons/wikiGet.php?lang='.$tmp.'&q=' .str_replace('+','_',urlencode(ucwords($purl))));
curl_setopt($wikiCh, CURLOPT_RETURNTRANSFER, true);
curl_setopt($wikiCh, CURLOPT_USERAGENT, 'Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13');
curl_setopt($wikiCh, CURLOPT_CONNECTTIMEOUT, 2);
curl_setopt($wikiCh, CURLOPT_TIMEOUT, 2.5);

$curlHandles[] = $wikiCh;
if(count($curlHandles) > $curlCount){
    $wikiOn = true;
    $curlCount = count($curlHandles);
}
//Define
if (isset($defWords)) {
    $defCh = curl_init($WordnikFile);
    curl_setopt($defCh, CURLOPT_RETURNTRANSFER, true);
    curl_setopt($defCh, CURLOPT_USERAGENT, 'Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13');
    curl_setopt($defCh, CURLOPT_CONNECTTIMEOUT, 2);
    curl_setopt($defCh, CURLOPT_TIMEOUT, 2.5);

      $curlHandles[] = $defCh;
  if(count($curlHandles) > $curlCount){
    $defOn = true;
    $curlCount = count($curlHandles);
    }
}
//Weather
if ($weatherTrue) {
    $weatherCh = curl_init($OpenWeatherFile);
    curl_setopt($weatherCh, CURLOPT_RETURNTRANSFER, true);
    curl_setopt($weatherCh, CURLOPT_USERAGENT, 'Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13');
    curl_setopt($weatherCh, CURLOPT_CONNECTTIMEOUT, 2);
    curl_setopt($weatherCh, CURLOPT_TIMEOUT, 2.5);

      $curlHandles[] = $weatherCh;
  if(count($curlHandles) > $curlCount){
    $weatherOn = true;
    $curlCount = count($curlHandles);
    }

    $weatherForecastCh = curl_init($OpenWeatherForecastFile);
    curl_setopt($weatherForecastCh, CURLOPT_RETURNTRANSFER, true);
    curl_setopt($weatherForecastCh, CURLOPT_USERAGENT, 'Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13');
    curl_setopt($weatherForecastCh, CURLOPT_CONNECTTIMEOUT, 2);
    curl_setopt($weatherForecastCh, CURLOPT_TIMEOUT, 2.5);
      $curlHandles[] = $weatherForecastCh;
  if(count($curlHandles) > $curlCount){
    $weatherForecastOn = true;
    $curlCount = count($curlHandles);
    }
}

//IP
if (preg_match('/\bip\b/i', $purl)) {
    $ipCh = curl_init('https://ipapi.co/'.$_SERVER['REMOTE_ADDR'].'/json/');
    curl_setopt($ipCh, CURLOPT_RETURNTRANSFER, true);
    curl_setopt($ipCh, CURLOPT_USERAGENT, 'Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13');
    curl_setopt($ipCh, CURLOPT_CONNECTTIMEOUT, 2);
    curl_setopt($ipCh, CURLOPT_TIMEOUT, 2.5);
      $curlHandles[] = $ipCh;
  if(count($curlHandles) > $curlCount){
    $ipOn = true;
    $curlCount = count($curlHandles);
    }
}
/*
//Crypto
$cryptoCh = curl_init('https://api.coingecko.com/api/v3/coins/'.$purl);
curl_setopt($cryptoCh, CURLOPT_RETURNTRANSFER, true);
curl_setopt($cryptoCh, CURLOPT_USERAGENT, 'Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13');
curl_setopt($cryptoCh, CURLOPT_CONNECTTIMEOUT, 2);
$curlHandles[] = $cryptoCh;
if(count($curlHandles) > $curlCount){
    $cryptoOn = true;
    $curlCount = count($curlHandles);
}
*/
//News
$newsCh = curl_init($WebNewsFile);
curl_setopt($newsCh, CURLOPT_RETURNTRANSFER, true);
curl_setopt($newsCh, CURLOPT_USERAGENT, 'Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13');
curl_setopt($newsCh, CURLOPT_CONNECTTIMEOUT, 2);
curl_setopt($newsCh, CURLOPT_TIMEOUT, 2.5);
$curlHandles[] = $newsCh;
if(count($curlHandles) > $curlCount){
    $newsOn = true;
    $curlCount = count($curlHandles);
}

//YouTube
$ytCh = curl_init($WebYoutubeFile);
curl_setopt($ytCh, CURLOPT_RETURNTRANSFER, true);
curl_setopt($ytCh, CURLOPT_USERAGENT, 'Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13');
curl_setopt($ytCh, CURLOPT_CONNECTTIMEOUT, 2);
curl_setopt($ytCh, CURLOPT_TIMEOUT, 2.5);
$curlHandles[] = $ytCh;
if(count($curlHandles) > $curlCount){
    $ytOn = true;
    $curlCount = count($curlHandles);
}

// Execute multi-curl requests
foreach ($curlHandles as $handle) {
    curl_multi_add_handle($multiHandle, $handle);
}

$running = null;
do {
    curl_multi_exec($multiHandle, $running);
    curl_multi_select($multiHandle);
} while ($running > 0);

// Close the multi-curl handle
curl_multi_close($multiHandle);

// Get response content and error codes for each request
foreach ($curlHandles as $handle) {
    if (curl_error($handle) === '') {
        $response = curl_multi_getcontent($handle);

        if(isset($redditOn) && !isset($redditObj)){$redditObj = json_decode($response, true);unset($redditOn);}
        elseif(isset($ddgOn) && $ddgObj == ''){$ddgObj = json_decode($response, true);unset($ddgOn);}

        elseif(isset($searchOn) && $obj == '' && !isset($g2obj)){
            if (!file_exists('disGoogle.txt')) {$obj = json_decode($response, true);}
            else{$g2obj = json_decode($response, true);}
        }
        elseif(isset($relatedOn) && !isset($related)){$related = json_decode($response, true);unset($relatedOn);}

        elseif(isset($wikiOn) && !isset($wikiobj)){$_SESSION[$purl.':-:wiki'] = $response;$wikiobj =explode('--]|[--',$response);$wikiTxt = $wikiobj[1];$wikiobj = json_decode($wikiobj[0],true); unset($wikiOn);}

        elseif(isset($defOn) && !isset($WordnikObj)){$WordnikObj = json_decode($response, true);unset($defOn);}
        elseif(isset($weatherOn) && !isset($OpenWeatherObj)){$OpenWeatherObj = json_decode($response, true);unset($weatherOn);}
        elseif(isset($weatherForecastOn) && !isset($OpenWeatherForecastObj)){$OpenWeatherForecastObj = json_decode($response, true);unset($weatherForecastOn);}
        elseif(isset($ipOn) && !isset($ipObj)){$ipObj = json_decode($response,true); unset($ipOn);}
        elseif(isset($cryptoOn) && !isset($cryptoData)){$cryptoData = json_decode($response,true);unset($cryptoOn);}

        elseif(isset($newsOn) && !isset($NewsObj)){$NewsObj = json_decode($response,true); unset($newsOn);}
        elseif(isset($ytOn) && !isset($YoutubeObj)){$YoutubeObj = json_decode($response,true); unset($ytOn);}
    }    
    curl_multi_remove_handle($multiHandle, $handle);
    }
curl_multi_close($multiHandle);
}
}
else{
    $obj = json_decode(file_get_contents($Googlefile), true);
    $redditObj = json_decode(file_get_contents($RedditFile),true);
    $ddgObj = json_decode(file_get_contents($DdgFile),true);
    $related = json_decode(file_get_contents('./Controller/dev/sug.json'), true);
    $OpenWeatherObj = json_decode(file_get_contents($OpenWeatherFile), true);
    $OpenWeatherForecastObj = json_decode(file_get_contents($OpenWeatherForecastFile), true);
    $ipObj = json_decode(file_get_contents('./Controller/dev/ip.json'), true);
    $NewsObj = json_decode(file_get_contents('./Controller/dev/news.json'), true);
    $YoutubeObj = json_decode(file_get_contents('./Controller/dev/yt.json'),true);
    $WordnikObj = json_decode(file_get_contents('./Controller/dev/def.json'), true);
}

    //Engines
    include 'engines/web/google.php';
    include 'engines/web/google2.php';
    include 'engines/web/qwant.php';
    include 'engines/web/brave.php';
    include 'engines/web/promo.php';
    include 'engines/web/prieco.php';
    //Addons
    include 'addons/wiki.php';
    include 'addons/hideQuery.php';
    include 'addons/reddit.php';
    include 'addons/related.php';
    include 'addons/news.php';
    include 'addons/yt.php';
    include 'addons/pHigh.php';
    //include 'addons/etsy.php';

    include 'addons/small/ip.php';

    if (preg_match('/\b(?:random|rand)\b/i', $purl)) {include 'addons/small/random.php';}
    if (preg_match('/\b(?:calculator|calc)\b/i', $purl) && !isset($_COOKIE['DisWid'])){include 'addons/small/calculator.php';}
    if (preg_match('/\b(?:color|col)\b/i', $purl) && !isset($_COOKIE['DisWid'])){include 'addons/small/color.html';}
    include 'addons/small/define.php';
    if(isset($OpenWeatherObj)){include 'addons/small/weather.php';}
    include 'addons/small/crypto.php';
    //include 'addons/small/convert.php';
    //include 'addons/small/didyoumean.php';
    include 'addons/summarizer/sum.php';

    //Next page
    function nextPage($purl,$page){
        if($page > 0){
            echo '<div class="nextPage" style="margin-left: calc(9vw + (clamp(552px, 675px, 90vw) - 210px) / 2);">';
            if(!isset($_COOKIE['hQuery'])){
            echo '<a href="/?q=',htmlspecialchars($purl, ENT_QUOTES | ENT_HTML5, 'UTF-8'),'&page=',$page-1,'">';
            }
            else{
                echo '<form method="POST" action="" style="display:inline;">
                <input type="hidden" name="q" value="', $purl,'">
                <input type="hidden" name="page" value="', $page-1 ,'">
                ';
            }
                echo '<button type="submit">⬅ Back</button>';
                if(!isset($_COOKIE['hQuery'])){echo '</a>';}
                else{echo '</form>';}
            echo '<p style="float:left;color:#8f8f8f;padding:10px;padding-top:22px;">Page: ',$page+1,'</p>';

            if(!isset($_COOKIE['hQuery'])){
                echo '<a href="/?q=',htmlspecialchars($purl, ENT_QUOTES | ENT_HTML5, 'UTF-8'),'&page=',$page+1,'">';
                }
                else{
                    echo '<form method="POST" action="" style="display:inline;">
                    <input type="hidden" name="q" value="', $purl,'">
                    <input type="hidden" name="page" value="', $page+1 ,'">
                    ';
                }
                echo'<button type="submit">Next ⮕</button>';
                if(!isset($_COOKIE['hQuery'])){echo '</a>';}
                else{echo '</form>';}
                echo '</div>
            <style>@media (max-width: 890px) {.nextPage {width:calc(50% + 115px);}}</style>';
        }
        else{
            echo '<div class="nextPage" style="margin-left: calc(9vw + (clamp(552px, 675px, 90vw) - 210px) / 2);">';
            if(!isset($_COOKIE['hQuery'])){
                echo '<a href="/?q=',htmlspecialchars($purl, ENT_QUOTES | ENT_HTML5, 'UTF-8'),'&page=',$page+1,'">';
                }
                else{
                    echo '<form method="POST" action="">
                    <input type="hidden" name="q" value="', $purl,'">
                    <input type="hidden" name="page" value="', $page+1 ,'">
                    ';
                }    
                echo '<button type="submit">Next ⮕</button>';

                if(!isset($_COOKIE['hQuery'])){echo '</a>';}
                else{echo '</form>';}
            echo '</div>
            <style>@media (max-width: 890px) {.nextPage {width:calc(50% + 40px);}}</style>';
        }
    }
    
    $loaded = array_fill(0, 7, false);
    if($page > 0){
        $results = qwant(null, $loaded, $Bpurl, $page);
        foreach($results as &$rs){
            echo $rs;
        }
        nextPage($purl, $page);
        return;
    }

    ##
    #Printing
    ##
    //Addons
    $simImg = $wiki = $news = $reddit = $youtube = $relatedP = '';
    if(isset($_COOKIE['hQuery'])){$hideQueryCopy = hideQuery($Bpurl);}
    else{$hideQueryCopy = null;}
    if(isset($wikiobj)){$wiki = wiki($wikiobj, $wikiTxt, $ddgObj, $ImpProfiles ?? null, $hideQueryCopy); $simImg = $wiki[1]; $wiki = $wiki[0];}
    if(isset($NewsObj)){$news = search_news($NewsObj);}
    if(isset($YoutubeObj)){$youtube = youtube($YoutubeObj);}

    $privateAltOut = pHigh($purl);if($privateAltOut != ''){$loaded[6] = true;}
    $reddit = search_reddit($redditObj);
    $relatedP = related($related, $simImg);
    $elseWhere = '<div class="findelsewhere"><p style="float:left;">Search with </p>
    <div style="min-width:340px; float:left;">
    <a href="https://startpage.com/do/metasearch.pl?query='.$purl.'"><button>Startpage</button></a>
    <a href="https://duckduckgo.com/?q='.$purl.'"><button>DuckDuckGo</button></a>
    <a href="https://search.brave.com/search?q='.$purl.'"><button>Brave</button></a>
    <a href="https://www.mojeek.com/search?q='.$purl.'"><button>Mojeek</button></a>
        </div>
    </div>';

    //Check which addons loaded
    if($wiki != ''){$loaded[1] = true;}
    if($news != ''){$loaded[2] = true;}
    if($reddit != ''){$loaded[3] = true;}
    if($youtube != ''){$loaded[4] = true;}
    if($related != ''){$loaded[5] = true;}

    //Turn of Google if rate limite
    if(str_starts_with(json_encode($obj), '{"error":{"code":429,')){
        if(date('H') >= 7){
        file_put_contents('disGoogle.txt',date('d'));
        }
        else{
            file_put_contents('disGoogle.txt',date('d')-1);
        }
    }

    //Engines
    if($obj != ''){$results = google($obj, $loaded);}
    if(!isset($results[0]) && isset($g2obj)){$results = google2($g2obj, $loaded);}
    if(isset($BraveObj) or !isset($results[0])){$results = brave($BraveObj, $loaded, $Bpurl);}
    if(!isset($results[0])){$results = qwant($QWantObj, $loaded, $Bpurl, $page);}

    if(!isset($_COOKIE['safe']) && !isset($_COOKIE['time'])){
    if($obj != ''){$_SESSION[$purl] = 'obj +=+ '.json_encode($obj);}
    if($g2obj != ''){$_SESSION[$purl] = 'g2obj +=+ '.json_encode($g2obj);}
    if($news != ''){$_SESSION[$purl.':-:news'] = json_encode($NewsObj);}
    if($youtube != ''){$_SESSION[$purl.':-:yt'] = json_encode($YoutubeObj);}
    if($reddit != ''){$_SESSION[$purl.':-:red'] = json_encode($redditObj);}
    if($related != ''){$_SESSION[$purl.':-:rel'] = json_encode($related);}
    }
    //Print
        //Query URL
    if(preg_match('/^(https?:\/\/)?([a-zA-Z0-9-]+\.)+[a-zA-Z0-9]{2,}(:\d{2,5})?(\/\S*)*$/', preg_replace('/\s+$/u', '', $purl))){
        echo "<div class='output' style='border-radius:20px;border: solid 3px green;margin-bottom:15px;padding: 5px;display:flex;flex-direction: column;flex-wrap: wrap;justify-content: space-around;align-items: center;'>
        <p style='text-align:center;'><b>Looks like you're searching for a website!</b><br>
        If you'd like us to redirect you there, simply click the button below:</p>
        <a style='padding-bottom:5px;' href='";if(strpos($purl, 'https://') === false && strpos($purl, 'http://') === false){echo 'https://';}
            echo $purl,"'";
        if(isset($_COOKIE['new'])){echo 'target="_blank"';}
        echo '><button class="redirectMe">Redirect me!</button></a>
        </div>';
    }
    if(isset($results))
    {
    echo $results[0], $privateAltOut,
    $wiki,
    $results[1],
    $news,
    $results[2], $results[3],
    $reddit,
    $results[4], $results[5],
    $youtube,
    $results[6], $results[7],
    $relatedP;
    if(count($results)>7){
        for ($i = 8; $i < count($results); $i++) {
            echo $results[$i];
          }
    }
    echo $elseWhere;
   if(!$dev){

    $priecoTime = microtime(true);
    if(isset($_SESSION[$purl.':-:pri'])){$priecoResults = explode('-<+>-', $_SESSION[$purl.':-:pri']);}
    else{$priecoResults = prieco($purl, $conn, $loc, $lang);}
    $priecoTime = microtime(true) - $priecoTime;

    $pRes = '';
    for($i = 0; $i < 10; $i++){
        echo $priecoResults[$i];
        $pRes .= $priecoResults[$i].'-<+>-';
    }
    $_SESSION[$purl.':-:pri'] = $pRes;
}
    nextPage($purl,$page);
}
else{
    echo '<div style="width: 100vw;
    height: 50vh;
    display: flex;
    align-content: center;
    flex-wrap: wrap;
    justify-content: center;
    align-items: center;
    filter:brightness(0)invert(0.5);"><h1>No results found!</h1><img src="/View/icon/no_link.svg"
    style="width:100px;height:auto;"></div>';
}
}
if ($type === 'image') {

     //Next page
     function nextPage($purl,$page, $size, $color, $type, $time, $right){
        if($page > 0){
            echo '<div class="nextPage" style="width: 100vw;display: flex;justify-content: center;"><a href="/?image&imgsize=' , $size , '&imgcolor=' , $color , '&imgtype=' , $type , '&imgtime=' , $time , '&imglicence=' , $right , '&q=' , htmlspecialchars($purl, ENT_QUOTES | ENT_HTML5, 'UTF-8'),'&page=',$page-1,'"><button>⬅ Back</button></a>
            <p style="float:left;color:#8f8f8f;padding:10px;padding-top:22px;">Page: ',$page+1,'</p>
            <a href="/?image&imgsize=' , $size , '&imgcolor=' , $color , '&imgtype=' , $type , '&imgtime=' , $time , '&imglicence=' , $right , '&q=' , htmlspecialchars($purl, ENT_QUOTES | ENT_HTML5, 'UTF-8'),'&page=',$page+1,'"><button>Next ⮕</button></a></div>
            <style>@media (max-width: 890px) {.nextPage {width:calc(50% + 115px);}}</style>';
        }
        else{
            echo '
            <div class="nextPage" style="width: 100vw;display: flex;justify-content: center;"><a href="/?image&imgsize=' , $size , '&imgcolor=' , $color , '&imgtype=' , $type , '&imgtime=' , $time , '&imglicence=' , $right , '&q=' , htmlspecialchars($purl, ENT_QUOTES | ENT_HTML5, 'UTF-8'),'&page=',$page+1,'"><button>Next ⮕</button></a></div>
            <style>@media (max-width: 890px) {.nextPage {width:calc(50% + 40px);}}</style>';
        }
    }

    if ($pixabay) {
        echo '<form method="post" action="">
        <input type="submit" style="margin-left:9vw;"class="imgtoolsOption" value="Back" name="imgback" />
        </form>';
        $Pix = curl_init();
        curl_setopt($Pix, CURLOPT_URL, $PixabayFile);
        curl_setopt($Pix, CURLOPT_USERAGENT, 'Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13');
        curl_setopt($Pix, CURLOPT_CONNECTTIMEOUT, 2.5);
        curl_setopt($Pix, CURLOPT_RETURNTRANSFER, true);

        $PixObj = json_decode(curl_exec($Pix), true);
        curl_close($Pix);

        echo '<div style="display:flex;margin-top:30px;flex-wrap:wrap;"><br>        
        <div>
         <div tabindex="0" class="imgoutbtn">
             <img src ="/View/img/pix.svg"class="imgout"><p style="color:black;padding-left: 10px;padding-right: 10px;">
         </div>
 
        
         <div class="bigimgout">           
                 <img src ="/View/img/pix.svg">
                 <br>
                 <h3>Thank you Pixabay for providing PriEco with these images</h3><br>
                 <p>From website: https://pixabay.com/</p><br>
                 <div class="bigimgbtn"><a href="https://pixabay.com/"><button class="imgtoolsOption">Go to website</button></a><br>
                 <a href="https://pixabay.com/static/img/logo.svg"> <button class="imgtoolsOption">Go to image</button></a></div>  
                 <div style="display: flex;justify-content: center;"><button class="bigimgclose imgtoolsOption">«Close</button></div>                      
         </div>
       </div>
         ';

        foreach ($PixObj['hits'] as &$item) {
            echo '
           <div class="imgoutdiv">
           <div tabindex="0"  class="imgoutbtn">
               <img style="max-height: 100%;" src="/Controller/functions/proxy.php?q=', $item['webformatURL'], '"class="imgout">
               </div>
               
           <div class="bigimgout">           
           <img src ="/Controller/functions/proxy.php?q=', $item['largeImageURL'], '">
           <br>
           <h3>From user: ', $item['user'], '</h3><br>
           <p>From website: ', $item['largeImageURL'], '</p><br>
           <div class="bigimgbtn"><a href="', $item['pageURL'], '"><button class="imgtoolsOption">Go to website</button></a><br>
           <a href="', $item['largeImageURL'], '"> <button class="imgtoolsOption">Go to image</button></a></div>         
           <div style="display: flex;justify-content: center;"><button class="bigimgclose imgtoolsOption">«Close</button></div>         
           </div>      
         </div>
             ';
        }
        echo '</div>';
        return;
    }

    include './Model/imgset.php';

    if(!$dev){
        $Qimg = '{"status":"error","data":{"error_code":24}}';

        if (!file_exists('disImg.txt')) {
            $mh = curl_multi_init();
            $handles = array();
        
            // Google request
            if($page == 0){
            $gCh = curl_init('https://www.googleapis.com/customsearch/v1?key='.$_ENV['GOOGLE_IMAGE_API_KEY'].'&cx='.$_ENV['GOOGLE_IMAGE_CX_KEY'].'&hl=all&gl=all&safe='.$safe.'&q='.$Bpurl.'&searchType=image');
            curl_setopt($gCh, CURLOPT_POST, true);
            curl_setopt($gCh, CURLOPT_RETURNTRANSFER, true);
            curl_setopt($gCh, CURLOPT_TIMEOUT, 3);
            curl_multi_add_handle($mh, $gCh);
            $handles[] = $gCh;
            }
            // Bing request
            $bCh = curl_init();
            $bUrl = 'https://api.qwant.com/v3/search/images/?count=75&offset=' . $page * 82 . '&uiv=1&locale=en_US&size=' . $imgsize . '&color=' . $imgcolor . '&imagetype=' . $imgtype . '&freshness=' . $imgtime . '&license=' . $imgright . '&q=' . $Bpurl;
            curl_setopt($bCh, CURLOPT_URL, $bUrl);
            curl_setopt($bCh, CURLOPT_USERAGENT, 'Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13');
            curl_setopt($bCh, CURLOPT_CONNECTTIMEOUT, 2.5);
            curl_setopt($bCh, CURLOPT_RETURNTRANSFER, true);
            curl_multi_add_handle($mh, $bCh);
            $handles[] = $bCh;
        
            // Execute the requests concurrently
            $active = null;
            do {
                curl_multi_exec($mh, $active);
            } while ($active > 0);
        
            // Get the responses
            if($page == 0){$gResponse = curl_multi_getcontent($gCh);}
            $bResponse = curl_multi_getcontent($bCh);
        
            // Close the handles
            foreach ($handles as $handle) {
                curl_multi_remove_handle($mh, $handle);
                curl_close($handle);
            }
            curl_multi_close($mh);
        
            // Process the responses
            if($page == 0){$Gimg = json_decode($gResponse, true);}
            $Qimg = $bResponse;
        }
        
        // Bing failed
        if ($Qimg == '{"status":"error","data":{"error_code":24}}' || $Qimg == '{"status":"error","data":{"error_code":20}}') {
            if (!file_exists('disImg.txt')) {
                file_put_contents('disImg.txt', date('d') . ' ' . date('H'));
            }
        
            $mh = curl_multi_init();
            $handles = array();
        
            // Bing request
            $bUrl = 'https://librex.jojoyou3.repl.co/qwant.php?offset=' . $page * 82 . '&imgsize=' . $imgsize . '&imgcolor=' . $imgcolor . '&imgtype=' . $imgtype . '&imgtime=' . $imgtime . '&imgright=' . $imgright . '&q=' . $Bpurl;
        
            $BCurl = curl_init();
            curl_setopt($BCurl, CURLOPT_URL, $bUrl);
            curl_setopt($BCurl, CURLOPT_USERAGENT, 'Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13');
            curl_setopt($BCurl, CURLOPT_CONNECTTIMEOUT, 2.5);
            curl_setopt($BCurl, CURLOPT_RETURNTRANSFER, true);
            curl_multi_add_handle($mh, $BCurl);
            $handles[] = $BCurl;
        
            if (!isset($Gimg) && $page == 0) {
                $gCh = curl_init('https://www.googleapis.com/customsearch/v1?key='.$_ENV['GOOGLE_IMAGE_API_KEY'].'&cx='.$_ENV['GOOGLE_IMAGE_CX_KEY'].'&hl=all&gl=all&safe='.$safe.'&q='.$Bpurl.'&searchType=image');
                curl_setopt($gCh, CURLOPT_POST, true);
                curl_setopt($gCh, CURLOPT_RETURNTRANSFER, true);
                curl_setopt($gCh, CURLOPT_TIMEOUT, 3);
                curl_multi_add_handle($mh, $gCh);
                $handles[] = $gCh;
            }
        
            $active = null;
        
            do {
                curl_multi_exec($mh, $active);
            } while ($active);
        
            $Qimg = curl_multi_getcontent($BCurl);
            if (!isset($Gimg)) {$Gimg = json_decode(curl_multi_getcontent($gCh), true);}

            foreach ($handles as $handle) {
                curl_multi_remove_handle($mh, $handle);
                curl_close($handle);
            }
        
            curl_multi_close($mh);
        
            $active = null;
        
            do {
                curl_multi_exec($mh, $active);
            } while ($active);
        
            curl_multi_close($mh);
        }
    }
    else{
        $Qimg = file_get_contents('./Controller/dev/img.json');
    }
        $Qimg = json_decode($Qimg, true);
        

echo '<div style="display:flex;margin-top:30px;flex-wrap:wrap;"><br>';

    foreach($Gimg['items'] as &$item){
        echo '
        <div class="imgoutdiv">
         <div tabindex="0" class="imgoutbtn" style="aspect-ratio:',$item['image']['width'],'/',$item['image']['height'],';">
             <div style="background-image: url(/Controller/functions/proxy.php?q=',$item['image']['thumbnailLink'], ');aspect-ratio:',$item['image']['width'],'/',$item['image']['height'],';" class="imgout"></div>
             </div>
         
         
 
         <div class="bigimgout">           
         <img src ="/Controller/functions/proxy.php?q=',$item['image']['thumbnailLink'], '" data-src="/Controller/functions/proxy.php?q=',$item['link'],'"';
         if(!isset($_COOKIE['DisHImg'])){echo 'style="filter: blur(5px);"';}
         echo '>
         <br>
         <h3>', $item['title'], '</h3><br>
         <p>From website: ', $item['displayLink'], '</p><br>
         <div class="bigimgbtn"><a href="https://', $item['displayLink'], '"><button class="imgtoolsOption">Go to website</button></a><br>
         <a href="',$item['link'], '"> <button class="imgtoolsOption">Go to image</button></a></div>  
         <div style="display: flex;justify-content: center;"><button class="bigimgclose imgtoolsOption">«Close</button></div>   
         </div>      
       </div>
         ';
         echo 'here';
    }
    foreach ($Qimg['data']['result']['items'] as &$item) {
        if (!isset($item['media']) or !isset($item['media_preview'])) {
            continue;
        }

        echo '
       <div class="imgoutdiv">
        <div tabindex="0" class="imgoutbtn" style="aspect-ratio:',$item['thumb_width'],'/',$item['thumb_height'],';">
            <img src="https://search.jojoyou.org/Controller/functions/img_proxy.php?q=',$item['media'], '" style="aspect-ratio:',$item['thumb_width'],'/',$item['thumb_height'],';" class="imgout">
        </div>
        <a style="color: var(--linkColor); cursor:pointer;text-decoration:none;" href="',$item['url'],'"';if (isset($_COOKIE['new'])) {echo 'target="_blank';}echo'>
        <p>';
            $pieces = parse_url($item['url']);
            $domain = isset($pieces['host']) ? $pieces['host'] : $pieces['path'];
            if (preg_match('/(?P<domain>[a-z0-9][a-z0-9\-]{1,63}\.[a-z\.]{2,6})$/i', $domain, $regs)) {
                echo $regs['domain'];
            }
            echo '</p></a>
            <p>',$item['title'],'</p>
        
        

        <div class="bigimgout">           
        <img src ="https://search.jojoyou.org/Controller/functions/img_proxy.php?q=',$item['media'], '" data-src="/Controller/functions/proxy.php?q=',$item['media'],'"';
        if(!isset($_COOKIE['DisHImg'])){echo 'style="filter: blur(5px);"';}
        echo '>
        <br>
        <h3>', $item['title'], '</h3><br>
        <p>From website: ', $item['url'], '</p><br>
        <div class="bigimgbtn"><a href="', $item['url'], '"><button class="imgtoolsOption">Go to website</button></a><br>
        <a href="',$item['media'], '"> <button class="imgtoolsOption">Go to image</button></a></div>  
        <div style="display: flex;justify-content: center;"><button class="bigimgclose imgtoolsOption">«Close</button></div>   
        </div>      
        </div>
        ';
    }
    echo '</div></div>';
   
    if(!isset($_COOKIE['DisHImg'])){
    echo '<script>
    const lazyImages = document.querySelectorAll("img[data-src]");

    const observer = new IntersectionObserver((entries, observer) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting) {
          const lazyImage = entry.target;
    
          lazyImage.src = lazyImage.dataset.src;
    
          lazyImage.removeAttribute("data-src");
    
          observer.unobserve(lazyImage);
    
          lazyImage.addEventListener("load", () => {
            lazyImage.classList.add("bigimage_loaded");
          });
        }
      });
    });
    
    // Start observing each lazy image
    lazyImages.forEach((lazyImage) => {
      observer.observe(lazyImage);
    });
    </script>';
}
    echo nextPage($purl, $page, $imgsize, $imgcolor,$imgtype, $imgtime, $imgright);
}
if ($type == 'video') {
    if (!$dev) {
        $YoutubeCurl = curl_init();
        curl_setopt($YoutubeCurl, CURLOPT_URL, $YoutubeFile);
        curl_setopt($YoutubeCurl, CURLOPT_USERAGENT, 'Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13');
        curl_setopt($YoutubeCurl, CURLOPT_CONNECTTIMEOUT, 2.5);
        curl_setopt($YoutubeCurl, CURLOPT_RETURNTRANSFER, true);
        $YoutubeObj = json_decode(curl_exec($YoutubeCurl), true);
        curl_close($YoutubeCurl);
    } else {
        $YoutubeObj =  json_decode(file_get_contents($YoutubeFile), true);
    }
    echo '<div style="display:flex;margin-top:30px;flex-wrap:wrap;"><br>';
    foreach ($YoutubeObj['items'] as &$item) {       
        echo '

                    <div class="imgoutdiv">
                    <a style="text-decoration: none;" href="https://www.youtube.com/watch?v=' . $item['id']['videoId'] . '"'; if (isset($_COOKIE['new'])) {
                        echo 'target="_blank"';
                    }
                    echo'>
                    <button class="imgoutbtn" style="flex-direction: column;align-items: center;">
            <img alt="ㅤ"src ="/Controller/functions/proxy.php?q=',  $item['snippet']['thumbnails']['medium']['url'], '"class="imgout" style="border-radius:20px;"loading="lazy">
            <div class="imgoutlink">
            <p style="font-size:12px;font-weight:bold;">',  substr($item['snippet']['title'], 0, 20) . '...</p>
            <p style="font-size:10px;">', substr($item['snippet']['description'], 0, 20) . '...</p>
            </div>
            </button>
        </a>
        </div>
              ';
    }
    echo '</div>';
}

if ($type == 'news') {
    if (!$dev) {
        $NewsCurl = curl_init();
        curl_setopt($NewsCurl, CURLOPT_URL, $NewsFile);
        curl_setopt($NewsCurl, CURLOPT_USERAGENT, 'Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13');
        curl_setopt($NewsCurl, CURLOPT_CONNECTTIMEOUT, 2.5);
        curl_setopt($NewsCurl, CURLOPT_RETURNTRANSFER, true);
        $NewsObj = json_decode(curl_exec($NewsCurl), true);
        curl_close($NewsCurl);
    } else {
        $NewsObj =  json_decode(file_get_contents($NewsFile), true);
    }
    $did = false;
    foreach ($NewsObj['articles'] as &$news) {
        $title = str_replace('<', '', $news['title']);
        $title = str_replace('>', '', $title);
        $desc = str_replace('<', '', $news['description']);
        $desc = str_replace('>', '', $desc);
        echo '<div class="output"';
        if (!$did) {
            echo 'style="border-top-left-radius: 20px;
            border-top-right-radius: 20px;"';
            $did = true;
        }
        echo ' id="output">';
        if ($news['urlToImage'] != '' and !isset($_COOKIE['datasave'])) {
            echo '<img loading="lazy" alt="‎" src="/Controller/functions/proxy.php?q=', $news['urlToImage'], '" class="OutSideImg">';
        }
        echo '<a ';
        if (isset($_COOKIE['new'])) {
            echo 'target="_blank"';
        }
        echo 'href="', $news['url'], '">
    <p style="color: gray;
    font-size: 14px;">', $news['source']['name'], '</p>
    <p class="OutTitle" style="font-size:16px;padding-top:0;padding-bottom: 5px;">', $title, '</p></a>
    <p class="snippet"style="font-size:12px;padding-bottom: 18px;">--', $desc, '</p>
    ';
        if ($providers == 'on') {
            echo '<p class="resProvider">Newsapi</p>';
        }
        echo '</div>';
    }
}
