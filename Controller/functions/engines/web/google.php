<?php
function google($obj, $loaded)
{
    $google[]=null;
    $i = 0;
    foreach ($obj['items'] as &$item) {
    $google[$i] = '<div class="output" id="output">';

$gurl = str_replace('/',' > ',str_replace('https://','',str_replace('http://','',str_replace('www.','', explode('?', $item['link'])[0]))));
if ( substr_compare($gurl, ' > ', -3) === 0 ) {
$gurl = substr($gurl, 0, -3);
}

if (strpos($item['link'], 'https://') !== false && !isset($_COOKIE['datasave'])) {
$google[$i] .= '<img class="Outfavicon" alt="‎" loading="lazy" src="/Controller/functions/proxy.php?q=https://judicial-peach-octopus.b-cdn.net/'. get_string_betweens($item['link'], 'https://', '/'). '">';
}
$google[$i] .= '<a ';
if (isset($_COOKIE['new'])) {$google[$i] .= 'target="_blank"';}
$google[$i] .= 'href="'. (isset($_GET['tabs']) ? 'Controller/functions/saveTab.php?tab='.$_GET['tab'].'&url='.urlencode($item['link']) : $item['link']). '" rel="noopener noreferrer" data-sxpr-link>';
$google[$i] .= '<p class="OutTitle">'.strip_tags($item['title']). '</p></a>
<div class="resLink"><p>'. $gurl . '</p>';
if (!isset($_COOKIE['DisWid'])) {
    $google[$i] .= '
    <div class="flex alignC justConC">
    <div class="resOptionsGroup">
    <a href="https://web.archive.org/web/*/'.$item['link'].'" rel="noopener noreferrer"';if (isset($_COOKIE['new'])) {$google[$i] .= 'target="_blank"';}$google[$i].='><img class="filterImage sumOpen width32" src="View/icon/archive.svg"></a>
    <a href="proxy/?url='.$item['link'].'" ';if (isset($_COOKIE['new'])) {$google[$i] .= 'target="_blank"';}$google[$i].='><img class="filterImage sumOpen opacity10 blueIcon" src="View/icon/mask.svg"></a>
    <img class="filterImage width33 sumOpen" id="sumRes" data-url="'.$item['link'].'" src="View/icon/circle-info.svg">
    </div>
    <img src="View/icon/dots_vertical.svg" class="filterImage resOptions">
    </div>';
}
$google[$i] .= '</div>
 <div id="sumResOut" class="sumOut snippet">
    <blockquote id="sumOut"></blockquote>
</div>
';


if(isset($item['snippet'])){$google[$i] .= '
    <p class="snippet" id="snippet">'.strip_tags($item['snippet']). '</p>';}
if (isset($_COOKIE['providers'])) {
$google[$i] .= '<p class="resProvider">Google</p>';
}
$google[$i].='</div>';
++$i;
    }
return $google;
}