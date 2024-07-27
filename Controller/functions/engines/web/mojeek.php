<?php
function mojeek($mojeekObj, $loaded){
$mojeek[]=null;

    $i = 0;

    foreach ($mojeekObj['response']['results'] as &$item) {
    $mojeek[$i] = '<div class="output" id="output">';
    
$gurl = str_replace('/',' > ',str_replace('https://','',str_replace('http://','',str_replace('www.','', $item['url']))));
if ( substr_compare($gurl, ' > ', -3) === 0 ) {
$gurl = substr($gurl, 0, -3);
}

if (strpos($item['url'], 'https://') !== false && !isset($_COOKIE['datasave'])) {
$mojeek[$i] .= '<img class="Outfavicon" alt="‎" loading="lazy" src="/Controller/functions/proxy.php?q=https://judicial-peach-octopus.b-cdn.net/'. get_string_betweens($item['url'], 'https://', '/'). '">';
}
$mojeek[$i] .= '<a ';
if (isset($_COOKIE['new'])) {
$mojeek[$i] .= 'target="_blank"';
}
$mojeek[$i] .= 'href="'. $item['url']. '" rel="noopener noreferrer" data-sxpr-link>';
$mojeek[$i] .= '<p class="OutTitle">'. $item['title']. '</p></a>
<div class="resLink">'. $gurl. '<img src="View/icon/dots_vertical.svg" class="filterImage resOptions">';
if (!isset($_COOKIE['DisWid'])) {
    $mojeek[$i] .= '<div class="resOptionsGroup">
    <a href="https://web.archive.org/web/*/'.$item['url'].'" rel="noopener noreferrer"';if (isset($_COOKIE['new'])) {$mojeek[$i] .= 'target="_blank"';}$mojeek[$i].='><img class="filterImage sumOpen width32" src="View/icon/archive.svg"></a>
    <a href="proxy/?url='.$item['url'].'" ';if (isset($_COOKIE['new'])) {$mojeek[$i] .= 'target="_blank"';}$mojeek[$i].='><img class="filterImage sumOpen opacity10 blueIcon" src="View/icon/mask.svg"></a>
    <img class="filterImage width33 sumOpen" id="sumRes" data-url="'.$item['url'].'" src="View/icon/circle-info.svg">
    </div>';
}
$mojeek[$i] .= '</div>
<div id="sumResOut" class="sumOut snippet">
        <blockquote id="sumOut"></blockquote>
    </div>
    ';

if(isset($item['desc'])){$mojeek[$i] .= '<p class="snippet" id="snippet">'. $item['desc']. '</p>';}
if (isset($_COOKIE['providers'])) {
$mojeek[$i] .= '<p class="resProvider">Mojeek</p>';
}
$mojeek[$i] .= '</div>';
++$i;
    }
return $mojeek;
}