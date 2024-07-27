<?php
$no = ['humans.txt', 'robots.txt'];
$disabled = glob('../*.txt');
foreach($disabled as &$d){
    $d = str_replace('../','',$d);
    if(!in_array($d, $no)){
        $timestamp = file_get_contents('../'.$d);
        $timezone = new DateTimeZone('UTC');
        $date = new DateTime("@$timestamp");
        $date->setTimezone($timezone);
        $convertedTime = $date->format('Y-m-d H:i:s');
        echo $d, ' ' , $convertedTime,'<br>';
    }
}