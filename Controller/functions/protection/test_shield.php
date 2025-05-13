<?php
function shield($pdo, $purl, $cssver)
{
    #IP
    if (!isset($_SESSION["IPPass"])) {
        $ip = explode(".", (isset($_SERVER['HTTP_CF_CONNECTING_IP']) ? $_SERVER['HTTP_CF_CONNECTING_IP'] : $_SERVER['REMOTE_ADDR']));
        $curl = curl_init("http://127.0.0.1:8000/api/api/ipShield?q=" . $ip[0]);
        curl_setopt($curl, CURLOPT_RETURNTRANSFER, true);
        $row = json_decode(curl_exec($curl), true);
        curl_close($curl);

        if (gettype($row) == "array" && count($row) > 0) {
            if (
                ($row["ip2"] == $ip[1] || $row["ip2"] == "0") &&
                ($row["ip3"] == $ip[2] || $row["ip3"] == "0") &&
                ($row["ip4"] == $ip[3] || $row["ip4"] == "0")
            ) {
              echo 'IP blocked';
              exit(0);
            } else {
                $_SESSION["IPPass"] = true;
            }
        } else {
            $_SESSION["IPPass"] = true;
        }
    }


    #HTTP version
    /*$httpVersion = isset($_SERVER['SERVER_PROTOCOL']) ? $_SERVER['SERVER_PROTOCOL'] : '';
    $supportedHttpVersions = ['HTTP/2.0'];

    if (!in_array($httpVersion, $supportedHttpVersions)) {
      $pass = false;
    }*/

    #Browser version
    $userAgent = $_SERVER['HTTP_USER_AGENT'];
    $browser = '';
    $version = 0;
    if (preg_match('/Firefox/i', $userAgent)) {
        $browser = 'Firefox';
        preg_match('/Firefox\/([0-9\.]+)/', $userAgent, $versionMatch);
        $version = isset($versionMatch[1]) ? $versionMatch[1] : '';
    } elseif (preg_match('/Chrome/i', $userAgent)) {
        $browser = 'Chrome';
        preg_match('/Chrome\/([0-9\.]+)/', $userAgent, $versionMatch);
        $version = isset($versionMatch[1]) ? $versionMatch[1] : '';
    } elseif (preg_match('/Safari/i', $userAgent)) {
        $browser = 'Safari';
        preg_match('/Version\/([0-9\.]+)/', $userAgent, $versionMatch);
        $version = isset($versionMatch[1]) ? $versionMatch[1] : '';
    }
    if (($browser == 'Safari' && $version < 16) || ($browser != '' && $version < 125 && !($browser == 'Firefox' && $version == 115))) {
      echo 'Blocked old browser version: ' . $browser . ' ' . $version;
      exit(0);
    }


    #No encoding
    if (!isset($_SERVER['HTTP_ACCEPT_ENCODING']) || empty($_SERVER['HTTP_ACCEPT_ENCODING'])) {
      echo 'Blocked, browser does not support encoding';
      exit(0);
    }
}
shield($pdo, $purl, $cssver);
