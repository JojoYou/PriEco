<?php
$data = json_decode(file_get_contents('php://input'), true);
if (!isset($data['dataToSend']['url'])) { header('HTTP/1.1 401 Unauthorized');return;}

$data = array(
    'article_url' => $data['dataToSend']['url'],
    'max_qa_pairs' => '4'
);
$data = json_encode($data);
$ch = curl_init('https://porciascreen.de/apps/qagen/');

curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);
curl_setopt($ch, CURLOPT_POST, 1);
curl_setopt($ch, CURLOPT_POSTFIELDS, $data);
curl_setopt($ch, CURLOPT_HTTPHEADER, array(
    'Content-Type: application/json',
    'Content-Length: ' . strlen($data))
);
echo curl_exec($ch);
curl_close($ch);