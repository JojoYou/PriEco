<?php
$ch = curl_init(
    "http://127.0.0.1:8000/static/img/prieco/" .
        filter_input(INPUT_GET, "q", FILTER_SANITIZE_STRING)
);
curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);
$img = curl_exec($ch);
if (curl_errno($ch)) {
    echo "cURL error: " . curl_error($ch);
    exit();
}
curl_close($ch);

header("Content-Type: image/webp");

echo $img;
