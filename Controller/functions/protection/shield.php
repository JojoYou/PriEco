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
    function query_encrypt($data, $key) {
        $cipher = "aes-256-cbc";
        $iv = openssl_random_pseudo_bytes(openssl_cipher_iv_length($cipher));
        $encryptedData = openssl_encrypt($data, $cipher, $key, 0, $iv);
        return base64_encode($iv . $encryptedData);
    }

    function query_decrypt($data, $key) {
        $cipher = "aes-256-cbc";
        $data = base64_decode($data);
        $ivLength = openssl_cipher_iv_length($cipher);
        $iv = substr($data, 0, $ivLength);
        $encryptedData = substr($data, $ivLength);
        return openssl_decrypt($encryptedData, $cipher, $key, 0, $iv);
    }

    function countWordOccurrences($filename, $key) {
        $wordCount = [];
        if (file_exists($filename)) {
            $contents = file_get_contents($filename);
            $lines = explode("\n", trim($contents));
            foreach ($lines as $line) {
                $line = query_decrypt($line, $key);
                $words = preg_split('/\s+/', strtolower(trim($line)));
                foreach ($words as $word) {
                    $wordCount[$word] = ($wordCount[$word] ?? 0) + 1;
                }
            }
        }
        return $wordCount;
    }

    function isSimilarQuery($newQuery, $previousQueries, $wordOccurrences, $key) {
        $newQuery = query_decrypt($newQuery, $key);
        $newQueryWords = array_map('strtolower', preg_split('/\s+/', trim($newQuery)));

        $similarCount = 0;

        foreach ($previousQueries as $query) {
            $query = query_decrypt($query, $key);
            $queryWords = array_map('strtolower', preg_split('/\s+/', trim($query)));
            $commonWords = array_intersect($newQueryWords, $queryWords);
            $commonWordCount = 0;

            foreach ($commonWords as $word) {
                if ($wordOccurrences[$word] ?? 0 > 1) {
                    $commonWordCount++;
                }
            }
            if ($commonWordCount > 1) {
                $similarCount++;
            }
           else if($commonWordCount > 0){
              $similarCount += 0.5;
           } else {
                $levDistance = levenshtein($newQuery, $query);
                if ($levDistance <= 3) {
                    $similarCount++;
                }
            }

            if ($similarCount >= 4) {
                return true;
            }
        }
        return false;
    }

    $query_file = "query.txt";
    $key = $_ENV['Query_Encryption'];

    if (!file_exists($query_file) || filesize($query_file) == 0) {
        if (!empty($purl)) {
            $encryptedPurl = query_encrypt($purl, $key);
            file_put_contents($query_file, $encryptedPurl . "\n");
        }
    } elseif (!empty($purl)) {
        $lines = file($query_file, FILE_IGNORE_NEW_LINES | FILE_SKIP_EMPTY_LINES);
        $wordOccurrences = countWordOccurrences($query_file, $key);

        $encryptedPurl = query_encrypt($purl, $key);
        if (isSimilarQuery($encryptedPurl, $lines, $wordOccurrences, $key)) {
            $pass = false;
        } else {
            if (count($lines) >= 100) {
                $lines = array_slice($lines, -99);
            }
            $lines[] = $encryptedPurl;
            file_put_contents($query_file, implode("\n", $lines) . "\n");
        }
    }


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
