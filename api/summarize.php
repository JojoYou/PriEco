<?php

if (!isset ($_GET['url'])) {
    exit();
}
$char = isset($_GET['char']) ? $_GET['char'] : 300;
$sumPath = '../';
include '../Controller/functions/addons/summarizer/sum.php';
include '../Controller/simple_html_dom.php';

$ch = curl_init($_GET['url']);
curl_setopt($ch, CURLOPT_FOLLOWLOCATION, true);
curl_setopt($ch, CURLOPT_USERAGENT, 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/106.0.0.0 Safari/537.36');
curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);
curl_setopt($ch, CURLOPT_CONNECTTIMEOUT, 5);
curl_setopt($ch, CURLOPT_TIMEOUT, 10);

$html = curl_exec($ch);
$data = curl_getinfo($ch);
if ($data['http_code'] !== 200) {
    exit();
}

$html = new simple_html_dom($html);

$extractedText = '';

foreach ($html->find('p') as $paragraph) {
    $text = trim($paragraph->plaintext);
    if (strlen($text) >= 50) {
        $extractedText .= $paragraph->plaintext . ' ';
    }
}


function cutString($inputString, $len)
{
    if (strlen($inputString) <= $len) {
        return $inputString;
    }

    $trimmed = substr($inputString, 0, $len);
    $lastSpacePos = strrpos($trimmed, ' ');

    if ($lastSpacePos === false) {
        return $trimmed . '...';
    }

    return substr($trimmed, 0, $lastSpacePos) . '...';
}



header('Content-Type: application/json');


$pattern = '/<img[^>]+src=["\']([^"\']+)["\'][^>]*>/i';

// Perform the regular expression match
preg_match_all($pattern, $html, $matches);

// $matches[1] now contains an array of image URLs
$imageUrls = $matches[1];
foreach ($imageUrls as &$iu) {
    if (strpos($iu, 'http://') === false && strpos($iu, 'https://') === false) {
        $iu = $_GET['url'] . $iu;
    }
}

if (!isset ($_GET['oldSummary'])) {
    $ch = curl_init('https://porciascreen.de/apps/chloe/');
    curl_setopt($ch, CURLOPT_POST, 1);
    curl_setopt($ch, CURLOPT_POSTFIELDS, http_build_query(array(
        'text' => urlencode($extractedText),
        'top_n' => 5,
        'char_limit' => $char,
    )));
    curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);
    $sum = json_decode(curl_exec($ch), true)['summarized_text'];
    curl_close($ch);
} else {
    $sum = cutString(implode(' ', summarizeText($extractedText, isset ($_GET['count']) ? $_GET['count'] : 2)), 300);
}

$output = array(
    'summary' => html_entity_decode($sum),
    'ip' => $data['primary_ip'],
    'ssl' => $data['primary_port'],
    'speed' => $data['total_time'],
    'images' => $imageUrls
);
echo json_encode($output);
?>