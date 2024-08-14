<!--  SPDX-FileCopyrightText: 2022, 2022-2022 Roman  Láncoš <jojoyou@jojoyou.org> -->
<!-- -->
<!--  SPDX-License-Identifier: AGPL-3.0-or-later -->

<?php
$fil_enc_que = urlencode(filter_input(INPUT_GET, "q", FILTER_SANITIZE_STRING));
$cipher = "aes-256-cbc";

if (
    $type != "image" &&
    $type != "video" &&
    $type != "news" &&
    $type != "shop"
) {
    function get_string_betweens($string, $start, $end)
    {
        $string = " " . $string;
        $ini = strpos($string, $start);
        if ($ini === false) {
            return "";
        }
        $ini += strlen($start);
        $len = strpos($string, $end, $ini);
        if ($len === false || $len <= $ini) {
            return "";
        }
        $len -= $ini;
        if ($len < 0) {
            return "";
        }

        return substr($string, $ini, $len);
    }

    //Initial call
    $indexY = true;
    if (!$dev) {
        $ImpProfiles = false;
        $ImpGoogle = true;
        $name = strtolower($purl);
        if (
            isset(
                $_SESSION[$purl . $lang . $loc . $safeS . $_COOKIE["time"]]
            ) &&
            !isset($_COOKIE["index"])
        ) {
            if (
                strpos(
                    $_SESSION[$purl . $lang . $loc . $safeS . $_COOKIE["time"]],
                    "obj +=+ "
                ) !== false
            ) {
                $obj = json_decode(
                    str_replace(
                        "obj +=+ ",
                        "",
                        $_SESSION[
                            $purl . $lang . $loc . $safeS . $_COOKIE["time"]
                        ]
                    ),
                    true
                );
                $searchId = 0;
            }
            if (
                strpos(
                    $_SESSION[$purl . $lang . $loc . $safeS . $_COOKIE["time"]],
                    "g2obj +=+ "
                ) !== false
            ) {
                $g2obj = json_decode(
                    str_replace(
                        "g2obj +=+ ",
                        "",
                        $_SESSION[
                            $purl . $lang . $loc . $safeS . $_COOKIE["time"]
                        ]
                    ),
                    true
                );
                $searchId = 1;
            }
            if (
                strpos(
                    $_SESSION[$purl . $lang . $loc . $safeS . $_COOKIE["time"]],
                    "qwant +=+ "
                ) !== false
            ) {
                $QWantObj = json_decode(
                    str_replace(
                        "qwant +=+ ",
                        "",
                        $_SESSION[
                            $purl . $lang . $loc . $safeS . $_COOKIE["time"]
                        ]
                    ),
                    true
                );
                $searchId = 2;
            }
            if (
                strpos(
                    $_SESSION[$purl . $lang . $loc . $safeS . $_COOKIE["time"]],
                    "brave +=+ "
                ) !== false
            ) {
                $BraveObj = str_replace(
                    "brave +=+ ",
                    "",
                    $_SESSION[$purl . $lang . $loc . $safeS . $_COOKIE["time"]]
                );
                $searchId = 4;
            }
            if (
                strpos(
                    $_SESSION[$purl . $lang . $loc . $safeS . $_COOKIE["time"]],
                    "mojeek +=+ "
                ) !== false
            ) {
                $MojeekObj = json_decode(
                    str_replace(
                        "mojeek +=+ ",
                        "",
                        $_SESSION[
                            $purl . $lang . $loc . $safeS . $_COOKIE["time"]
                        ]
                    ),
                    true
                );
                $searchId = 5;
            }

            if (isset($_SESSION[$purl . ":-:shop"])) {
                $ShopObj = json_decode($_SESSION[$purl . ":-:shop"], true);
            }
            if (isset($_SESSION[$purl . ":-:wiki"])) {
                $wikiobj = explode("--]|[--", $_SESSION[$purl . ":-:wiki"]);
                $wikiTxt = $wikiobj[1];
                $ddgObj = json_decode($wikiobj[2], true);
                $wikiobj = json_decode($wikiobj[0], true);
                $indexW = true;
            }
            if (isset($_SESSION[$purl . ":-:new"])) {
                $NewsObj = $_SESSION[$purl . ":-:new"];
                $indexN = true;
            }
            if (isset($_SESSION[$purl . ":-:red"])) {
                $redditObj = $_SESSION[$purl . ":-:red"];
            }
            if (isset($_SESSION[$purl . ":-:yt"])) {
                $YoutubeObj = $_SESSION[$purl . ":-:yt"];
            }
            if (isset($_SESSION[$purl . ":-:rel"])) {
                $related = $_SESSION[$purl . ":-:rel"];
            }
            if (isset($_SESSION[$purl . ":-:priimg"])) {
                $PriEcoImg = $_SESSION[$purl . ":-:priimg"];
            }
            if (preg_match("/\bip\b/i", $purl)) {
                $ipCh = curl_init(
                    "http://127.0.0.1:8000/api/ip2loc?q=" .
                        ip2long(getenv("REMOTE_ADDR"))
                );
                curl_setopt($ipCh, CURLOPT_RETURNTRANSFER, true);
                curl_setopt(
                    $ipCh,
                    CURLOPT_USERAGENT,
                    "Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13"
                );
                curl_setopt($ipCh, CURLOPT_CONNECTTIMEOUT, 2);
                curl_setopt($ipCh, CURLOPT_TIMEOUT, 2.5);
                $ipObj = json_decode(curl_exec($ipCh), true);
                curl_close($ipCh);
            }
            $cached = true;
            $ImpGoogle = false;
        } else {
            $priecoTime = microtime(true);
            //Google
            $hashedQuery = hash("md5", $purl . $lang . $loc);

            if (file_exists("cache/" . $hashedQuery . ".txt")) {
                $lines = file("cache/" . $hashedQuery . ".txt");
                if (!empty($lines)) {
                    $date = explode("---", array_shift($lines));
                    $searchId = $lines[1];
                    $date = $date[1];
                    $cache = decrypt(
                        gzuncompress(implode("", $lines)),
                        $purl . $lang . $loc,
                        $purl . $lang . $loc,
                        $cipher
                    );
                }
            }
            if (isset($cache)) {
                switch ($searchId) {
                    case 0:
                        $obj = json_decode($cache, true);
                        break;
                    case 1:
                        $g2obj = json_decode($cache, true);
                        break;
                    case 2:
                    case 3:
                        $QWantObj = json_decode($cache, true);
                        break;
                    case 4:
                        $BraveObj = json_decode($cache, true);
                        break;
                    case 5:
                        $MojeekObj = json_decode($cache, true);
                        break;
                }
                $ImpGoogle = false;
            }
            $mh = curl_multi_init();

            $e = $_ENV["Obunic"];
            $wiki_query = isset($_GET['lyrics_artist']) ? urlencode($_GET['lyrics_artist']) : $fil_enc_que;

            $ch2 = curl_init("http://127.0.0.1:8000/api/wiki?q=$wiki_query"); // Wiki
            $ch3 = curl_init("http://127.0.0.1:8000/api/reddit?q=$fil_enc_que"); // Reddit
            $ch4 = curl_init("http://127.0.0.1:8000/api/news?q=$fil_enc_que"); // News
            $ch5 = curl_init(
                "http://127.0.0.1:8000/api/youtube?q=$fil_enc_que"
            ); // YouTube
            $ch6 = curl_init(
                "http://127.0.0.1:8000/api/image?api=$e&lang=$lang&loc=$loc&q=$fil_enc_que"
            ); // PriEco Images
            $ch8 = curl_init(
                "http://127.0.0.1:8000/api/sug/?q=" . $fil_enc_que . "&lang=en"
            ); // Related searches

            curl_setopt($ch2, CURLOPT_RETURNTRANSFER, true);
            curl_setopt($ch3, CURLOPT_RETURNTRANSFER, true);
            curl_setopt($ch4, CURLOPT_RETURNTRANSFER, true);
            curl_setopt($ch5, CURLOPT_RETURNTRANSFER, true);
            curl_setopt($ch6, CURLOPT_RETURNTRANSFER, true);
            curl_setopt($ch8, CURLOPT_RETURNTRANSFER, true);

            curl_multi_add_handle($mh, $ch2);
            curl_multi_add_handle($mh, $ch3);
            curl_multi_add_handle($mh, $ch4);
            curl_multi_add_handle($mh, $ch5);
            curl_multi_add_handle($mh, $ch6);
            curl_multi_add_handle($mh, $ch8);

            // PriEco Results
            if (strpos($purl, " ") !== false) {
                $words = explode(" ", $purl);
                foreach ($words as $word) {
                    $ch = curl_init(
                        "http://127.0.0.1:8000/api/indexing/?q=" .
                            urlencode($word)
                    );
                    curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);
                    curl_multi_add_handle($mh, $ch);
                    $indexing_handles[] = $ch;
                }
            } else {
                $ch7 = curl_init(
                    "http://127.0.0.1:8000/api/indexing/?q=" . urlencode($purl)
                );
                curl_setopt($ch7, CURLOPT_RETURNTRANSFER, true);
                curl_multi_add_handle($mh, $ch7);
                $indexing_handles[] = $ch7;
            }
            $running = null;
            do {
                curl_multi_exec($mh, $running);
                curl_multi_select($mh);
            } while ($running > 0);

            curl_multi_remove_handle($mh, $ch2);
            curl_multi_remove_handle($mh, $ch3);
            curl_multi_remove_handle($mh, $ch4);
            curl_multi_remove_handle($mh, $ch5);
            curl_multi_remove_handle($mh, $ch6);
            curl_multi_remove_handle($mh, $ch8);

            $wikiObj = json_decode(curl_multi_getcontent($ch2), true);
            $redditObj = json_decode(curl_multi_getcontent($ch3), true);
            $NewsObj = json_decode(curl_multi_getcontent($ch4), true);
            $YoutubeObj = json_decode(curl_multi_getcontent($ch5), true);
            $PriEcoImg = json_decode(curl_multi_getcontent($ch6), true);
            $tmp = json_decode(curl_multi_getcontent($ch8), true);
            $PriEcoObj = [];
            foreach ($indexing_handles as $ch) {
                $response = json_decode(curl_multi_getcontent($ch), true);
                if (is_array($response)) {
                    $PriEcoObj = array_merge($PriEcoObj, $response);
                }
                curl_multi_remove_handle($mh, $ch);
                curl_close($ch);
            }
            curl_multi_close($mh);

            //Wiki
            $indexW = false;
            foreach ($wikiObj as $item) {
                if (strtolower($item["title"]) === strtolower($purl) || strtolower($item["title"]) === strtolower($_GET['lyrics_artist'])) {
                    $wikiObj = $item;
                    $indexW = true;
                    break;
                }
            }

            if (!empty($wikiObj) && $indexW) {
                $wikiTxt = urldecode($wikiObj["paragraph"]);
                $ddgObj = json_decode(
                    str_replace("%22", '"', $wikiObj["profiles"]),
                    true
                );
                $wikiobj = stripslashes(urldecode($wikiObj["infobox"]));
            } else {
                $wikiObj = "";
            }

            //News
            if (!isset($NewsObj) || count($NewsObj) < 3) {
                $indexN = false;
            } else {
                $indexN = true;
            }

            //YouTube
            if (!isset($YoutubeObj) || count($YoutubeObj) < 3) {
                $indexY = false;
                unset($YoutubeObj);
            } else {
                $indexY = true;
            }
            //Related
            $suggestions = [];
            foreach ($tmp as $row) {
                $suggestions[] = [
                    "name" => $row["name"],
                    "img" => $row["img"],
                ];
            }
            $similarityScores = [];
            foreach ($suggestions as $suggestion) {
                if (
                    substr($suggestion["name"], 0, 1) === "!" &&
                    substr($purl, 0, 1) !== "!"
                ) {
                    continue;
                }
                if (strlen($suggestion["name"]) < strlen($purl)) {
                    continue;
                }

                $similarityScores[$suggestion["name"]] =
                    similar_text($purl, $suggestion["name"]) +
                    (10 - strlen($suggestion["name"]));
            }

            arsort($similarityScores);

            $topSuggestions = array_keys(array_slice($similarityScores, 0, 6));
            $related = [];
            foreach ($topSuggestions as $suggestion) {
                foreach ($suggestions as $item) {
                    if ($item["name"] === $suggestion) {
                        $related[] = $item;
                        break;
                    }
                }
            }
            $priecoTime = microtime(true) - $priecoTime;
        }

        // Initialize multi-curl handle
        $multiHandle = curl_multi_init();

        // Initialize curl handles for each request
        $curlHandles = [];
        $curlCount = 0;
        $disabled = array_fill(0, 6, 1);
        $disabled[4] = 0;

        // Request 3: Googlefile, Brave or Qwant
        if (
            $obj == "" &&
            ($g2obj == "" or $g2obj["status"] == "error") &&
            !isset($QWantObj) &&
            !isset($BraveObj) &&
            !isset($MojeekObj) &&
            !isset($_COOKIE["index"])
        ) {
            $complex = getSearchQueryComplexity($purl);

            $disabled = array_fill(0, 6, 0);
            if (file_exists("disGoogle.txt")) {
                $disabled[0] = 1;
            }
            if (file_exists("disGoogle2.txt")) {
                $disabled[1] = 1;
            }
            if (file_exists("disBing.txt")) {
                $disabled[2] = 1;
            }
            if (file_exists("disBing2.txt")) {
                $disabled[3] = 1;
            }
            if (file_exists("disBrave.txt")) {
                $disabled[4] = 1;
            }
            if (file_exists("disMojeek.txt")) {
                $disabled[5] = 1;
            }

            $searchId = chooseAPI($disabled, $complex);
            switch ($searchId) {
                case 0:
                    $ch3 = curl_init($Googlefile);
                    break;
                case 1:
                $tmp = $lang;
                if ($tmp = "all" || empty($tmp) || !isset($tmp)) {
                    $tmp = "en";
                }
                    $ch3 = curl_init(
                        'https://obunic.net/tests/prieco/?s=g&google_language_results='.$tmp.'&google_language_site='.$tmp.'&api=' .
                            $_ENV["Index2"] .
                            '&q=' .
                            $Bpurl
                    );

                    curl_setopt($ch3, CURLOPT_COOKIE, $cookies);
                    break;
                case 2:
                    $ch3 = curl_init(
                        'https://api.qwant.com/v3/search/web/?count=10&offset=' .
                            $page .
                            '0&uiv=1&locale=en_us&q=' .
                            $Bpurl
                    );
                    curl_setopt(
                        $ch3,
                        CURLOPT_USERAGENT,
                        "Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13"
                    );
                    break;
                case 3:
                    $ch3 = curl_init(
                        "https://obunic.net/tests/prieco/?s=b&api=" .
                            $_ENV["Index2"] .
                            "&q=" .
                            $Bpurl
                    );
                    curl_setopt(
                        $ch3,
                        CURLOPT_USERAGENT,
                        "Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13"
                    );
                    break;
                case 4:
                    $ch3 = curl_init($BraveFile);
                    curl_setopt($ch3, CURLOPT_HTTPHEADER, [
                        "Accept: application/json",
                        "X-Subscription-Token: " . $_ENV["BRAVE_API_KEY"],
                    ]);
                    break;
                case 5:
                    $ch3 = curl_init($MojeekFile);
                    break;
            }

            curl_setopt($ch3, CURLOPT_RETURNTRANSFER, true);
            curl_setopt($ch3, CURLOPT_CONNECTTIMEOUT, 2.5);
            $curlHandles[] = $ch3;
            if (count($curlHandles) > $curlCount) {
                $searchOn = true;
                $curlCount = count($curlHandles);
            }
        }

        $tmp = $_COOKIE["Language"];
        if ($tmp == "all" or $tmp == null) {
            $tmp = "en";
        }

        //Define
        if (isset($defWords) && !isset($_COOKIE["index"])) {
            $defCh = curl_init($WordnikFile);
            curl_setopt($defCh, CURLOPT_RETURNTRANSFER, true);
            curl_setopt(
                $defCh,
                CURLOPT_USERAGENT,
                "Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13"
            );
            curl_setopt($defCh, CURLOPT_CONNECTTIMEOUT, 2);
            curl_setopt($defCh, CURLOPT_TIMEOUT, 2.5);

            $curlHandles[] = $defCh;
            if (count($curlHandles) > $curlCount) {
                $defOn = true;
                $curlCount = count($curlHandles);
            }
        }
        //Weather
        if ($weatherTrue && !isset($_COOKIE["index"])) {
            $weatherCh = curl_init($OpenWeatherFile);
            curl_setopt($weatherCh, CURLOPT_RETURNTRANSFER, true);
            curl_setopt(
                $weatherCh,
                CURLOPT_USERAGENT,
                "Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13"
            );
            curl_setopt($weatherCh, CURLOPT_CONNECTTIMEOUT, 2);
            curl_setopt($weatherCh, CURLOPT_TIMEOUT, 2.5);

            $curlHandles[] = $weatherCh;
            if (count($curlHandles) > $curlCount) {
                $weatherOn = true;
                $curlCount = count($curlHandles);
            }

            $weatherForecastCh = curl_init($OpenWeatherForecastFile);
            curl_setopt($weatherForecastCh, CURLOPT_RETURNTRANSFER, true);
            curl_setopt(
                $weatherForecastCh,
                CURLOPT_USERAGENT,
                "Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13"
            );
            curl_setopt($weatherForecastCh, CURLOPT_CONNECTTIMEOUT, 2);
            curl_setopt($weatherForecastCh, CURLOPT_TIMEOUT, 2.5);
            $curlHandles[] = $weatherForecastCh;
            if (count($curlHandles) > $curlCount) {
                $weatherForecastOn = true;
                $curlCount = count($curlHandles);
            }
        }

        //IP
        if (preg_match("/\bip\b/i", $purl)) {
            $ipCh = curl_init(
                "https://prieco.net/api/ipAPI.php/?ip=" .
                    $_SERVER["REMOTE_ADDR"]
            );
            curl_setopt($ipCh, CURLOPT_RETURNTRANSFER, true);
            curl_setopt(
                $ipCh,
                CURLOPT_USERAGENT,
                "Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13"
            );
            curl_setopt($ipCh, CURLOPT_CONNECTTIMEOUT, 2);
            curl_setopt($ipCh, CURLOPT_TIMEOUT, 2.5);
            $curlHandles[] = $ipCh;
            if (count($curlHandles) > $curlCount) {
                $ipOn = true;
                $curlCount = count($curlHandles);
            }
        }

        //Crypto
        /*$cryptoCh = curl_init('https://api.coingecko.com/api/v3/coins/'.$purl);
        curl_setopt($cryptoCh, CURLOPT_RETURNTRANSFER, true);
        curl_setopt($cryptoCh, CURLOPT_USERAGENT, 'Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13');
        curl_setopt($cryptoCh, CURLOPT_CONNECTTIMEOUT, 2);
        $curlHandles[] = $cryptoCh;
        if(count($curlHandles) > $curlCount){
            $cryptoOn = true;
            $curlCount = count($curlHandles);
        }*/

        //Wikipedia
        if (!$indexW && !isset($_COOKIE["index"])) {
            $wikiCh = curl_init(
                "https://prieco.net/Controller/functions/addons/wikiGet.php?lang=" .
                    $tmp .
                    "&q=" .
                    str_replace("+", "_", urlencode(ucwords($purl)))
            );
            curl_setopt($wikiCh, CURLOPT_RETURNTRANSFER, true);
            curl_setopt(
                $wikiCh,
                CURLOPT_USERAGENT,
                "Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13"
            );
            curl_setopt($wikiCh, CURLOPT_CONNECTTIMEOUT, 2);
            curl_setopt($wikiCh, CURLOPT_TIMEOUT, 2.5);

            $curlHandles[] = $wikiCh;
            if (count($curlHandles) > $curlCount) {
                $wikiOn = true;
                $curlCount = count($curlHandles);
            }
        }
        //News
        if (!$indexN && !isset($_COOKIE["index"])) {
            $newsCh = curl_init($NewsFile);
            curl_setopt($newsCh, CURLOPT_RETURNTRANSFER, true);
            curl_setopt(
                $newsCh,
                CURLOPT_USERAGENT,
                "Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13"
            );
            curl_setopt($newsCh, CURLOPT_CONNECTTIMEOUT, 2);
            curl_setopt($newsCh, CURLOPT_TIMEOUT, 2.5);
            $curlHandles[] = $newsCh;
            if (count($curlHandles) > $curlCount) {
                $newsOn = true;
                $curlCount = count($curlHandles);
            }
        }
        //YouTube
        if (!$indexY && !isset($_COOKIE["index"])) {
            $ytCh = curl_init($YoutubeFile);
            curl_setopt($ytCh, CURLOPT_RETURNTRANSFER, true);
            curl_setopt(
                $ytCh,
                CURLOPT_USERAGENT,
                "Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13"
            );
            curl_setopt($ytCh, CURLOPT_CONNECTTIMEOUT, 2);
            curl_setopt($ytCh, CURLOPT_TIMEOUT, 2.5);
            $curlHandles[] = $ytCh;
            if (count($curlHandles) > $curlCount) {
                $ytOn = true;
                $curlCount = count($curlHandles);
            }
        }

        //Shop Ads
        $tmp = $loc == "all" ? "us" : $loc;

        if (!isset($_SESSION[$purl . ":-:shop"]) && !isset($_COOKIE["index"])) {
            $shopCh = curl_init(
                "https://api.yadore.com/v2/offer?market=" .
                    $tmp .
                    "&keyword=" .
                    $Bpurl .
                    "&precision=fuzzy&sort=rel_desc&limit=20"
            );
            curl_setopt($shopCh, CURLOPT_RETURNTRANSFER, true);

            $headers = ["API-Key: " . $_ENV["SHOP_API_KEY"]];
            curl_setopt($shopCh, CURLOPT_HTTPHEADER, $headers);
            $curlHandles[] = $shopCh;
            if (count($curlHandles) > $curlCount) {
                $shopOn = true;
                $curlCount = count($curlHandles);
            }
        }

        preg_match('/song:\s*(.*?)\s*artist:\s*(.*)/', $purl, $matches);

        $artist = $matches[2];
        $song = $matches[1];

        if(strpos($purl, 'lyric') !== false || (!empty($artist) && !empty($song)) || (isset($_GET['album']) && $_GET['album'] == 'true')){
        $song_name = trim(preg_replace('/\b(l(yric|yrics))\b/i', '', $purl));

          if(isset($_GET['lyrics_id'])){
          $lyrCh =curl_init($_ENV['ObunicSongId'].$_GET['lyrics_id']);
          }
          elseif(isset($_GET['album']) && !empty($_GET['album'])){
            if($_GET['album'] != 'true'){
              $lyrCh =curl_init($_ENV['ObunicAlbumId'].$_GET['album']);
            }
            else{
              $lyrCh =curl_init($_ENV['ObunicAlbumName'].urlencode(str_replace('album: ','',$purl)));
            }
          }
          elseif(!empty($artist) && !empty($song)){
          $lyrCh =curl_init($_ENV['ObunicSong'].str_replace('+', '%20',urlencode($song)).'&artist='.str_replace('+', '%20',urlencode($artist)));
          }
          else{
          $lyrCh =curl_init($_ENV['ObunicLyrics'].urlencode($song_name));
          }
            curl_setopt($lyrCh, CURLOPT_RETURNTRANSFER, true);
            curl_setopt(
                $lyrCh,
                CURLOPT_USERAGENT,
                "Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13"
            );
            curl_setopt($lyrCh, CURLOPT_CONNECTTIMEOUT, 2);
            curl_setopt($lyrCh, CURLOPT_TIMEOUT, 2.5);
            $curlHandles[] = $lyrCh;
            if (count($curlHandles) > $curlCount) {
                $lyrOn = true;
                $curlCount = count($curlHandles);
            }

            if(isset($_GET['lyrics_artist'])){
            $artCh =curl_init($_ENV['ObunicArtist'].urlencode($_GET['lyrics_artist']));
            curl_setopt($artCh, CURLOPT_RETURNTRANSFER, true);
            curl_setopt(
                $artCh,
                CURLOPT_USERAGENT,
                "Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13"
            );
            $curlHandles[] = $artCh;
            if (count($curlHandles) > $curlCount) {
                $artOn = true;
                $curlCount = count($curlHandles);
            }

            }
        }

        // Execute multi-curl requests
        foreach ($curlHandles as $handle) {
            curl_multi_add_handle($multiHandle, $handle);
        }

        $running = null;
        do {
            curl_multi_exec($multiHandle, $running);
            curl_multi_select($multiHandle);
        } while ($running > 0);

        // Close the multi-curl handle
        curl_multi_close($multiHandle);

        // Get response content and error codes for each request
        foreach ($curlHandles as $handle) {
            if (curl_error($handle) === "") {
                $response = curl_multi_getcontent($handle);
                if (isset($searchOn)) {
                    switch ($searchId) {
                        case 0:
                            $obj = json_decode($response, true);
                            unset($searchOn);
                            break;
                        case 1:
                            $g2obj = json_decode($response, true);
                            unset($searchOn);
                            break;
                        case 2:
                        case 3:
                            $QWantObj = json_decode($response, true);
                            unset($searchOn);
                            break;
                        case 4:
                            $BraveObj = json_decode($response, true);
                            unset($searchOn);
                            break;
                        case 5:
                            $MojeekObj = json_decode($response, true);
                            unset($searchOn);
                            break;
                    }
                } elseif (isset($defOn) && !isset($WordnikObj)) {
                    $WordnikObj = json_decode($response, true);
                    unset($defOn);
                } elseif (isset($weatherOn) && !isset($OpenWeatherObj)) {
                    $OpenWeatherObj = json_decode($response, true);
                    unset($weatherOn);
                } elseif (
                    isset($weatherForecastOn) &&
                    !isset($OpenWeatherForecastObj)
                ) {
                    $OpenWeatherForecastObj = json_decode($response, true);
                    unset($weatherForecastOn);
                } elseif (isset($ipOn) && !isset($ipObj)) {
                    $ipObj = json_decode($response, true);
                    unset($ipOn);
                } elseif (isset($cryptoOn) && !isset($cryptoData)) {
                    $cryptoData = json_decode($response, true);
                    unset($cryptoOn);
                } elseif (isset($wikiOn) && empty($wikiobj)) {
                    $_SESSION[$purl . ":-:wiki"] = $response;
                    $wikiobj = explode("--]|[--", $response);
                    $wikiTxt = $wikiobj[1];
                    $wikiobj = json_decode($wikiobj[0], true);
                    unset($wikiOn);
                } elseif (isset($newsOn) && empty($NewsObj)) {
                    $NewsObj = [];
                    $NewsObjN = json_decode($response, true);
                    foreach ($NewsObjN["articles"] as $article) {
                        $NewsObj[] = [
                            "title" => $article["title"],
                            "description" => $article["description"],
                            "img" => $article["urlToImage"],
                            "url" => $article["url"],
                            "date" => $article["publishedAt"],
                        ];
                    }
                    usort($NewsObj, function ($a, $b) {
                        return strtotime($b["date"]) - strtotime($a["date"]);
                    });

                    unset($newsOn);
                } elseif (isset($ytOn) && empty($YoutubeObj)) {
                    $YoutubeObj = [];
                    $YoutubeObjN = json_decode($response, true);
                    foreach ($YoutubeObjN["items"] as $item) {
                        $title = substr($item["snippet"]["title"], 0, 47);
                        $description = substr(
                            $item["snippet"]["description"],
                            0,
                            47
                        );

                        $YoutubeObj[] = [
                            "title" => $title,
                            "description" => $description,
                            "url" => $item["id"]["videoId"],
                            "thumb" => str_replace(
                                "https://i.ytimg.com/vi/",
                                "",
                                $item["snippet"]["thumbnails"]["medium"]["url"]
                            ),
                            "date" => $item["snippet"]["publishTime"],
                        ];
                    }
                    unset($ytOn);
                } elseif (isset($shopOn) && !isset($ShopObj)) {
                    $ShopObj = json_decode($response, true);
                    unset($shopOn);
                } elseif (isset($lyrOn) && !isset($LyricsObj)){
                if(isset($_GET['lyrics_id']) || (!empty($artist) && !empty($song))){
                $LyricsObj = json_decode('['.$response.']', true);
                }
                else{
                $LyricsObj = json_decode($response, true);
                }
                unset($lyrOn);
                }
                elseif (isset($artOn) && !isset($artistObj)){
                $artistObj = json_decode($response, true);
                  unset($artOn);
                }
            }
            curl_multi_remove_handle($multiHandle, $handle);
        }
        curl_multi_close($multiHandle);
    } else {
        $searchId = 0;
        $obj = json_decode(file_get_contents($Googlefile), true);
        $redditObj = json_decode(file_get_contents($RedditFile), true);
        $ddgObj = json_decode(file_get_contents($DdgFile), true);
        $related = json_decode(
            file_get_contents("./Controller/dev/sug.json"),
            true
        );
        $OpenWeatherObj = json_decode(
            file_get_contents($OpenWeatherFile),
            true
        );
        $OpenWeatherForecastObj = json_decode(
            file_get_contents($OpenWeatherForecastFile),
            true
        );
        $ipObj = json_decode(
            file_get_contents("./Controller/dev/ip.json"),
            true
        );
        $NewsObj = json_decode(
            file_get_contents("./Controller/dev/news.json"),
            true
        );
        $YoutubeObj = json_decode(
            file_get_contents("./Controller/dev/yt.json"),
            true
        );
        $WordnikObj = json_decode(
            file_get_contents("./Controller/dev/def.json"),
            true
        );
        $ShopObj = json_decode(
            file_get_contents("Controller/dev/yadore.json"),
            true
        );
        $wikiobj = explode(
            "--]|[--",
            file_get_contents("./Controller/dev/wiki.txt")
        );
        $wikiTxt = $wikiobj[1];
        $wikiobj = json_decode($wikiobj[0], true);
    }

    //Engines
    include "engines/web/remote.php";
    include "engines/web/promo.php";
    include "engines/web/prieco.php";
    //Addons
    include "addons/wiki.php";
    include "addons/hideQuery.php";
    include "addons/reddit.php";
    include "addons/related.php";
    include "addons/topImgs.php";
    include "addons/answer.php";
    include "addons/lyrics.php";
    //include "addons/small/peopleAsk.php";

    include "addons/news.php";
    include "addons/yt.php";

    include "addons/pHigh.php";
    include "addons/shop.php";

    include "addons/small/ip.php";

    if (preg_match("/\b(?:random|rand)\b/i", $purl)) {
        include "addons/small/random.php";
    }
    if (
        preg_match("/\b(?:calculator|calc)\b/i", $purl) &&
        !isset($_COOKIE["DisWid"])
    ) {
        include "addons/small/calculator.php";
    }
    if (
        preg_match("/\b(?:color|col)\b/i", $purl) &&
        !isset($_COOKIE["DisWid"])
    ) {
        include "addons/small/color.html";
    }
    include "addons/small/define.php";
    if (isset($OpenWeatherObj)) {
        include "addons/small/weather.php";
    }
    /* include 'addons/small/crypto.php';
     include 'addons/small/convert.php';
     */
    //include 'addons/small/didyoumean.php';
    if (!$dev) {
        include "addons/summarizer/sum.php";
    }

    //Next page
    /* function nextPage($purl,$page){
         if($page > 0){
             echo '<div class="nextPage" style="margin-left: calc(9vw + (clamp(552px, 675px, 90vw) - 210px) / 2);">';
             if(!isset($_COOKIE['hQuery'])){
             echo '<a href="/?q=',htmlspecialchars($purl, ENT_QUOTES | ENT_HTML5, 'UTF-8'),'&page=',$page-1,'">';
             }
             else{
                 echo '<form method="POST" action="" style="display:inline;">
                 <input type="hidden" name="q" value="', $purl,'">
                 <input type="hidden" name="page" value="', $page-1 ,'">
                 ';
             }
                 echo '<button type="submit">⬅ Back</button>';
                 if(!isset($_COOKIE['hQuery'])){echo '</a>';}
                 else{echo '</form>';}
             echo '<p style="float:left;color:#8f8f8f;padding:10px;padding-top:22px;">Page: ',$page+1,'</p>';

             if(!isset($_COOKIE['hQuery'])){
                 echo '<a href="/?q=',htmlspecialchars($purl, ENT_QUOTES | ENT_HTML5, 'UTF-8'),'&page=',$page+1,'">';
                 }
                 else{
                     echo '<form method="POST" action="" style="display:inline;">
                     <input type="hidden" name="q" value="', $purl,'">
                     <input type="hidden" name="page" value="', $page+1 ,'">
                     ';
                 }
                 echo'<button type="submit">Next ⮕</button>';
                 if(!isset($_COOKIE['hQuery'])){echo '</a>';}
                 else{echo '</form>';}
                 echo '</div>
             <style>@media (max-width: 890px) {.nextPage {width:calc(50% + 115px);}}</style>';
         }
         else{
             echo '<div class="nextPage" style="margin-left: calc(9vw + (clamp(552px, 675px, 90vw) - 210px) / 2);">';
             if(!isset($_COOKIE['hQuery'])){
                 echo '<a href="/?q=',htmlspecialchars($purl, ENT_QUOTES | ENT_HTML5, 'UTF-8'),'&page=',$page+1,'">';
                 }
                 else{
                     echo '<form method="POST" action="">
                     <input type="hidden" name="q" value="', $purl,'">
                     <input type="hidden" name="page" value="', $page+1 ,'">
                     ';
                 }
                 echo '<button type="submit">Next ⮕</button>';

                 if(!isset($_COOKIE['hQuery'])){echo '</a>';}
                 else{echo '</form>';}
             echo '</div>
             <style>@media (max-width: 890px) {.nextPage {width:calc(50% + 40px);}}</style>';
         }
     }

     $loaded = array_fill(0, 7, false);
     if($page > 0){
         $results = qwant(null, $loaded, $Bpurl, $page);
         foreach($results as &$rs){
             echo $rs;
         }
         nextPage($purl, $page);
         return;
     }*/

    ##
    #Printing
    ##
    //Addons
    $lyrics = $questionAnswer = $wiki = $news = $reddit = $youtube = $relatedP = "";
    if (isset($_COOKIE["hQuery"])) {
        $hideQueryCopy = hideQuery($Bpurl);
    } else {
        $hideQueryCopy = null;
    }

    if (isset($wikiTxt) && !empty($wikiTxt)) {
        if (gettype($wikiobj) != "array") {
            $wikiobj = json_decode($wikiobj, true);
        }

        if (empty($wikiobj["title"])) {
            $wikiobj["title"] = $purl;
        }
        $wiki = wiki(
            $purl,
            $wikiobj,
            $wikiTxt,
            $ddgObj,
            $ImpProfiles ?? null,
            $hideQueryCopy,
            $PriEcoImg
        );
    }

    if (isset($NewsObj)) {
        $news = search_news($NewsObj);
    }

    if (isset($YoutubeObj)) {
        $youtube = youtube($YoutubeObj);
    }
    if (isset($ShopObj)) {
        $shop = shop($ShopObj);
    }


    $privateAltOut = "";
    $privateAltOut = pHigh($purl);
    if ($privateAltOut != "") {
        $loaded[6] = true;
    }
    $reddit = search_reddit($redditObj);
    $relatedP = related($related);
    $elseWhere =
        '<div class="findelsewhere"><p class="float-left">Search with </p>
    <div>
    <a href="https://startpage.com/do/metasearch.pl?query=' .
        $purl .
        '"><button>Startpage</button></a>
    <a href="https://duckduckgo.com/?q=' .
        $purl .
        '"><button>DuckDuckGo</button></a>
    <a href="https://www.mojeek.com/search?q=' .
        $purl .
        '"><button>Mojeek</button></a>
        </div>
    </div>';

    //Check which addons loaded
    if ($wiki != "") {
        $loaded[1] = true;
    }
    if ($news != "") {
        $loaded[2] = true;
    }
    if ($shop != "") {
        $loaded[3] = true;
    }
    if ($reddit != "") {
        $loaded[4] = true;
    }
    if ($youtube != "") {
        $loaded[5] = true;
    }
    if ($relatedP != "") {
        $loaded[6] = true;
    }

    //Backup call and turn off API
    $apiErrors = [
        '{"error":{"code":',
        '{"response":{"status":"OK"',
        '{"status":"error",',
    ];
    $apiFailed = true;

    function disFile($name)
    {
        file_put_contents($name, time());
        return true;
    }

    while ($apiFailed) {
        $newCall = false;

        if ($searchId == 0 && strpos(json_encode($obj), $apiErrors[0]) !== false) {
            $newCall = disFile("disGoogle.txt");
            $disabled[0] = 1;
        } elseif (
            $searchId == 1 &&
            ($g2obj["status"] == "error" || empty($g2obj) || !isset($g2obj))
        ) {
            $newCall = disFile("disGoogle2.txt");
            $disabled[1] = 1;
        } elseif (
            $searchId == 2 &&
            (strpos(json_encode($QWantObj), $apiErrors[2]) !== false ||
                !isset($QWantObj["data"]["result"]["items"]["mainline"]))
        ) {
            $newCall = disFile("disBing.txt");
            $disabled[2] = 1;
        } elseif (
            $searchId == 3 &&
            (strpos(json_encode($QWantObj), $apiErrors[2]) !== false ||
                !isset($QWantObj["data"]["result"]["items"]["mainline"]))
        ) {
            $newCall = disFile("disBing2.txt");
            $disabled[3] = 1;
        }
        //Add Brave
        elseif (
            $searchId == 5 &&
            strpos(json_encode($MojeekObj), $apiErrors[1]) === false
        ) {
            $newCall = disFile("disMojeek.txt");
            $disabled[5] = 1;
        }

        if (!$newCall) {
            $apiFailed = false;
        }

        if ($newCall) {
            $returnCall = backupCall(
                $searchId,
                $disabled,
                $complex,
                $Bpurl,
                $Googlefile,
                $BraveFile,
                $MojeekFile,
                $lang,
                $page
            );
            $searchId = $returnCall[1];

            switch ($returnCall[1]) {
                case 1:
                    $g2obj = $returnCall[0];
                    break;
                case 2:
                case 3:
                    $QWantObj = $returnCall[0];
                    break;
                case 4:
                    $BraveObj = $returnCall[0];
                    break;
                case 5:
                    $MojeekObj = $returnCall[0];
                    break;
            }
        }
    }

    //Engines
    $all_urls = [];
    if (!isset($_COOKIE["index"])) {
        if ($searchId == 0) {
            $results = remote($obj, "Google");
            $all_urls = $results[1];
            $results = $results[0];
        }
        if (!isset($results[0]) && isset($g2obj)) {
            $results = remote($g2obj, "Google2");
            $all_urls = $results[1];
            $results = $results[0];
        }
        if (!isset($results[0]) && isset($QWantObj)) {
            $results = remote($QWantObj, "QWant");
            $all_urls = $results[1];
            $results = $results[0];
        }
        if (!isset($results[0]) && isset($BraveObj)) {
            $results = remote($BraveObj, "Brave");
            $all_urls = $results[1];
            $results = $results[0];
        }
        if (!isset($results[0]) && isset($MojeekObj)) {
            $results = remote($MojeekObj, "Mojeek");
            $all_urls = $results[1];
            $results = $results[0];
        }
    }


    //$questionAnswer = isset($_COOKIE["DisWid"]) ? "" : questionAnswer($purl, $results, $wikiTxt, $wikiobj);

    /*if (isset($_COOKIE["index"])) {
        $peopleAskUrl = $PriEcoObj;
    } else {
        switch ($searchId) {
            case 0:
                $peopleAskUrl = $obj;
                break;
            case 1:
                $peopleAskUrl = $g2obj;
                break;
            case 2:
            case 3:
                $peopleAskUrl = $QWantObj;
                break;
            case 4:
                $peopleAskUrl = $BraveObj;
                break;
            case 5:
                $peopleAskUrl = $MojeekObj;
                break;
            default:
                $peopleAskUrl =
                    "https://" .
                    $tmp .
                    ".wikipedia.org/wiki/" .
                    str_replace("+", "_", ucwords($wikiobj["title"]));
                break;
        }
    }*/

    if (gettype($results) == "array") {
        preg_match_all(
            "/\b(?:https?):\/\/[^\s]+/",
            implode(" ", $results),
            $matches
        );

        // Output the URLs
        /*$peopleAskUrl = [];
        foreach ($matches[0] as $url) {
            if (strpos($url, "wikipedia.org") !== false) {
                $peopleAskUrl[] = $url;
            }
        }
        $peopleAskUrl = array_filter(
            array_map(function ($url) {
                $cleanUrl = rtrim($url, '">');
                $cleanUrl = trim($cleanUrl);
                return $cleanUrl;
            }, $peopleAskUrl),
            function ($url) {
                return filter_var($url, FILTER_VALIDATE_URL) !== false;
            }
        );

        $peopleAskUrl = array_filter($peopleAskUrl, function ($peopleAskurl) {
            return preg_match(
                "~^https://[a-z]{2,}\.wikipedia\.org/~",
                $peopleAskurl
            );
        });
        ksort($peopleAskUrl);
        $peopleAskUrl = reset($peopleAskUrl);

        if (empty($peopleAskUrl) && !empty($wikiobj["title"])) {
            $peopleAskUrl =
                "https://" .
                $tmp .
                ".wikipedia.org/wiki/" .
                str_replace("+", "_", ucwords($wikiobj["title"]));
        }
        $peopleAsk = isset($_COOKIE["DisWid"]) ? "" : peopleAsk($peopleAskUrl);*/
    }
    if (!isset($_COOKIE["safe"]) && !isset($_COOKIE["time"])) {
        if (
            !isset($_SESSION[$purl . $lang . $loc . $safeS . $_COOKIE["time"]])
        ) {
            if ($obj != "") {
                $_SESSION[$purl . $lang . $loc . $safeS . $_COOKIE["time"]] =
                    "obj +=+ " . json_encode($obj);
            }
            if (isset($g2obj)) {
                $_SESSION[$purl . $lang . $loc . $safeS . $_COOKIE["time"]] =
                    "g2obj +=+ " . json_encode($g2obj);
            }
            if (isset($QWantObj)) {
                $_SESSION[$purl . $lang . $loc . $safeS . $_COOKIE["time"]] =
                    "qwant +=+ " . json_encode($QWantObj);
            }
            if (isset($BraveObj)) {
                $_SESSION[$purl . $lang . $loc . $safeS . $_COOKIE["time"]] =
                    "brave +=+ " . json_encode($BraveObj);
            }
            if (isset($MojeekObj)) {
                $_SESSION[$purl . $lang . $loc . $safeS . $_COOKIE["time"]] =
                    "mojeek +=+ " . json_encode($MojeekObj);
            }
        }
        if (isset($ShopObj)) {
            $_SESSION[$purl . ":-:shop"] = json_encode($ShopObj);
        }
        if (isset($wikiobj)) {
            $_SESSION[$purl . ":-:wiki"] =
                json_encode($wikiobj) .
                "--]|[--" .
                $wikiTxt .
                "--]|[--" .
                json_encode($ddgObj);
        }
        if (isset($NewsObj)) {
            $_SESSION[$purl . ":-:new"] = $NewsObj;
        }
        if (isset($redditObj)) {
            $_SESSION[$purl . ":-:red"] = $redditObj;
        }
        if (isset($YoutubeObj)) {
            $_SESSION[$purl . ":-:yt"] = $YoutubeObj;
        }
        if (isset($related)) {
            $_SESSION[$purl . ":-:rel"] = $related;
        }
        if (isset($PriEcoImg)) {
            $_SESSION[$purl . ":-:priimg"] = $PriEcoImg;
        }
    }
    //Print
    //Query URL
    if (
        preg_match(
            '/^(https?:\/\/)?([a-zA-Z0-9-]+\.)+[a-zA-Z0-9]{2,}(:\d{2,5})?(\/\S*)*$/',
            preg_replace('/\s+$/u', "", $purl)
        )
    ) {
        echo "<div class='queryRedirect output'>
        <p><b>Looks like you're searching for a website!</b><br>
        If you'd like us to redirect you there, simply click the button below:</p>
        <a href='";
        if (
            strpos($purl, "https://") === false &&
            strpos($purl, "http://") === false
        ) {
            echo "https://";
        }
        echo $purl, "'";
        if (isset($_COOKIE["new"])) {
            echo 'target="_blank"';
        }
        echo '><button class="redirectMe">Redirect me!</button></a>
        </div>';
    }

    if (isset($_COOKIE["index"])) {
        $priecoResults = prieco($PriEcoObj, $purl, $loc, $lang);
        $all_urls = $priecoResults[1];
        $priecoResults = $priecoResults[0];
        if (isset($LyricsObj) && !isset($LyricsObj[0]['error'])) {
        $first_genius_url = array_shift(array_values(array_filter($all_urls, function($url) {
            return strpos($url, 'genius.com') !== false;
        })));
            $lyrics = lyrics($LyricsObj, $artistObj, $first_genius_url);
        }
        echo $lyrics, $insAnswer,
            $priecoResults[0],
            $privateAltOut,
            $wiki,
            $priecoResults[1],
            $news,
            $priecoResults[2],
            $priecoResults[3],
           // $peopleAsk,
            $priecoResults[4],
            $priecoResults[5],
            $reddit,
            $priecoResults[6],
            $priecoResults[7],
            $youtube,
            $priecoResults[8],
            $relatedP;
        if (count($priecoResults) > 8) {
            for ($i = 9; $i < 20; $i++) {
                echo $priecoResults[$i];
            }
        }
        echo $elseWhere;
    } elseif (isset($results)) {
    if (isset($LyricsObj) && !isset($LyricsObj[0]['error'])) {
    $first_genius_url = array_shift(array_values(array_filter($all_urls, function($url) {
        return strpos($url, 'genius.com') !== false;
    })));
        $lyrics = lyrics($LyricsObj, $artistObj, $first_genius_url);
   }
        echo $topInfo, $lyrics,
            $insAnswer,
            //$questionAnswer["answer"],
            $results[1],
            $privateAltOut,
            $wiki,
            $results[2],
            $news,
            $results[3],
            $results[4],
            $shop,
            $results[5],
         //  $peopleAsk,
            $results[6],
            $reddit,
            $results[7],
            $results[8],
            $youtube,
            $results[9],
            $relatedP;
        if (!is_string($results) && count($results) > 9) {
            for ($i = 10; $i < count($results); $i++) {
                echo $results[$i];
            }
        }
        echo $elseWhere;
        if (!$dev) {
            if (isset($_SESSION[$purl . ":-:pri"])) {
                $priecoResults = explode("-<+>-", $_SESSION[$purl . ":-:pri"]);
            } else {
                $priecoResults = prieco($PriEcoObj, $purl, $loc, $lang);
                $all_urls = $priecoResults[1];
                $priecoResults = $priecoResults[0];
            }
            $pRes = "";
            for ($i = 0; $i < 10; $i++) {
                echo $priecoResults[$i];
                $pRes .= $priecoResults[$i] . "-<+>-";
            }
            $_SESSION[$purl . ":-:pri"] = $pRes;

        }
        //nextPage($purl,$page);
    } else {
        echo '<div class="width100V height50V flex alignC wrap justConC inv05b"><h1 class="mr-20">No results found!</h1><img src="/View/icon/no_link.svg"
    class="width100"></div>';
    }
}
if ($type === "image") {
    //Next page
    function nextPage($purl, $page, $size, $color, $type, $time, $right)
    {
        if ($page > 0) {
            echo '<div class="nextPage" style="width: 100vw;display: flex;justify-content: center;"><a href="/?image&imgsize=',
                $size,
                "&imgcolor=",
                $color,
                "&imgtype=",
                $type,
                "&imgtime=",
                $time,
                "&imglicence=",
                $right,
                "&q=",
                htmlspecialchars($purl, ENT_QUOTES | ENT_HTML5, "UTF-8"),
                "&page=",
                $page - 1,
                '"><button>⬅ Back</button></a>
            <p style="float:left;color:#8f8f8f;padding:10px;padding-top:22px;">Page: ',
                $page + 1,
                '</p>
            <a href="/?image&imgsize=',
                $size,
                "&imgcolor=",
                $color,
                "&imgtype=",
                $type,
                "&imgtime=",
                $time,
                "&imglicence=",
                $right,
                "&q=",
                htmlspecialchars($purl, ENT_QUOTES | ENT_HTML5, "UTF-8"),
                "&page=",
                $page + 1,
                '"><button>Next ⮕</button></a></div>
            <style>@media (max-width: 890px) {.nextPage {width:calc(50% + 115px);}}</style>';
        } else {
            echo '
            <div class="nextPage" style="width: 100vw;display: flex;justify-content: center;"><a href="/?image&imgsize=',
                $size,
                "&imgcolor=",
                $color,
                "&imgtype=",
                $type,
                "&imgtime=",
                $time,
                "&imglicence=",
                $right,
                "&q=",
                htmlspecialchars($purl, ENT_QUOTES | ENT_HTML5, "UTF-8"),
                "&page=",
                $page + 1,
                '"><button>Next ⮕</button></a></div>
            <style>@media (max-width: 890px) {.nextPage {width:calc(50% + 40px);}}</style>';
        }
    }

    if ($pixabay && !isset($_COOKIE["index"])) {
        echo '<form method="post" action="">
        <input type="submit" style="margin-left:9vw;"class="imgtoolsOption" value="Back" name="imgback" />
        </form>';
        $Pix = curl_init();
        curl_setopt($Pix, CURLOPT_URL, $PixabayFile);
        curl_setopt(
            $Pix,
            CURLOPT_USERAGENT,
            "Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13"
        );
        curl_setopt($Pix, CURLOPT_CONNECTTIMEOUT, 2.5);
        curl_setopt($Pix, CURLOPT_RETURNTRANSFER, true);

        $PixObj = json_decode(curl_exec($Pix), true);
        curl_close($Pix);

        echo '<div class="imgContainer"><br>
        <div>
         <div tabindex="0" class="imgoutbtn">
             <img src ="/View/img/pix.svg"class="imgout"><p style="color:black;padding-left: 10px;padding-right: 10px;">
         </div>


         <div class="bigimgout">
                 <img src ="/View/img/pix.svg">
                 <br>
                 <h3>Thank you Pixabay for providing PriEco with these images</h3><br>
                 <p>From website: https://pixabay.com/</p><br>
                 <div class="bigimgbtn"><a href="https://pixabay.com/"><button class="imgtoolsOption">Go to website</button></a><br>
                 <a href="https://pixabay.com/static/img/logo.svg"> <button class="imgtoolsOption">Go to image</button></a></div>
                 <div style="display: flex;justify-content: center;"><button class="bigimgclose imgtoolsOption">«Close</button></div>
         </div>
       </div>
         ';

        foreach ($PixObj["hits"] as &$item) {
            echo '
           <div class="imgoutdiv">
           <div tabindex="0"  class="imgoutbtn">
               <img style="max-height: 100%;" src="/Controller/functions/proxy.php?q=',
                $item["webformatURL"],
                '"class="imgout">
               </div>

           <div class="bigimgout">
           <img src ="/Controller/functions/proxy.php?q=',
                $item["largeImageURL"],
                '">
           <br>
           <h3>From user: ',
                $item["user"],
                '</h3><br>
           <p>From website: ',
                $item["largeImageURL"],
                '</p><br>
           <div class="bigimgbtn"><a href="',
                $item["pageURL"],
                '"><button class="imgtoolsOption">Go to website</button></a><br>
           <a href="',
                $item["largeImageURL"],
                '"> <button class="imgtoolsOption">Go to image</button></a></div>
           <div style="display: flex;justify-content: center;"><button class="bigimgclose imgtoolsOption">«Close</button></div>
           </div>
         </div>
             ';
        }
        echo "</div>";
        return;
    }
    if (isset($_COOKIE["index"])) {
        include "Controller/functions/engines/img/prieco.php";
    } elseif (isset($_GET["openverse"])) {
        include "Controller/functions/engines/img/openverse.php";
    } elseif (!file_exists("disBing.txt") || !file_exists("disBing2.txt") || isset($_SESSION[
                $Bpurl .
                    $page .
                    $imgsize .
                    $imgcolor .
                    $imgtype .
                    $imgtime .
                    $imgright .
                    ":-:imgBing"
            ]
        )
    ) {
        include "Model/imgset.php";
        include "Controller/functions/engines/img/bing.php";
    } else {
        include "Controller/functions/engines/img/openverse.php";
    }
}
if ($type == "video") {
    if (!$dev) {
        $curl = curl_init("http://127.0.0.1:8000/api/youtube?q=$fil_enc_que");
        curl_setopt($curl, CURLOPT_RETURNTRANSFER, true);
        $YoutubeObj = json_decode(
            str_replace("%22", '\"', curl_exec($curl)),
            true
        );
        curl_close($curl);

        if (!isset($_COOKIE["index"])) {
            if (count($YoutubeObj) < 3) {
                $indexY = false;
                unset($YoutubeObj);
            } else {
                $indexY = true;
            }
            if (!$indexY) {
                $YoutubeCurl = curl_init();
                curl_setopt($YoutubeCurl, CURLOPT_URL, $YoutubeFile);
                curl_setopt(
                    $YoutubeCurl,
                    CURLOPT_USERAGENT,
                    "Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13"
                );
                curl_setopt($YoutubeCurl, CURLOPT_CONNECTTIMEOUT, 2.5);
                curl_setopt($YoutubeCurl, CURLOPT_RETURNTRANSFER, true);
                $YoutubeObjN = json_decode(curl_exec($YoutubeCurl), true);
                curl_close($YoutubeCurl);

                foreach ($YoutubeObjN["items"] as $item) {
                    $title = substr($item["snippet"]["title"], 0, 47);
                    $description = substr(
                        $item["snippet"]["description"],
                        0,
                        47
                    );

                    $YoutubeObj[] = [
                        "title" => $title,
                        "description" => $description,
                        "url" => $item["id"]["videoId"],
                        "thumb" => str_replace(
                            "https://i.ytimg.com/vi/",
                            "",
                            $item["snippet"]["thumbnails"]["medium"]["url"]
                        ),
                        "date" => $item["snippet"]["publishTime"],
                    ];
                }
            }
        }
    } else {
        $YoutubeObj = json_decode(file_get_contents($YoutubeFile), true);
    }
    echo '<div class="imgContainer">';
    $i = 0;
    foreach ($YoutubeObj as &$item) {
        if ($i >= 75) {
            break;
        }

        echo '<div class="imgoutdiv">
                    <a href="';
        if (!isset($_COOKIE["vidURL"]) || $_COOKIE["vidURL"] == "") {
            echo "https://www.youtube.com/watch?v=";
        } else {
            echo $_COOKIE["vidURL"];
        }
        echo $item["url"] . '"';
        if (isset($_COOKIE["new"])) {
            echo 'target="_blank"';
        }
        echo '>
                    <button class="videoBtn imgoutbtn">
            <img src ="/Controller/functions/proxy.php?q=https://i.ytimg.com/vi/',
            $item["thumb"],
            '"class="imgout" loading="lazy">
            </button>
        </a>
        </div>
              ';
        ++$i;
    }
    echo "</div>";
}

