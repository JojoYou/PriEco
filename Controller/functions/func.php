<?php
function cutString($inputString, $len) {
    if (strlen($inputString) <= $len) {return $inputString;}

    $trimmed = substr($inputString, 0, $len);
    $lastSpacePos = strrpos($trimmed, ' ');

    if ($lastSpacePos === false) {return $trimmed.'...';}

    return substr($trimmed, 0, $lastSpacePos).'...';
  }
function openverseToken(){
if($_ENV['OPENVERSE']==''|| explode(';',$_ENV['OPENVERSE'])[1] <= time()){
    $curl = curl_init();
    curl_setopt_array($curl, array(
        CURLOPT_URL => "https://api.openverse.org/v1/auth_tokens/token/",
        CURLOPT_RETURNTRANSFER => true,
        CURLOPT_ENCODING => "",
        CURLOPT_MAXREDIRS => 10,
        CURLOPT_TIMEOUT => 0,
        CURLOPT_FOLLOWLOCATION => true,
        CURLOPT_HTTP_VERSION => CURL_HTTP_VERSION_1_1,
        CURLOPT_CUSTOMREQUEST => "POST",
        CURLOPT_POSTFIELDS => "grant_type=client_credentials&client_id=".$_ENV['OPENVERSE_CLIENT_ID']."&client_secret=".$_ENV['OPENVERSE_CLIENT_SECRET'],
        CURLOPT_HTTPHEADER => array(
            "Content-Type: application/x-www-form-urlencoded"
        ),
    ));
    $response = json_decode(curl_exec($curl),true);
    curl_close($curl);

    $envFile = explode("\n",file_get_contents('Controller/.env'));
    $envFile[count($envFile)-1]='OPENVERSE='.$response['access_token'].';'.time() + $response['expires_in'];

    file_put_contents('Controller/.env', implode("\n",$envFile));

    $_ENV['OPENVERSE'] = $response['access_token'];
}
}

###
#OUTPUT
###
function getSearchQueryComplexity($purl)
{
    $words = explode(' ', $purl);
    $uniqueWords = count($words);
    $averageWordLength = array_sum(array_map('strlen', $words)) / $uniqueWords;
    $specialCharacterCount = 1;
    foreach ($words as $word) {
        $specialCharacterCount += (strpos($word, '[^a-zA-Z0-9]') !== false);
    }
    return $complexityScore = $uniqueWords * $averageWordLength * $specialCharacterCount;
}
function apiRandom($apis = array())
{
    shuffle($apis);
    return $apis[0];
}
function chooseAPI($disabled, &$complex)
{
    if ($complex >= 20) {
        $apis = [];
        if ($disabled[0] == 0) {
            $apis[] = 0;
        }
        if ($disabled[1] == 0) {
            $apis[] = 1;
        }

        if (empty($apis)) {
            $complex = 19;
        } else {
            $searchId = apiRandom($apis);
        }
    }

    if ($complex >= 10 && $complex < 20) {
        $apis = [];
        if ($disabled[1] == 0) {
            $apis[] = 1;
        }
        if ($disabled[2] == 0) {
            $apis[] = 2;
        }

        if (empty($apis)) {
            $complex = 9;
        } else {
            $searchId = apiRandom($apis);
        }
    }
    if ($complex >= 10 && $complex < 10) {
        $apis = [];
        if ($disabled[2] == 0) {
            $apis[] = 2;
        }
        if ($disabled[3] == 0) {
            $apis[] = 3;
        }

        if (empty($apis)) {
            $complex = 9;
        } else {
            $searchId = apiRandom($apis);
        }
    }
    if ($complex < 10) {
        $apis = [];
        if ($disabled[3] == 0) {
            $apis[] = 3;
        }
        if ($disabled[4] == 0) {
            $apis[] = 4;
        }

        if (empty($apis)) {
            $complex = 5;
        }
        $searchId = apiRandom($apis);
    }
    if ($complex <= 5) {
        if ($disabled[4] == 0) {
            $apis[] = 4;
        }
        if ($disabled[5] == 0) {
            $apis[] = 5;
        }

        $searchId = apiRandom($apis);
    }
    return $searchId;
}
function backupCall($searchId, $disabled, &$complex, $Bpurl, $Googlefile, $BraveFile, $MojeekFile, $lang, $page)
{
    if ($complex >= 30) {
        $complex = 29;
    } elseif ($complex >= 20) {
        $complex = 19;
    } else {
        $complex = 9;
    }
    $searchId = chooseAPI($disabled, $complex);

    switch ($searchId) {
        case 0:
            $ch3 = curl_init($Googlefile);
            break;
        case 1:
            $ch3 = curl_init('https://jojoyou.org/librex/api.php?p=0&t=0&g2API=' . $_ENV['PriEcoGoogle'] . '&q=' . $Bpurl);
            $tmp = $lang;
            if ($lang = 'all') {
                $tmp = 'en';
            }
            $cookies = 'google_language_results=' . $tmp . ';google_number_of_results=20;google_language_site=' . $tmp . ';';
            if (!isset($_COOKIE['safe'])) {
                $cookies .= 'safe_search=on;';
            }
            curl_setopt($ch3, CURLOPT_COOKIE, $cookies);
            break;
        case 2:
            $ch3 = curl_init('https://api.qwant.com/v3/search/web/?count=10&offset=' . $page . '0&uiv=1&locale=en_us&q=' . $Bpurl);
            curl_setopt($ch3, CURLOPT_HTTPHEADER, [
                "Accept: text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/png,image/svg+xml,*/*;q=0.8",
                "Accept-Language: en-US,en;q=0.5",
                "Connection: keep-alive",
                "DNT: 1",
                "Host: api.qwant.com",
                "Priority: u=0, i",
                "Sec-Fetch-Dest: document",
                "Sec-Fetch-Mode: navigate",
                "Sec-Fetch-Site: cross-site",
                "TE: trailers",
                "Upgrade-Insecure-Requests: 1",
                "User-Agent: $randomUserAgent"
            ]);            break;
        case 3:
            $ch3 = curl_init('https://jojoyou.org/librex/bing.php?g2API=' . $_ENV['PriEcoGoogle'] . '&q=' . $Bpurl);
            curl_setopt($ch3, CURLOPT_USERAGENT, 'Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13');
            break;
        case 4:
            $ch3 = curl_init($BraveFile);
            $headers = array(
                'X-Subscription-Token: '. $_ENV['BRAVE_API_KEY'],
                'Accept: application/json',
                'Authorization: Bearer my-access-token'
            );

            curl_setopt($ch3, CURLOPT_HTTPHEADER, $headers);
            break;
        case 5:
            $ch3 = curl_init($MojeekFile);
            break;
    }

    curl_setopt($ch3, CURLOPT_RETURNTRANSFER, true);
    curl_setopt($ch3, CURLOPT_CONNECTTIMEOUT, 2.5);

    if ($searchId == 4) {
        $response = curl_exec($ch3);
    } else {
        $response = json_decode(curl_exec($ch3), true);
    }

    curl_close($ch3);
    return [$response, $searchId];
}

function encrypt($text, $key, $iv, $cipher) {
    $encrypted = openssl_encrypt($text, $cipher, $key, 0, $iv);
    return base64_encode($encrypted);
}

function decrypt($encryptedText, $key, $iv, $cipher) {
    $decoded = base64_decode($encryptedText);
    return openssl_decrypt($decoded, $cipher, $key, 0, $iv);
}
