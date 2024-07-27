<?php
header("Content-type: application/json; charset=utf-8");
if (!isset($_GET['q'])) {
    echo '["No Query!"]';
    return;
}
if (!isset($_GET['api'])) {
    echo '["No API key!"]';
    return;
}
$dev = false;
include '../Controller/database.php';

$lang = isset($_GET['lang']) ? $_GET['lang'] :'all';
$loc = isset($_GET['loc']) ? $_GET['lang'] :'all';

$stmt = $pdo->prepare("SELECT * FROM `imgs` WHERE MATCH(alt, title, description) AGAINST(:Bpurl IN BOOLEAN MODE) ORDER BY `points` DESC LIMIT 100;");
//$stmt->execute(['Bpurl' => $_GET['q']]);
if ($stmt->rowCount() > 0) {
    $PriEcoData = array();
    $likable = array();
    $i = 0;
    $purlKeywords = explode(' ', strtolower($_GET['q']));
    while ($row = $stmt->fetch(PDO::FETCH_ASSOC)) {
        $PriEcoData[$i]=$row;
        $likable[$i] = $row['points'];
        $PriEcoUrl = $row['url'];

        ##
        #Keywords
        ##

        #Keywords in title
        $titleKeywords = explode(' ', strtolower($row['title']));
        $j = 0;
        foreach ($titleKeywords as &$tit) {
            if (in_array($tit, $purlKeywords)) {
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
        $desKeywords = explode(' ', strtolower($row['description']));
        foreach ($desKeywords as &$tit) {
            if (in_array($tit, $purlKeywords)) {
                $likable[$i] += 50;
            }
        }
        #Keywords in alt
        $desKeywords = explode(' ', strtolower($row['alt']));
        foreach ($desKeywords as &$tit) {
            if (in_array($tit, $purlKeywords)) {
                $likable[$i] += 200;
            }
        }
        unset($desKeywords);
        #Keywords in url
        foreach ($purlKeywords as &$tit) {
            if (strpos($row['url'], $tit) !== false) {
                $likable[$i] += 100;
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
            $likable[$i] += 1000;
        }

        //Same url country/language
        if (strpos($PriEcoUrl, '.' . $loc) !== false && $loc != null) {
            $likable[$i] += 50;
        }
        if (strpos($PriEcoUrl, '.' . $lang) !== false && $lang != null) {
            $likable[$i] += 50;
        }

        //Same website language
        if ($lang == $row['lang']) {
            $likable[$i] += 100;
        }
        //Same website location
        if ($loc == $row['server']) {
            $likable[$i] += 100;
        }
        
        ++$i;
    }

   array_multisort($likable, SORT_DESC, $PriEcoData);
   echo json_encode($PriEcoData);
}