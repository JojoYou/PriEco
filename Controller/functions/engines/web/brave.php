<?php
function getBetween($string, $start = "", $end = ""){
    if (strpos($string, $start)) { // required if $start not exist in $string
        $startCharCount = strpos($string, $start) + strlen($start);
        $firstSubStr = substr($string, $startCharCount, strlen($string));
        $endCharCount = strpos($firstSubStr, $end);
        if ($endCharCount == 0) {
            $endCharCount = strlen($firstSubStr);
        }
        return substr($firstSubStr, 0, $endCharCount);
    } else {
        return '';
    }
}

function brave($BraveObj, $loaded, $Bpurl){

//add headers
if(file_exists('disGoogle.txt') && !isset($BraveObj)){
    $ch = curl_init();
$headers = array(
    'Accept: text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8',
    'Accept-Language: en-US,en;q=0.8',
    'Cache-Control: max-age=0',
    'Connection: keep-alive',
    'Upgrade-Insecure-Requests: 1',
    'User-Agent: Mozilla/5.0 (Windows NT 6.1; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/56.0.2924.87 Safari/537.36',
);
curl_setopt($ch, CURLOPT_HTTPHEADER, $headers);
curl_setopt($ch, CURLOPT_URL, 'https://search.brave.com/search?q='.$Bpurl);
curl_setopt($ch, CURLOPT_CUSTOMREQUEST, 'GET');
curl_setopt($ch, CURLOPT_RETURNTRANSFER, 1);

$BraveObj = curl_exec($ch);
curl_close($ch);
}
if(!isset($BraveObj)){return;}
$snippet = str_get_html($BraveObj)->find('div.snippet');

if(isset($snippet)){
$i=0;
$CountBraveSnippets = count($snippet);
if($CountBraveSnippets >= 20){
    $j=0;
    foreach ($snippet as $snip) {
        $href = $snip->find('a', 0);
        if($href == null){
            break;
        }
        ++$j;
    }
    $CountBraveSnippets = $j-1;
}
else{
    $CountBraveSnippets -= 1;
}
$i = 0;
$brave[] = null;
foreach ($snippet as $snip) {
    $href = $snip->find('a', 0);
    if($href == null){
        break;
    }

    if($href->find('div.favicon-wrapper', 0)->outertext != null){
    $href->find('div.favicon-wrapper', 0)->outertext = '';
    }
    $url = $href->href;
    if(!filter_var($url, FILTER_VALIDATE_URL)){
        continue;
    }
    $description = $snip->find('p.snippet-description', 0);
    
    $brave[$i] = '<div class="';
    switch ($i){
        case 0:
            if(!$loaded[0] && $loaded[1]) {$brave[$i] .= ' mBorderBoth2 mBorderTop ';}
            elseif(!$loaded[0]){$brave[$i] .= ' mBorderTop ';}
            elseif($loaded[1]){$brave[$i] .= ' mBorderBottom2 ';}
            break;
        case 1:
            if($loaded[1]) {$brave[$i] .= ' mBorderTop2 ';}
            if($loaded[2]) {$brave[$i] .= ' mBorderBottom ';}
            break;
        case 2:
            if($loaded[2]) {$brave[$i] .= ' mBorderTop ';}
           break;
        case 3:
            if($loaded[3]) {$brave[$i] .= ' mBorderBottom ';}
            break;
        case 4:
            if($loaded[3]) {$brave[$i] .= ' mBorderTop ';}
            break;
        case 5:
            if($loaded[4]) {$brave[$i] .= ' mBorderBottom ';}
            break;
        case 6:
            if($loaded[4]) {$brave[$i] .= ' mBorderTop ';}
            break;
        case 7:
            if($loaded[5]) {$brave[$i] .= ' mBorderBottom ';}
            break;
        case 8:
            if($loaded[5]) {$brave[$i] .= ' mBorderTop ';}
            break;
        case $CountBraveSnippets:
            $brave[$i] .= ' mBorderBottom ';
            break;
    }

        $brave[$i] .= ' output" id="output">';

    if(strpos($url, 'https://') !== false){$brave[$i] .= '<img class="Outfavicon" alt="‎" loading="lazy" src="https://judicial-peach-octopus.b-cdn.net/'.get_string_betweens($url, 'https://', '/').'">';}
        $brave[$i] .= '<a ';
        if (isset($_COOKIE['new'])) {
            $brave[$i] .= 'target="_blank"';
        }
        $gurl = str_replace('/',' > ',str_replace('https://','',str_replace('http://','',str_replace('www.','', $url))));
            if ( substr_compare($gurl, ' > ', -3) === 0 ) {
            $gurl = substr($gurl, 0, -3);
            }
        $brave[$i] .= 'href="' . $url . '">';
        $brave[$i] .= '<p class="OutTitle">' . $href . '</p></a>
    <p class="resLink">' . $gurl . '</p>
    <p class="snippet">--' . $description->innertext . '</p>
    ';
    if (isset($_COOKIE['providers'])) {
        $brave[$i] .= '<p class="resProvider">Brave</p>';
    }
    $brave[$i] .= '</div>';
    ++$i;
}
}
return $brave;
}