<?php
function remote($obj, $engine)
{
    if(gettype($obj) != 'array'){$obj = json_decode($obj, true);}

    switch ($engine) {
        case 'Google':
            $namespace = ['', 'link', 'title', 'snippet'];
            break;
        case 'Google2':
            $namespace = ['', 'url', 'title', 'description'];
            break;
        case 'Brave':
            $namespace = ['items', 'url', 'title', 'description'];
            $obj['items'] = $obj['web']['results'];
            break;
        case 'QWant':
            $namespace = ['items', 'url', 'title', 'desc'];
            $obj['items'] = $obj['data']['result']['items']['mainline'];
            break;
        case 'Mojeek':
            $namespace = ['items', 'url', 'title', 'desc'];
            $obj['items'] = $obj['response']['results'];
            break;

    }


    $urls = $remote = [];

    if($engine == 'Google'){
        foreach ($obj['items'] as &$item) {
            $remote[] = prepareResult($item, $namespace, $engine);
            $urls[] = $item[$namespace[1]]. '<--->'. strip_tags($item[$namespace[2]]);
        }
    }
    elseif($engine == 'Google2'){
        foreach ($obj as &$item) {
            if(!isset($item['url'])){continue;}
            $remote[] = prepareResult($item, $namespace, $engine);
            $urls[] = $item[$namespace[1]]. '<--->'. strip_tags($item[$namespace[2]]);
        }
    }
    elseif ($engine == 'QWant') {
        foreach ($obj[$namespace[0]] as &$item) {
        if ($item['type'] === 'web') {
            foreach ($item['items'] as &$items) {
                $remote[] = prepareResult($items, $namespace, $engine);
                $urls[] = $items[$namespace[1]]. '<--->'. strip_tags($items[$namespace[2]]);
            }
        }
    }
    }
    else{
        foreach ($obj[$namespace[0]] as &$item) {
            $remote[] = prepareResult($item, $namespace, $engine);
            $urls[] = $item[$namespace[1]]. '<--->'. strip_tags($item[$namespace[2]]);
        }
    }


    return [
      $remote,
      $urls
    ];
}

function prepareResult($item, $namespace, $engine)
{
    if(!isset($item[$namespace[2]])){return;}
    $format = '<div class="output" id="output">';

    $gurl = str_replace('/', ' > ', str_replace('https://', '', str_replace('http://', '', str_replace('www.', '', explode('?', $item[$namespace[1]])[0]))));
    if (substr_compare($gurl, ' > ', -3) === 0) {
        $gurl = substr($gurl, 0, -3);
    }

    if (strpos($item[$namespace[1]], 'https://') !== false && !isset($_COOKIE['datasave'])) {
        $format .= '<img class="Outfavicon" alt="‎" loading="lazy" src="/Controller/functions/proxy.php?q=https://judicial-peach-octopus.b-cdn.net/' . get_string_betweens($item[$namespace[1]], 'https://', '/') . '">';
    }
    if (!isset($_COOKIE['DisWid'])) {
        $format .= '
        <div class="resOptionsGroup">
        <a class="removeULink" href="https://web.archive.org/web/*/' . $item[$namespace[1]] . '" rel="noopener noreferrer"';
        if (isset($_COOKIE['new'])) {
            $format .= 'target="_blank"';
        }
        $format .= '><img class="filterImage sumOpen width32" src="View/icon/archive.svg"></a>
        <a class="removeULink" href="proxy/?url=' . $item[$namespace[1]] . '" ';
        if (isset($_COOKIE['new'])) {
            $format .= 'target="_blank"';
        }
        $format .= '><img class="filterImage sumOpen opacity10 blueIcon" src="View/icon/mask.svg"></a>
        <img class="filterImage width33 sumOpen" id="sumRes" data-url="' . $item[$namespace[1]] . '" src="View/icon/circle-info.svg">
        </div>
    ';
    }
    $format .= '<a ';
    if (isset($_COOKIE['new'])) {
        $format .= 'target="_blank"';
    }

    $format .= 'href="' . (isset($_GET['tabs']) ? 'Controller/functions/saveTab.php?tab=' . $_GET['tab'] . '&url=' . urlencode($item[$namespace[1]]) : $item[$namespace[1]]) . '" rel="noopener noreferrer" data-sxpr-link>';
    $format .= '<p class="OutTitle">' . strip_tags($item[$namespace[2]]) . '</p></a>
    <p class="resLink">' . strip_tags($gurl) . '</p>
     <div id="sumResOut" class="sumOut snippet">
        <blockquote id="sumOut"></blockquote>
    </div>
    ';


    if (isset($item[$namespace[3]])) {
        $format .= '
        <p class="snippet" id="snippet">' . strip_tags($item[$namespace[3]]) . '</p>';
    }
    if (isset($_COOKIE['providers'])) {
        $format .= '<p class="resProvider">' . $engine . '</p>';
    }
    $format .= '</div>';
    return $format;
}
