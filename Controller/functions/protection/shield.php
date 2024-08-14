<?php
function shield($pdo, $purl, $cssver)
{
    #hCaptcha
    if (isset($_SESSION["Pass"])) {
        return;
    }

    if (isset($_POST["submit"])) {
        if (!empty($_POST["h-captcha-response"])) {
            $verifyURL = "https://hcaptcha.com/siteverify";
            $token = $_POST["h-captcha-response"];
            $data = [
                "secret" => $_ENV["hCaptcha_Secret"],
                "response" => $token,
                "remoteip" => $_SERVER["REMOTE_ADDR"],
            ];
            $curlConfig = [
                CURLOPT_URL => $verifyURL,
                CURLOPT_POST => true,
                CURLOPT_RETURNTRANSFER => true,
                CURLOPT_POSTFIELDS => $data,
            ];
            $ch = curl_init();
            curl_setopt_array($ch, $curlConfig);
            $response = curl_exec($ch);
            curl_close($ch);
            $responseData = json_decode($response);
            if ($responseData->success) {
                $_SESSION["Pass"] = true;
                header("refresh:0");
                exit();
            }
        }
    }

    $pass = true;
    #IP
    if (!isset($_SESSION["IPPass"])) {
        $ip = explode(".", $_SERVER["REMOTE_ADDR"]);
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
                $pass = false;
            } else {
                $_SESSION["IPPass"] = true;
            }
        } else {
            $_SESSION["IPPass"] = true;
        }
    }

    #Suspicious words
    $words = ['slot', 'podatelna', 'kosmetik'];
    if (!empty(array_intersect(explode(" ", $purl), $words))) {
        $pass = false;
    }

    #Similar words repeate
    function calculateSimilarity($string1, $string2) {
        $words1 = preg_split('/\s+/', trim($string1));
        $words2 = preg_split('/\s+/', trim($string2));
        $commonWords = array_intersect($words1, $words2);
        return count($commonWords);
    }

    /*$filePath = "query.txt";
    $similarityThreshold = 3;

    if (file_exists($filePath)) {
        $lines = file($filePath, FILE_IGNORE_NEW_LINES | FILE_SKIP_EMPTY_LINES);

        $similarStrings = array_filter($lines, function($line) use ($purl, $similarityThreshold) {
            return levenshtein(trim($line), $purl) <= $similarityThreshold;
        });
        $count = count($similarStrings);

        if ($count >= 4) {
           $pass = false;
        }
        else{
          $similarStrings = array_filter($lines, function($line) use ($purl, $similarityThreshold) {
               return calculateSimilarity(trim($line), $purl) >= $similarityThreshold;
          });
          $count = count($similarStrings);
          if ($count >= 4) {$pass=false;}
        }
        if($pass){
          $lines[] = $purl;

          if (count($lines) > 30) {
            $lines = array_slice($lines, -30);
          }
          file_put_contents($filePath, implode("\n", $lines));
        }
    }
    else{
      file_put_contents($filePath, $purl);
    }*/


    #CAPTCHA
    if (!$pass) {
        header("HTTP/1.0 403 Forbidden");
        echo '<!DOCTYPE html><html lang="en"><head><title>Blocked | PriEco</title><meta name="description" content="PriEco, the Private, Secure and Ecofriendly search engine."><meta charset="UTF-8"><meta http-equiv="X-UA-Compatible" content="IE=edge"><meta name="viewport" content="width=device-width, initial-scale=1.0"><link rel="icon" href="./favicon.ico?1"><link rel="search"type="application/opensearchdescription+xml"title="PriEco"href="osd.xml"></head><body><div class="width100V height100V flex flexDColumn alignC justConC centerTxt"><h1>Shield 🛡️</h1><p><b>Your activity looks suspicious to us.</b><br>It is alright, if you are a real human, fill up this CAPTCHA and click unlock.</p><form action="" method="post"><div class="h-captcha" data-sitekey="',
            $_ENV["hCaptcha_Site"],
            '"></div>';
        if (isset($_POST["submit"])) {
            if (empty($_POST["h-captcha-response"])) {
                echo '<p class="colorRed">Fill up hCaptcha!</p>';
            }
        }
        echo '<input type="submit" name="submit" value="🔓 Unlock" class="whiteAblackBg borderNone borderRadius Pointer padding10"></form></div><script src="https://hcaptcha.com/1/api.js" async defer></script>';
        $beforePathStyle = "/";
        include "Model/style.php";
        include "Model/footer.php";
        echo "</body></html>";

        exit();
    }
    return;
}
shield($pdo, $purl, $cssver);
