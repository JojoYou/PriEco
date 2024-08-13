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
    $likable = array();
    $i = 0;
    $pres = '';

    foreach($PriEcoObj as $row){
          $PriEcoUrl = $row['url'];
            $row['title'] = utf8_encode($row['title']);
            $urls[] = $PriEcoUrl . '<-->'.$row['title'];
            $row['description'] = utf8_encode($row['description']);
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
            if (strpos($PriEcoUrl, 'https://') !== false) {
                $doma = get_string_betweens($PriEcoUrl, 'https://', '/');
            }
            $pres = '<div class="output" id="output">';
            if ($row['img'] != '' && !isset($_COOKIE['datasave'])) {
                $outImg = true;
                $pres .= '<img loading="lazy" alt="‎" src="/Controller/functions/proxy.php?q=' . $row['img'] . '" class="OutSideImg">';
            }
            if (strpos($PriEcoUrl, 'https://') !== false && !isset($_COOKIE['datasave'])) {
                $pres .= '<img loading="lazy" alt="‎" class="Outfavicon" src="/Controller/functions/proxy.php?q=https://judicial-peach-octopus.b-cdn.net/' . $doma . '">';
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
            ##Sort by likeable##

            //Get quality points
            $likable[$i] = $row['likeable'];
            ##
            #Keywords
            ##

            #Keywords in title
            $titleKeywords = explode(' ', strtolower($row['title']));
            $j = 0;
            foreach ($titleKeywords as &$tit) {
                if (in_array($tit, $purlKeywords)) {
                    $likable[$i] += (isset($_COOKIE['rankTitle']) ? $_COOKIE['rankTitle'] : 200);
                    if ($j == 0) {
                        $likable[$i] += (isset($_COOKIE['rankTitle']) ? $_COOKIE['rankTitle'] : 200);
                    }
                }
                if ($tit != null) {
                    ++$j;
                }
            }
            unset($titleKeywords);

            #Keywords in H1
            $h1Keywords = explode(' ', strtolower($row['H1']));
            foreach ($h1Keywords as &$tit) {
                if (in_array($tit, $purlKeywords)) {
                    $likable[$i] += (isset($_COOKIE['rankSecTitle']) ? $_COOKIE['rankSecTitle'] : 200);
                }
            }
            unset($h1Keywords);
            #Keywords in description
            $desKeywords = explode(' ', strtolower($row['description']));
            foreach ($desKeywords as &$tit) {
                if (in_array($tit, $purlKeywords)) {
                    $likable[$i] += (isset($_COOKIE['rankDesc']) ? $_COOKIE['rankDesc'] : 50);
                }
            }
            unset($desKeywords);
            #Keywords in url
            foreach ($purlKeywords as &$tit) {
                if (strpos($row['url'], $tit) !== false) {
                    $likable[$i] += (isset($_COOKIE['rankURL']) ? $_COOKIE['rankURL'] : 100);
                }
            }
            //url == query
            $domain = parse_url($PriEcoUrl, PHP_URL_HOST);
            if (substr_count($domain, '.') > 1) {
                $tmp = explode('.', $domain);
                $tmpNum = count($tmp);
                $domain = $tmp[$tmpNum - 2] . '.' . $tmp[--$tmpNum];
            }
            $domainName = explode('.', $domain)[0];

            if (strpos($PriEcoUrl, 'www.') !== false) {
                $domain = 'www.' . $domain;
            }
            $schemeUrl = parse_url($PriEcoUrl)['scheme'];
            $domain = $schemeUrl . '://' . $domain;
            if ($PriEcoUrl[strlen($PriEcoUrl) - 1] == '/') {
                $domain .= '/';
            }
            if ($domain == $PriEcoUrl && in_array($domainName, $purlKeywords)) {
                $likable[$i] += (isset($_COOKIE['rankURL']) ? ($_COOKIE['rankURL'] * 10) : 1000);
            }

            //home page
            if (in_array($domainName, $purlKeywords)) {
            }
            //Same url country/language
            if (strpos($PriEcoUrl, '.' . $loc) !== false && $loc != null) {
                $likable[$i] += (isset($_COOKIE['rankDomain']) ? $_COOKIE['rankDomain'] : 50);
            }
            if (strpos($PriEcoUrl, '.' . $lang) !== false && $lang != null) {
                $likable[$i] += (isset($_COOKIE['rankDomain']) ? $_COOKIE['rankDomain'] : 50);
            }

            //Same website language
            if ($lang == $row['lang']) {
                $likable[$i] += (isset($_COOKIE['rankLang']) ? $_COOKIE['rankLang'] : 100);
            }
            //Same website location
            if ($loc == $row['server']) {
                $likable[$i] += (isset($_COOKIE['rankLoc']) ? $_COOKIE['rankLoc'] : 100);
            }
            #Keywords in masKey
            foreach ($purlKeywords as &$tit) {
                if ($tit == $row['masKey']) {
                    $likable[$i] += (isset($_COOKIE['rankMas']) ? $_COOKIE['rankMas'] : 250);
                    break;
                }
            }
            $likable[$i] = $likable[$i] + $row['pagerank'] * 1000000;
            ##END Sort by likeable##
            $pres .= '</div>';
            $PriEcoData[$i] = $pres;
            ++$i;
            $pres = null;
        }
        array_multisort($likable, SORT_DESC, $PriEcoData);

    unset($allRes);
    return [$PriEcoData, $urls];
}
