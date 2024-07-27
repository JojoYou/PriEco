<?php
function topInfo($openVerse, $news, $wiki)
{
    if (sizeof($wiki) < 2) {
        return '';
    }
    if (!isset($openVerse['results'][0]['thumbnail'])) {
        return '';
    }
    $doNotUse = ['title', 'Website', 'images'];
    do {
        $randomKeys = array_rand($wiki, 2);
    } while (in_array($randomKeys[0], $doNotUse) || in_array($randomKeys[1], $doNotUse));
    
    $topInfo = '
    <div id="output" class="topInfoBox scroll flex-start padding5505 max-width100VW float-unset redditCon output">
    <a class="paddingUnset" href="?image&openverse&q=' . urlencode($_GET['q']) . '">
    <div class="topInfoImgBox">
        <div class="borderRadiusLeft">
    <img src="/Controller/functions/proxy.php?q=' . $openVerse['results'][8]['thumbnail'] . '">
    </div>
    <div class="borderRadiusRight">
        <div class="height50P">
    <img src="/Controller/functions/proxy.php?q=' . $openVerse['results'][9]['thumbnail'] . '">
    </div>
    <div>
    <img src="/Controller/functions/proxy.php?q=' . $openVerse['results'][10]['thumbnail'] . '">
    </div>
    </div>
    </div>
    </a>

    <a class="mr-20 ml-20 height100P" href="' . $news['url'] . '"';
    if (isset($_COOKIE['new'])) {
        $topInfo .= 'target="_blank"';
    }
    $topInfo .= '>
                    <button title="News button" class="height135 width220 ytvideobtn">';
    if (!isset($_COOKIE['datasave']) && $news['img'] != '') {
        $topInfo .= '<img src="Controller/functions/proxy.php?q=' . urlencode($news['img']) . '">';
    }
    $topInfo .= '</button>
            <div class="width220 height100 centerTxt imgoutlink videossearch">
              <div class="addonScroll">
                <div class="addonLogo">';
    if (!isset($_COOKIE['datasave'])) {
        $topInfo .= '<img src="/Controller/functions/proxy.php?q=https://judicial-peach-octopus.b-cdn.net/' . get_string_betweens($news['url'], 'https://', '/') . '">';
    }
    $topInfo .= '<p>' . str_replace('www.', '', parse_url($news['url'])['host']) . '</p></div>
                <p class="txt12">';
    $currentDate = new DateTime();
    $specifiedDate = new DateTime($news['date']);
    $topInfo .= $currentDate->diff($specifiedDate)->format('%a') . ' days ago</p>
              </div>
                <p class="ytTitle">' . substr($news['title'], 0, 47) . '...</p>
        <p class="addonDesc">' . substr(strip_tags($news['description']), 0, 120) . '...</p>
        </div>
        </a>

    <div class="topInfoImgBox">
        <div class="borderRadiusLeft">

<a href="?image&openverse&q=' . urlencode($_GET['q']) . '">
<img class="top0 height50P" src="/Controller/functions/proxy.php?q=' . $openVerse['results'][11]['thumbnail'] . '">
<p class="padding0 curve absolute ytTitle"></p>
</a>


   
    <div class="height50P padding10">
    <p><b>' . $randomKeys[0] . '</b></p>
    <p class="opacity7 txt12">' . $wiki[$randomKeys[0]] . '</p>
    </div>
    
    </div>


    <div class="borderRadiusRight">
    <div class="height50P padding10">
    <p><b>' . $randomKeys[1] . '</b></p>
    <p class="opacity7 txt12">' . $wiki[$randomKeys[1]] . '</p>
</div>

        <a href="?image&openverse&q=' . urlencode($_GET['q']) . '">
        <img class="height50P" src="/Controller/functions/proxy.php?q=' . $openVerse['results'][12]['thumbnail'] . '">
        </a>
    </div>
    </div>
    </div>

';
    if (!isset($_COOKIE['DisWid'])) {
        echo '<script src="View/js/dynColor.js"></script>';
    }
    return $topInfo;
}