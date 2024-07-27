<?php
if (!isset($_GET['q'])) {
    echo '["No Query!"]';
    return;
}
if (!isset($_GET['api'])) {
    echo '["No API key!"]';
    return;
}

header("Content-type: application/json; charset=utf-8");

$ch = curl_init("http://127.0.0.1:8000/api/indexing/?q=".urlencode(filter_input(INPUT_GET, 'q', FILTER_SANITIZE_STRING)));
curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);
$response = str_replace('&quot;','%22' ,curl_exec($ch));
if(curl_errno($ch)){
    echo 'cURL error: ' . curl_error($ch);
    exit();
}
curl_close($ch);

echo $response;