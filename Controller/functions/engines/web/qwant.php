<?php
function qwant($QWantObj, $loaded, $Bpurl, $page){
        if(!isset($QWantObj)){
            $QWant = curl_init();
        curl_setopt($QWant, CURLOPT_URL, 'https://api.qwant.com/v3/search/web/?count=10&offset='.$page.'0&uiv=1&locale=en_us&q=' . $Bpurl);
        curl_setopt($QWant, CURLOPT_USERAGENT, 'Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13');
        curl_setopt($QWant, CURLOPT_CONNECTTIMEOUT, 5);
        curl_setopt($QWant, CURLOPT_RETURNTRANSFER, true);

        $QWantObj = json_decode(curl_exec($QWant), true);
        curl_close($QWant);
        }
        $qwant[]=null;
        $i = 0;
foreach ($QWantObj['data']['result']['items']['mainline'] as &$items) {
    if ($items['type'] === 'web') {
        foreach ($items['items'] as &$item) {
            $qwant[$i] .= '<div class="';

            switch ($i){
                case 0:
                    if(!$loaded[0] && $loaded[1]) {$qwant[$i] .= ' mBorderBoth2 mBorderTop ';}
                    elseif(!$loaded[0]){$qwant[$i] .= ' mBorderTop ';}
                    elseif($loaded[1]){$qwant[$i] .= ' mBorderBottom2 ';}
                    break;
                case 1:
                    if($loaded[1]) {$qwant[$i] .= ' mBorderTop2 ';}
                    if($loaded[2]) {$qwant[$i] .= ' mBorderBottom ';}
                    break;
                case 2:
                    if($loaded[2]) {$qwant[$i] .= ' mBorderTop ';}
                   break;
                case 3:
                    if($loaded[3]) {$qwant[$i] .= ' mBorderBottom ';}
                    break;
                case 4:
                    if($loaded[3]) {$qwant[$i] .= ' mBorderTop ';}
                    break;
                case 5:
                    if($loaded[4]) {$qwant[$i] .= ' mBorderBottom ';}
                    break;
                case 6:
                    if($loaded[4]) {$qwant[$i] .= ' mBorderTop ';}
                    break;
                case 7:
                    if($loaded[5]) {$qwant[$i] .= ' mBorderBottom ';}
                    break;
                case 8:
                    if($loaded[5]) {$qwant[$i] .= ' mBorderTop ';}
                    break;
                case 9:
                    $qwant[$i] .= ' mBorderBottom ';
                    break;
            }
$qwant[$i] .= ' output" id="output">';
            $qwant[$i] .= '<a ';
            if (isset($_COOKIE['new'])) {
                $qwant[$i] .= 'target="_blank"';
            }
            $qwant[$i] .= 'href="'. $item['url']. '">';
            if (strpos($item['url'], 'https://') !== false) {
                $qwant[$i] .= '<img loading="lazy" alt="‎" class="Outfavicon" src="/Controller/functions/proxy.php?q=https://judicial-peach-octopus.b-cdn.net/'.get_string_betweens($item['url'], 'https://', '/'). '">';
            }
            $gurl = str_replace('/',' > ',str_replace('https://','',str_replace('http://','',str_replace('www.','', $item['url']))));
            if ( substr_compare($gurl, ' > ', -3) === 0 ) {
            $gurl = substr($gurl, 0, -3);
            }
            $qwant[$i] .= '<p class="OutTitle">'. $item['title']. '</p></a>
                    <p class="resLink">'. $gurl .'</p>
                    <p class="snippet">'. htmlspecialchars($item['desc']). '</p>';
                    if (isset($_COOKIE['providers'])) {
                        $qwant[$i] .= '<p class="resProvider">Bing</p>';
                    }
                    $qwant[$i] .='</div>';
            ++$i;
        }
    }
}
return $qwant;
}