if ($type == "news") {
    if (!$dev) {
        $curl = curl_init("http://127.0.0.1:8000/api/news?q=$fil_enc_que");
        curl_setopt($curl, CURLOPT_RETURNTRANSFER, true);
        $NewsObj = json_decode(
            str_replace("%22", '\"', curl_exec($curl)),
            true
        );
        curl_close($curl);

        if (!isset($_COOKIE["index"])) {
            if (count($NewsObj) < 3) {
                $indexN = false;
            } else {
                $indexN = true;
            }
            if (!$indexN) {
                $newsCh = curl_init($NewsFile);
                curl_setopt($newsCh, CURLOPT_RETURNTRANSFER, true);
                curl_setopt(
                    $newsCh,
                    CURLOPT_USERAGENT,
                    "Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13"
                );
                curl_setopt($newsCh, CURLOPT_CONNECTTIMEOUT, 2);
                curl_setopt($newsCh, CURLOPT_TIMEOUT, 2.5);
                $NewsObj = [];
                $NewsObjN = json_decode(curl_exec($newsCh), true);
                foreach ($NewsObjN["articles"] as $article) {
                    $NewsObj[] = [
                        "title" => $article["title"],
                        "description" => $article["description"],
                        "img" => $article["urlToImage"],
                        "url" => $article["url"],
                        "date" => $article["publishedAt"],
                    ];
                }
                usort($NewsObj, function ($a, $b) {
                    return strtotime($b["date"]) - strtotime($a["date"]);
                });
                curl_close($newsCh);
            }
        }
    } else {
        $NewsObj = json_decode(file_get_contents($NewsFile), true);
    }

    $i = 0;
    foreach ($NewsObj as &$news) {
        if ($i >= 30) {
            break;
        }
        $title = str_replace("<", "", $news["title"]);
        $title = str_replace(">", "", $title);
        $desc = str_replace("<", "", $news["description"]);
        $desc = str_replace(">", "", $desc);
        echo '<div class="';
        if ($i == 0) {
            echo "mBorderTop ";
        } elseif ($i == count($NewsObj) - 1 || $i == 29) {
            echo "mBorderBottom ";
        }
        echo 'output" id="output">';
        $out = false;
        if ($news["img"] != "" and !isset($_COOKIE["datasave"])) {
            $out = true;
            echo '<img loading="lazy" src="/Controller/functions/img_proxy.php?q=',
                $news["img"],
                '" class="OutSideImg">';
        }
        echo "<a ";
        if (isset($_COOKIE["new"])) {
            echo 'target="_blank"';
        }
        echo 'href="', $news["url"], '">';
        if (
            strpos($news["url"], "https://") !== false &&
            !isset($_COOKIE["datasave"])
        ) {
            echo '<img class="newsFavicon Outfavicon" loading="lazy" src="/Controller/functions/proxy.php?q=https://judicial-peach-octopus.b-cdn.net/' .
                parse_url($news["url"])["host"] .
                '">';
        }
        echo '<p class="newsTitle OutTitle">',
            $title,
            '</p></a>
    <p class="';
        if ($out) {
            echo "width100P-131 ";
        }
        echo 'newsSnippet snippet">',
            $desc,
            '</p>
    ';
        if ($providers == "on") {
            echo '<p class="resProvider">Newsapi</p>';
        }
        echo "</div>";

        ++$i;
    }
} elseif ($type == "shop") {
    $tmp = $loc == "all" ? "us" : $loc;

    $shopCh = curl_init(
        "https://api.yadore.com/v2/offer?market=" .
            $tmp .
            "&keyword=" .
            urlencode($purl) .
            "&precision=fuzzy&limit=100"
    );
    curl_setopt($shopCh, CURLOPT_RETURNTRANSFER, true);

    $headers = ["API-Key: " . $_ENV["SHOP_API_KEY"]];
    curl_setopt($shopCh, CURLOPT_HTTPHEADER, $headers);

    $ShopObj = json_decode(curl_exec($shopCh), true);

    include "Model/shopSet.php";
    echo '<div class="imgContainer">';
    foreach ($ShopObj["offers"] as &$item) {
        if (
            isset($_GET["shopMin"]) &&
            $item["price"]["amount"] < $_GET["shopMin"]
        ) {
            continue;
        }
        if (
            isset($_GET["shopMax"]) &&
            $item["price"]["amount"] > $_GET["shopMax"]
        ) {
            continue;
        }
        echo '
                 <div class="imgoutdiv">
                  <div tabindex="0" class="videoBtn imgoutbtn">
                  <a class="clearLink" href="',
            $item["clickUrl"],
            '"';
        if (isset($_COOKIE["new"])) {
            echo 'target="_blank';
        }
        echo '>
                      <img src="Controller/functions/proxy.php?q=',
            urlencode($item["thumbnail"]["url"]),
            '" class="shopout imgout">
                      <p class="shopTitle">',
            $item["title"],
            '</p>
                      <p class="shopPrice">$',
            $item["price"]["amount"],
            '</p><br>
                      <p class="shopLogo shopPrice">';
        if ($item["merchant"]["logo"]["exists"]) {
            echo '<img src="/Controller/functions/proxy.php?q=',
                $item["merchant"]["logo"]["url"],
                '">';
        }
        echo $item["merchant"]["name"],
            '</p>
                      </a>
                  </div>
                  </div>
                  ';
    }
    echo "</div>";
}
