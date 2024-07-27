<?php
function youtube($YoutubeObj)
{
    $i = 0;
    $rPrint = false;

    $yt =
        '<a href="/?video&q=' .
        urlencode($_GET["q"]) .
        '"><p class="sectionTitle">📷 Videos</p></a>

    <div class="addonOut output" id="output">';
    foreach ($YoutubeObj as &$item) {
        $item["title"] = str_replace(
            "%27",
            "'",
            str_replace("%22", '"', $item["title"])
        );
        $item["description"] = str_replace(
            "%27",
            "'",
            str_replace("%22", '"', $item["description"])
        );
        if ($i > 6) {
            break;
        }
        $rPrint = true;
        $yt .= '
        <div class="addonImgOut imgoutdiv">
        <a href="';
        if (!isset($_COOKIE["vidURL"]) || $_COOKIE["vidURL"] == "") {
            $yt .= "https://www.youtube.com/watch?v=";
        } else {
            $yt .= $_COOKIE["vidURL"];
        }
        $yt .= $item["url"] . '"';
        if (isset($_COOKIE["new"])) {
            $yt .= 'target="_blank"';
        }
        $yt .= '>
                    <button title="YouTube video button" class="ytvideobtn">';
        if (!isset($_COOKIE["datasave"])) {
            $yt .=
                '<img src="/Controller/functions/proxy.php?q=https://i.ytimg.com/vi/' .
                $item["thumb"] .
                '">';
        }
        $yt .= '</button>
            <div class="imgoutlink videossearch">
            <div class="addonScroll">
              <div class="addonLogo">';
        if (!isset($_COOKIE["datasave"])) {
            $yt .= '<img src="View/icon/profiles/youtube.svg">';
        }
        $yt .= '<p>YouTube</p></div>
                <p>';
        $currentDate = new DateTime();
        $specifiedDate = new DateTime($item["date"]);
        $yt .=
            $currentDate->diff($specifiedDate)->format("%a") .
            ' days ago</p>
              </div>
                <p class="ytTitle">' .
            $item["title"] .
            '</p>
        <p class="addonDesc">' .
            strip_tags($item["description"]) .
            '</p>
        </div>
        </a>
        </div>
              ';
        ++$i;
    }
    $yt .=
        '<div class="addonImgOut imgoutdiv">
                <a href="/?video&q=' .
        urlencode($_GET["q"]) .
        '">
                 <div class="addonArrow videossearch">
                 <img class="filterImage" src="View/icon/arrow-right.svg"></div>
                 </a>
                 </div>';
    $yt .= "</div>";
    if ($rPrint) {
        return $yt;
    }
}
