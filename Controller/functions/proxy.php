<?php

function request($url)
{
    $ch = curl_init($url);
    curl_setopt_array($ch, array(
        CURLOPT_RETURNTRANSFER => true,
        CURLOPT_ENCODING => "",
        CURLOPT_USERAGENT => "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/107.0.0.0 Safari/537.36",
        CURLOPT_IPRESOLVE => CURL_IPRESOLVE_V4,
        CURLOPT_CUSTOMREQUEST => "GET",
        CURLOPT_PROTOCOLS => CURLPROTO_HTTPS | CURLPROTO_HTTP,
        CURLOPT_REDIR_PROTOCOLS => CURLPROTO_HTTPS | CURLPROTO_HTTP,
        CURLOPT_MAXREDIRS => 3,
        CURLOPT_TIMEOUT => 8,
        CURLOPT_VERBOSE => false,
        CURLOPT_FOLLOWLOCATION => true
    ));
    return curl_exec($ch);
}

function printimg($url){
  $image_src = request($url);

  $img_info = getimagesize('data://application/octet-stream;base64,' . base64_encode($image_src));
  $mime_type = $img_info['mime'];

  if ($mime_type == 'image/svg+xml') {
    header("Content-Type: $mime_type");
  } else {
    header("Content-Type: " . $img_info['mime']);
  }

  echo $image_src;
}
//Get url
if(isset($_SERVER['HTTPS']) && $_SERVER['HTTPS'] === 'on')
     $url = "https://";
else
     $url = "http://";
$url.= $_SERVER['HTTP_HOST'];
$url.= $_SERVER['REQUEST_URI'];
$url .= "▛";

//Get query from url
function get_string_between($string, $start, $end){
    $string = ' ' . $string;
    $ini = strpos($string, $start);
    if ($ini == 0) return '';
    $ini += strlen($start);
    $len = strpos($string, $end, $ini) - $ini;
    return substr($string, $ini, $len);
}
$purl = get_string_between($url, '?', '▛');
$urlSet = '&'.$purl;
$purl = urldecode($purl);
$purl = '▛'.$purl;
$purl = $purl.'▛';
$url = get_string_between($purl, 'q=', '▛');
printimg($url);