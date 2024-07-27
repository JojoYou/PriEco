<?php
if (!isset($_GET['q'])) {
    exit;
}

header("Content-type: application/json; charset=utf-8");

$ch = curl_init("http://127.0.0.1:8000/api/youtube/?q=".urlencode(filter_input(INPUT_GET, 'q', FILTER_SANITIZE_STRING)));
curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);
$response = str_replace('%22','\"', curl_exec($ch));
if(curl_errno($ch)){
    echo 'cURL error: ' . curl_error($ch);
    exit();
}
curl_close($ch);

echo $response;