<?php
function search_news($NewsObj)
{
    if (count($NewsObj) <= 1) {
        return "";
    }
    $i = 0;
    $rPrint = false;
    $news =
        '<a href="/?news&q=' .
        urlencode($_GET["q"]) .
        '"><p class="sectionTitle">🗞️ News</p></a>

    <div class="addonOut output" id="output">';
    foreach ($NewsObj as &$item) {
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
        $rPrint = true;
        if ($i == 0) {
            ++$i;
            continue;
        }
        if ($i > 8) {
            break;
        }
        $domain = str_replace("www.", "", parse_url($item["url"])["host"]);

        $news .=
            '
                    <div class="addonImgOut imgoutdiv">
                    <a href="' .
            $item["url"] .
            '"';
        if (isset($_COOKIE["new"])) {
            $news .= 'target="_blank"';
        }
        $news .= '>
                    <button title="News button" class="ytvideobtn">';
        if (!isset($_COOKIE["datasave"]) && $item["img"] != "") {
            $news .=
                '<img src="Controller/functions/img_proxy.php?q=' .
                urlencode($item["img"]) .
                '">';
        }
        $news .= '</button>
            <div class="imgoutlink videossearch">
              <div class="addonScroll">
                <div class="addonLogo">';
        if (!isset($_COOKIE["datasave"])) {
            $news .=
                '<img src="/Controller/functions/proxy.php?q=https://judicial-peach-octopus.b-cdn.net/' .
                get_string_betweens($item["url"], "https://", "/") .
                '">';
        }
        $news .=
            "<p>" .
            $domain .
            '</p></div>
                <p>';

        $currentDate = new DateTime();

        if (!empty($item["date"])) {
            $specifiedDate = DateTime::createFromFormat("Y-m-d", $item["date"]);

            if ($specifiedDate !== false) {
                $news .=
                    $currentDate->diff($specifiedDate)->format("%a") .
                    'days ago';
            }
        }

        $news .=
            '</p>
              </div>
                <p class="ytTitle">' .
            substr(strip_tags($item["title"]), 0, 47) .
            '...</p>
        <p class="addonDesc">' .
            substr(strip_tags($item["description"]), 0, 120) .
            '...</p>
        </div>
        </a>
        </div>
              ';
        ++$i;
    }
    $news .=
        '<div class="addonImgOut imgoutdiv">
                <a href="/?news&q=' .
        urlencode($_GET["q"]) .
        '">
                 <div class="addonArrow videossearch">
                 <img class="filterImage" src="View/icon/arrow-right.svg"></div>
                 </a>
                 </div>';
    $news .= "</div>";
    if ($rPrint) {
        return $news;
    }
}
