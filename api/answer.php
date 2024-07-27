<?php
    $data = json_decode(file_get_contents('php://input'), true);
    if (!isset($data['dataToSend']['textData']) || !isset($data['dataToSend']['question'])) { header('HTTP/1.1 401 Unauthorized');return;}
    
    $textData = $data['dataToSend']['textData'];
    $question = $data['dataToSend']['question'];

 $data = array(
     'context' => $textData, 
     'question' => $question
 );

 $payload = json_encode($data);

 $ch = curl_init('https://porciascreen.de/apps/leaq/');
 curl_setopt($ch, CURLOPT_CUSTOMREQUEST, "POST");
 curl_setopt($ch, CURLOPT_POSTFIELDS, $payload);
 curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);
 curl_setopt(
     $ch,
     CURLOPT_HTTPHEADER,
     array(
         'Content-Type: application/json',
         'Content-Length: ' . strlen($payload)
     )
 );

 echo curl_exec($ch);
