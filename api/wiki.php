<?php
if (!isset($_GET['q'])) {
    exit;
}
$dev = false;
include '../Controller/database.php';
header("Content-type: application/json; charset=utf-8");

$txt = filter_input(INPUT_GET, 'q', FILTER_SANITIZE_STRING);
$ch = curl_init("http://127.0.0.1:8000/api/wiki/?q=".urlencode(filter_input(INPUT_GET, 'q', FILTER_SANITIZE_STRING)));
curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);
$response = json_decode(curl_exec($ch), true);
if(curl_errno($ch)){
    echo 'cURL error: ' . curl_error($ch);
    exit();
}
curl_close($ch);

$closestTitle = null;
$closestDistance = PHP_INT_MAX;

foreach($response as $row){
$title = $row['title'];
        $titleExplode = explode(' ', $title);
        $distance = levenshtein($txt, $title);

        if ($distance < $closestDistance) {
            $closestDistance = $distance;
            $closestTitle = $title;
            $wikiobj = str_replace('\\', '', urldecode($row['infobox']));
            
            $wikiTxt = str_replace('\\', '', urldecode($row['paragraph']));
            $ddgObj = str_replace('\\', '', urldecode($row['profiles']));
        }
    }

if(empty($wikiTxt)){
    exit();
}
echo (empty($wikiobj) ? '' : json_encode($wikiobj)),'<--->',$wikiTxt,'<--->',$ddgObj;