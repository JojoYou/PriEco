<?php
function prieco($PriEcoObj, $purl, $loc, $lang)
{
    $urls = [];
    $sitelimit = false;
    $siteDomain = $purl;
    if (strpos($purl, 'site:') !== false) {
        $sitelimit = true;
        $siteDomain = str_replace('site:', '', $purl);
    }
    //Get query keywords
    $purlKeywords = explode(' ', strtolower($siteDomain));

    $allRes[0] = '';
    $jP = 0;
    $PriEcoData = array();
    $i = 0;
    $pres = '';

    foreach($PriEcoObj as $row){
          $PriEcoUrl = urldecode($row['url']);
            $row['title'] = urldecode($row['title']);
            $urls[] = $PriEcoUrl . '<-->'.$row['title'];
            $row['description'] = urldecode($row['description']);
            $outImg = false;
            if (!in_array($PriEcoUrl, $allRes)) {
                $allRes[$jP] = $PriEcoUrl;
                ++$jP;
            } else {
                continue;
            }
            if ($sitelimit && parse_url($PriEcoUrl, PHP_URL_HOST) != $siteDomain) {
                continue;
            }
            if (!isset($_COOKIE['safe']) && $row['safeS'] == '1') {
                continue;
            }
            if ($PriEcoUrl[strlen($PriEcoUrl) - 1] == '/') {
                $PriEcoUrl = substr_replace($PriEcoUrl, '', -1);
            }
            ##Make PriEcoObjs##
            $pres = '<div class="output" id="output">';
            if ($row['image'] != '' && !isset($_COOKIE['datasave'])) {
                $outImg = true;
                $pres .= '<img loading="lazy" alt="‎" src="/Controller/functions/proxy.php?q=' . $row['image'] . '" class="OutSideImg">';
            }
            if (strpos($PriEcoUrl, 'https://') !== false && !isset($_COOKIE['datasave'])) {
                $pres .= '<img loading="lazy" alt="‎" class="Outfavicon" src="https://jojoyou.org/static/prieco_favicons/' . $row['favicon'] . '">';
            }
            if (!isset($_COOKIE['DisWid'])) {
                $pres .= '<div class="resOptionsGroup">
                <a href="https://web.archive.org/web/*/' . $PriEcoUrl . '" rel="noopener noreferrer"';
                if (isset($_COOKIE['new'])) {
                    $pres .= 'target="_blank"';
                }
                $pres .= '><img class="filterImage sumOpen width32" src="View/icon/archive.svg"></a>
                <a href="proxy/?url=' . $PriEcoUrl . '" ';
                if (isset($_COOKIE['new'])) {
                    $pres .= 'target="_blank"';
                }
                $pres .= '><img class="filterImage sumOpen opacity10 blueIcon" src="View/icon/mask.svg"></a>
                <img class="filterImage width33 sumOpen" id="sumRes" data-url="' . $PriEcoUrl . '" src="View/icon/circle-info.svg">
                </div>';
            }

            $pres .= '<a ';
            if (isset($_COOKIE['new'])) {
                $pres .= 'target="_blank"';
            }
            $gurl = str_replace('/', ' > ', str_replace('https://', '', str_replace('http://', '', str_replace('www.', '', $PriEcoUrl))));
            if (substr_compare($gurl, ' > ', -3) === 0) {
                $gurl = substr($gurl, 0, -3);
            }
            $pres .= 'href="' . (isset($_GET['tabs']) ? 'Controller/functions/saveTab.php?tab=' . $_GET['tab'] . '&url=' . urlencode($PriEcoUrl) : $PriEcoUrl) . '" rel="noopener noreferrer" data-sxpr-link>';
            $pres .= '<p class="'. ($outImg ? 'width100P-131' : '') . ' OutTitle">' . $row['title'] . '</p></a>
        <p class="'. ($outImg ? 'width100P-131' : '') . ' resLink">' . $gurl . '</p>
        <div id="sumResOut" class="'. ($outImg ? 'width100P-131' : '') . ' sumOut snippet">
        <blockquote id="sumOut"></blockquote>
    </div>';
            $pres .= '<p class="'. ($outImg ? 'width100P-131' : '') . ' snippet" id="snippet">' . $row['description'] . '</p>';
            if ($row['tab'] != null and $row['tab'] != '') {
                $tmp = explode('<===>', $row['tab']);
                foreach ($tmp as $rt) {
                    if (filter_var($rt, FILTER_VALIDATE_URL)) {
                        $pres .= '<a class="outputTab" href="' . $rt . '" ';
                        if (isset($_COOKIE['new'])) {
                            $pres .= 'target="_blank"';
                        }
                        $pres .= ' rel="noopener noreferrer">' . parse_url($rt, PHP_URL_HOST) . '</a>';
                    }
                }
            }
            if (isset($_COOKIE['providers'])) {
                $pres .= '<p class="resProvider">PriEco</p>';
            }
            ##END Make PriEcoObjs##

            $pres .= '</div>';
            $PriEcoData[$i] = $pres;
            ++$i;
            $pres = null;
        }

    unset($allRes);
    return [$PriEcoData, $urls];
}
