<?php
if(!isset($_GET['url'])){exit();}

function fetchWebsiteContent($url, $maxRetries = 3, $delayBetweenRetries = 1) {
    $ch = curl_init($url);
    curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);
    curl_setopt($ch, CURLOPT_USERAGENT, 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537.3');
    curl_setopt($ch, CURLOPT_FOLLOWLOCATION, true);
    curl_setopt($ch, CURLOPT_AUTOREFERER, true);
    curl_setopt($ch, CURLOPT_MAXREDIRS, 5);
    curl_setopt($ch, CURLOPT_TIMEOUT, 30);
    $response = curl_exec($ch);

    curl_close($ch);

    return [
        'html' => $response,
        'url' => curl_getinfo($ch)['url']
    ];
}
function fixRelativeLinks($html, $baseUrl) {
  $pattern = '/(href|src)="([^"]+)"/';

  return preg_replace_callback($pattern, function($matches) use ($baseUrl) {
    $attributeName = $matches[1];
    $relativeUrl = $matches[2];

    if (!parse_url($relativeUrl, PHP_URL_SCHEME)) {
      $absoluteUrl = $baseUrl . '/' . ltrim($relativeUrl, '/');
      return "$attributeName=\"$absoluteUrl\"";
    } else {
      return $matches[0];
    }
  }, $html);
}

function modifyHTML($html){
  return preg_replace('/\bhref\s*(?:=|\s+)"(.*?)"/', 'href="/Controller/functions/proxy.php?q=$1"', preg_replace('/\bsrc\s*(?:=|\s+)"(.*?)"/', 'src="/Controller/functions/proxy.php?q=$1"', $html));
}

$result = fetchWebsiteContent($_GET['url']);
$result['html'] = modifyHTML(fixRelativeLinks($result['html'],$result['url']));
if($result['html'] != '' && isset($result['html'])){
  echo $result['html'];
}
else{
  echo 'Failed!';
}