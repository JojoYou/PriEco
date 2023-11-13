<?php

if (!isset($_GET['url'])) {
    exit();
}

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
if (curl_getinfo($ch, CURLINFO_HTTP_CODE) !== 200) {
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


function cutString($inputString, $len) {
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

echo cutString(implode(' ', summarizeText($extractedText, isset($_GET['count']) ? $_GET['count'] : 2)), 300);
?>
