<?php
function qwant($QWantObj, $loaded, $Bpurl, $page){
        $qwant[]=null;
        $i = 0;
foreach ($QWantObj['data']['result']['items']['mainline'] as &$items) {
    if ($items['type'] === 'web') {
        foreach ($items['items'] as &$item) {
            $qwant[$i] .= '<div class="output" id="output">';
            $qwant[$i] .= '<a ';
            if (isset($_COOKIE['new'])) {
                $qwant[$i] .= 'target="_blank"';
            }
            $qwant[$i] .= 'href="'. $item['url']. '" rel="noopener noreferrer" data-sxpr-link>';
            if (strpos($item['url'], 'https://') !== false && !isset($_COOKIE['datasave'])) {
                $qwant[$i] .= '<img loading="lazy" alt="‎" class="Outfavicon" src="/Controller/functions/proxy.php?q=https://judicial-peach-octopus.b-cdn.net/'.get_string_betweens($item['url'], 'https://', '/'). '">';
            }
            $gurl = str_replace('/',' > ',str_replace('https://','',str_replace('http://','',str_replace('www.','', $item['url']))));
            if ( substr_compare($gurl, ' > ', -3) === 0 ) {
            $gurl = substr($gurl, 0, -3);
            }

            $description = strip_tags($item['desc']);
            $description = strlen($description)>150 ? substr($description,0,150).'...' : $description;

            $qwant[$i] .= '<p class="OutTitle">'. $item['title']. '</p></a>
                    <div class="resLink">'. $gurl . '<img src="View/icon/dots_vertical.svg" class="filterImage resOptions">';
                    if (!isset($_COOKIE['DisWid'])) {
                        $qwant[$i] .= '<div class="resOptionsGroup">
                        <a href="https://web.archive.org/web/*/'.$item['url'].'" rel="noopener noreferrer"';if (isset($_COOKIE['new'])) {$qwant[$i] .= 'target="_blank"';}$qwant[$i].='><img class="filterImage sumOpen width32" src="View/icon/archive.svg"></a>
                        <a href="proxy/?url='.$item['url'].'" ';if (isset($_COOKIE['new'])) {$qwant[$i] .= 'target="_blank"';}$qwant[$i].='><img class="filterImage sumOpen opacity10 blueIcon" src="View/icon/mask.svg"></a>
                        <img class="filterImage width33 sumOpen" id="sumRes" data-url="'.$item['url'].'" src="View/icon/circle-info.svg">
                        </div>';
                    }
                    $qwant[$i] .= '
                    </div>

                    <div id="sumResOut" class="sumOut snippet">
                        <blockquote id="sumOut"></blockquote>
                    </div>
                    
                    <p class="snippet">'.$description.'</p>';
                    if (isset($_COOKIE['providers'])) {
                        $qwant[$i] .= '<p class="resProvider">Bing</p>';
                    }
                    $qwant[$i] .= '</div>';
            ++$i;
        }
    }
}
return $qwant;
}