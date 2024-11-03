<?php
function search_reddit($results)
{
    $rPrint = false;
    $conversations = '';
    $conversations .= '<p class="sectionTitle">💬 Discussions</p>

        <div class="redditCon output" id="output">
            <input type="checkbox" id="redOpener" class="openerCheck none">';
    $i = 0;
    foreach ($results as &$res) {
        if ($i >= 7) {
            break;
        }
        if (!is_array($res)) {
            break;
        }
        $rPrint = true;
        $conversations .= '<div class="';
        if($i >= 3){
            $conversations .= 'opened ';
        }
        $conversations .= 'redElement">';
        if (filter_var($res['img'], FILTER_VALIDATE_URL) && !isset($_COOKIE['datasave'])) {
            $conversations .= '<img class="OutSideImg" src="/Controller/functions/proxy.php?q=' . urlencode($res['img']) . '">';
        }
        if (!isset($_COOKIE['datasave'])) {
            $conversations .= '<img alt="" class="Outfavicon" loading="lazy" src="/View/img/reddit.webp">  ';
        }
        $conversations .= ' <a href="';
        if (!isset($_COOKIE['redURL']) || $_COOKIE['redURL'] == '') {
            $conversations .= 'https://www.reddit.com';
        } else {
            $conversations .= $_COOKIE['redURL'];
        }
        $conversations .= $res['url'] . '"';
        if (isset($_COOKIE['new'])) {
            $conversations .= 'target="_blank"';
        }
        $conversations .= '>
        <p class="OutTitle">' . $res['title'] . '</p></a>
            <section>r/' . $res['sub'] . ' ⋮ ' . $res['nCom'] . ' 💬 ⋮ ' . $res['ups'] . ' 🔼 ⋮ ' . $res['upR'] * 100 . '% 👍 ⋮ Author: <p class="inline"><b>' . $res['author'] . '</b></p></section>
            </div>';
        ++$i;
    }

    if($i > 3){
        $conversations .= '
        <label id="redExp" for="redOpener" class="flex Pointer width100P">
        <span></span>
        <p class="block centerTxt">Expand Reddit<img class="filterImage width16 ml-10 height15 opacity5" src="View/icon/dropdown.svg"></p>
        <span></span></label>

        <label id="redMin" for="redOpener" class="flex Pointer width100P">
        <span></span>
        <p class="block centerTxt">Minimize Reddit<img class="filterImage width16 ml-10 height15 opacity5 rotate180" src="View/icon/dropdown.svg"></p>
        <span></span></label>
        ';
    }

    $conversations .= '</div>';
    if ($rPrint) {
        return $conversations;
    }
}
