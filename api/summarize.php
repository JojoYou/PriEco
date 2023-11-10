<?php

if(!isset($_GET['url'])){exit();}

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
if(curl_getinfo($ch, CURLINFO_HTTP_CODE) !== 200){
    exit();
}

$dom = str_get_html($html);
      $paragraphs = $dom->find('p');
      $paragraphText = [];
    foreach ($paragraphs as $paragraph) {
      $paragraphText[] = $paragraph->plaintext;
    }
  
$txt = '';
foreach ($paragraphText as $text) {
  $txt .= $text . PHP_EOL;
}


function cutString($inputString, $len) {
    if (strlen($inputString) <= $len) {return $inputString;}

    $trimmed = substr($inputString, 0, $len);
    $lastSpacePos = strrpos($trimmed, ' ');

    if ($lastSpacePos === false) {return $trimmed.'...';}

    return substr($trimmed, 0, $lastSpacePos).'...';
}

echo cutString(implode(' ', summarizeText($txt, isset($_GET['count']) ? $_GET['count'] : 2)), 300);