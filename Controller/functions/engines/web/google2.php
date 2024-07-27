<?php
function google2($g2obj, $loaded){
$google2[]=null;

    $i = 0;
    $bottomBorder = count($g2obj)-1;
    foreach ($g2obj as &$item) {
        if(!isset($item['url'])){--$bottomBorder;continue;}
    $google2[$i] = '<div class="output" id="output">';   
    
$gurl = str_replace('/',' > ',str_replace('https://','',str_replace('http://','',str_replace('www.','', $item['url']))));
if ( substr_compare($gurl, ' > ', -3) === 0 ) {
$gurl = substr($gurl, 0, -3);
}

if (strpos($item['url'], 'https://') !== false && !isset($_COOKIE['datasave'])) {
$google2[$i] .= '<img class="Outfavicon" alt="‎" loading="lazy" src="/Controller/functions/proxy.php?q=https://judicial-peach-octopus.b-cdn.net/'. get_string_betweens($item['url'], 'https://', '/'). '">';
}
$google2[$i] .= '<a ';
if (isset($_COOKIE['new'])) {
$google2[$i] .= 'target="_blank"';
}
$google2[$i] .= 'href="'. $item['url']. '" rel="noopener noreferrer" data-sxpr-url>';
$google2[$i] .= '<p class="OutTitle">'. $item['title']. '</p></a>
<div class="resLink">'. $gurl . '<img src="View/icon/dots_vertical.svg" class="filterImage resOptions">';
if (!isset($_COOKIE['DisWid'])) {
    $google2[$i] .= '<div class="resOptionsGroup">
    <a href="https://web.archive.org/web/*/'.$item['url'].'" rel="noopener noreferrer"';if (isset($_COOKIE['new'])) {$google2[$i] .= 'target="_blank"';}$google2[$i].='><img class="filterImage sumOpen width32" src="View/icon/archive.svg"></a>
    <a href="proxy/?url='.$item['url'].'" ';if (isset($_COOKIE['new'])) {$google2[$i] .= 'target="_blank"';}$google2[$i].='><img class="filterImage sumOpen opacity10 blueIcon" src="View/icon/mask.svg"></a>
    <img class="filterImage width33 sumOpen" id="sumRes" data-url="'.$item['url'].'" src="View/icon/circle-info.svg">
    </div>';
}
$google2[$i] .= '</div>
<div id="sumResOut" class="sumOut snippet">
<blockquote id="sumOut"></blockquote>
</div>';

if(isset($item['description'])){$google2[$i] .= '<p class="snippet" id="snippet">'. $item['description']. '</p>';}
if (isset($_COOKIE['providers'])) {
$google2[$i] .= '<p class="resProvider">google2</p>';
}
$google2[$i].='</div>';
++$i;
    }
return $google2;
}