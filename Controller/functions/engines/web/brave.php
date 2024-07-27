<?php
function brave($BraveObj, $loaded, $Bpurl){
    if(!isset($BraveObj['web']['results'])){return '';}
    $CountBraveSnippets = count($BraveObj['web']['results'])-1;
    $i=0;
    foreach($BraveObj['web']['results'] as &$item){
        $brave[$i] = '<div class="output" id="output">';
        
        $burl = str_replace('/',' > ',str_replace('https://','',str_replace('http://','',str_replace('www.','', $item['url']))));
        if ( substr_compare($burl, ' > ', -3) === 0 ) {
        $burl = substr($burl, 0, -3);
        }
        
        if (isset($item['thumbnail']['src']) && !isset($_COOKIE['datasave'])) {
        $brave[$i] .= '<img loading="lazy" alt="‎" src="/Controller/functions/proxy.php?q='. $item['thumbnail']['src']. '" class="OutSideImg">';
        }
        if (strpos($item['url'], 'https://') !== false && !isset($_COOKIE['datasave'])) {
        $brave[$i] .= '<img class="Outfavicon" alt="‎" loading="lazy" src="/Controller/functions/proxy.php?q=https://judicial-peach-octopus.b-cdn.net/'. get_string_betweens($item['url'], 'https://', '/'). '">';
        }
        $brave[$i] .= '<a ';
        if (isset($_COOKIE['new'])) {
        $brave[$i] .= 'target="_blank"';
        }
        $brave[$i] .= 'href="'. $item['url']. '" rel="noopener noreferrer" data-sxpr-url>';
        $brave[$i] .= '<p class="OutTitle">'.$item['title']. '</p></a>
        <div class="resLink">'. $burl . '<img src="View/icon/dots_vertical.svg" class="filterImage resOptions">';
        if (!isset($_COOKIE['DisWid'])) {
            $brave[$i] .= '<div class="resOptionsGroup">
            <a href="https://web.archive.org/web/*/'.$item['url'].'" rel="noopener noreferrer"';if (isset($_COOKIE['new'])) {$brave[$i] .= 'target="_blank"';}$brave[$i].='><img class="filterImage sumOpen width32" src="View/icon/archive.svg"></a>
            <a href="proxy/?url='.$item['url'].'" ';if (isset($_COOKIE['new'])) {$brave[$i] .= 'target="_blank"';}$brave[$i].='><img class="filterImage sumOpen opacity10 blueIcon" src="View/icon/mask.svg"></a>
            <img class="filterImage width33 sumOpen" id="sumRes" data-url="'.$item['url'].'" src="View/icon/circle-info.svg">
            </div>';
        }
        $brave[$i] .= '</div>
        <div id="sumResOut" class="sumOut snippet">
        <blockquote id="sumOut"></blockquote>
    </div>';
        
        if(isset($item['description'])){$brave[$i] .= '<p class="snippet" id="snippet">'.$item['description']. '</p>';}
        if (isset($_COOKIE['providers'])) {
        $brave[$i] .= '<p class="resProvider">Brave</p>';
        }
        $brave[$i] .= '</div>';
        ++$i;
    }

    return $brave;
}