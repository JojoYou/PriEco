<?php

$query = filter_input(INPUT_GET, "q", FILTER_SANITIZE_STRING);
$words = explode(" ", $query);

$multi_curl = curl_multi_init();
$curl_handles = [];

foreach ($words as $word) {
    $ch = curl_init("http://127.0.0.1:8000/api/imgs/?q=" . urlencode($word));
    curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);
    curl_multi_add_handle($multi_curl, $ch);
    $curl_handles[] = $ch;
}

do {
    curl_multi_exec($multi_curl, $running);
} while ($running > 0);

$rows = [];
foreach ($curl_handles as $ch) {
    $response = json_decode(
        str_replace("%22", '\"', curl_multi_getcontent($ch)),
        true
    );
    if (curl_errno($ch)) {
        echo "cURL error: " . curl_error($ch);
        exit();
    }
    $rows = array_merge($rows, $response);
    curl_multi_remove_handle($multi_curl, $ch);
    curl_close($ch);
}
$urls = array_column($rows, "url");
$uniqueUrls = array_unique($urls);
$rows = array_intersect_key($rows, $uniqueUrls);

$PriEcoData = [];
$likable = [];
$i = 0;
foreach ($rows as $row) {
    $likable[$i] = $row["points"];
    $PriEcoUrl = $row["url"];

    ##
    #Keywords
    ##

    #Keywords in title
    $titleKeywords = explode(" ", strtolower($row["title"]));
    $j = 0;
    foreach ($titleKeywords as &$tit) {
        if (in_array($tit, $words)) {
            $likable[$i] += 100;
            if ($j == 0) {
                $likable[$i] += 100;
            }
        }
        if ($tit != null) {
            ++$j;
        }
    }
    unset($titleKeywords);

    #Keywords in description
    $desKeywords = explode(" ", strtolower($row["description"]));
    foreach ($desKeywords as &$tit) {
        if (in_array($tit, $words)) {
            $likable[$i] += 50;
        }
    }
    #Keywords in alt
    $desKeywords = explode(" ", strtolower($row["alt"]));
    foreach ($desKeywords as &$tit) {
        if (in_array($tit, $words)) {
            $likable[$i] += 200;
        }
    }
    unset($desKeywords);
    #Keywords in url
    foreach ($words as &$tit) {
        if (strpos($row["url"], $tit) !== false) {
            $likable[$i] += 100;
        }
    }
    //url == query
    $clear_domain = $domain = parse_url($PriEcoUrl, PHP_URL_HOST);
    if (substr_count($domain, ".") > 1) {
        $tmp = explode(".", $domain);
        $tmpNum = count($tmp);
        $domain = $tmp[$tmpNum - 2] . "." . $tmp[--$tmpNum];
    }
    $domainName = explode(".", $domain)[0];
    if (strpos($PriEcoUrl, "www.") !== false) {
        $domain = "www." . $domain;
    }
    $schemeUrl = parse_url($PriEcoUrl)["scheme"];
    $domain = $schemeUrl . "://" . $domain;
    if ($PriEcoUrl[strlen($PriEcoUrl) - 1] == "/") {
        $domain .= "/";
    }

    if ($domain == $PriEcoUrl && in_array($domainName, $words)) {
        $likable[$i] += 1000;
    }

    //Same url country/language
    if (strpos($PriEcoUrl, "." . $loc) !== false && $loc != null) {
        $likable[$i] += 50;
    }
    if (strpos($PriEcoUrl, "." . $lang) !== false && $lang != null) {
        $likable[$i] += 50;
    }

    //Same website language
    if ($lang == $row["lang"]) {
        $likable[$i] += 100;
    }
    //Same website location
    if ($loc == $row["server"]) {
        $likable[$i] += 100;
    }
    #Keywords in masKey
    foreach ($words as &$tit) {
        if ($tit == $row["masKey"]) {
            $likable[$i] += 250;
            break;
        }
    }

    #Size
    if ($row["size"] < 1000) {
        $likable[$i] -= 500;
    }
    $PriEcoData[$i] =
        '
           <div class="imgoutdiv">
            <div tabindex="0" class="imgoutbtn">
                <img src="Controller/functions/engines/img/db_proxy.php?q=' .
        substr(urlencode($row["thumb"]), 0, -4) .
        '.webp" class="imgout" loading="lazy">

        <a class="imgoutTxt link" href="' .
        $row["web"] .
        '"';
    if (isset($_COOKIE["new"])) {
        $PriEcoData[$i] .= 'target="_blank';
    }
    $PriEcoData[$i] .=
        '>
<img class="Outfavicon curve height20" alt="&lrm;" loading="lazy" src="/Controller/functions/proxy.php?q=https://judicial-peach-octopus.b-cdn.net/' .
        $clear_domain .
        '">
<p class="colorWhite">' .
        $clear_domain .
        '</p>
</a>
            </div>



            <div class="bigimgout">
            <img src ="Controller/functions/engines/img/db_proxy.php?q=' .
        substr(urlencode($row["thumb"]), 0, -4) .
        '.webp"
            data-src="Controller/functions/proxy.php?q=' .
        urlencode($row["url"]) .
        '"';
    if (!isset($_COOKIE["DisHImg"])) {
        $PriEcoData[$i] .= 'class="blur-5"';
    }
    $PriEcoData[$i] .=
        ' loading="lazy">
            <br>
            <h3>' .
        $row["title"] .
        '</h3><br>
            <p>From website: ' .
        $row["web"] .
        '</p><br>
            <div class="bigimgbtn"><a href="' .
        $row["web"] .
        '"><button class="imgtoolsOption">Go to website</button></a><br>
            <a href="' .
        $row["url"] .
        '"> <button class="imgtoolsOption">Go to image</button></a></div>
            <button class="mt-20 imgtoolsOption">Close</button>
            </div>
            </div>';
    ++$i;
    $pres = null;
}

array_multisort($likable, SORT_DESC, $PriEcoData);
echo '<div class="imgContainer"><br>';
$i = 0;
foreach ($PriEcoData as &$row) {
    if ($i > 75) {
        break;
    }
    echo $row;
    ++$i;
}
echo "</div></div>";

if (!isset($_COOKIE["DisHImg"])) {
    echo '<script src="View/js/highImg.js"></script>';
}
