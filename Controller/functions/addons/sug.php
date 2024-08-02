<?php
if (!isset($_GET["t"]) || !isset($_GET["q"]) || !isset($_GET["c"])) {
    exit();
}

$purl = filter_input(INPUT_GET, "q", FILTER_SANITIZE_STRING);

if ($_GET["t"] == "p") {
    $ch = curl_init(
        "http://127.0.0.1:8000/api/sug/?q=" . urlencode($purl) . "&lang=all"
    );
    curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);
    curl_setopt($ch, CURLOPT_SSL_VERIFYPEER, false);
    curl_setopt($ch, CURLOPT_SSL_VERIFYHOST, false);
    $rows = json_decode(curl_exec($ch), true);
    if (curl_errno($ch)) {
        echo "cURL error: " . curl_error($ch);
        exit();
    }
    curl_close($ch);


    $suggestions = [];
    foreach ($rows as $row) {
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
    $sortedSuggestions = [];
    foreach ($topSuggestions as $suggestion) {
        foreach ($suggestions as $item) {
            if ($item["name"] === $suggestion) {
                $sortedSuggestions[] = $item;
                break;
            }
        }
    }

    $response = [
        0 => $purl,
        1 => $sortedSuggestions,
    ];
} elseif ($_GET["t"] == "d") {
    $purl = urlencode($purl);
    $ch = curl_init(
        "https://ac.duckduckgo.com/ac/?type=list&callback=jsonCallback&_=1600956892202&q=" .
            $purl
    );
    curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);
    curl_setopt(
        $ch,
        CURLOPT_USERAGENT,
        "Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13"
    );
    curl_setopt($ch, CURLOPT_TIMEOUT, 3);
    $response = json_decode(curl_exec($ch), true);
    curl_close($ch);
} elseif ($_GET["t"] == "g") {
    $purl = urlencode($purl);
    $ch = curl_init(
        "https://www.google.com/complete/search?client=firefox&q=" . $purl
    );
    curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);
    curl_setopt(
        $ch,
        CURLOPT_USERAGENT,
        "Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13"
    );
    curl_setopt($ch, CURLOPT_TIMEOUT, 3);
    $response = json_decode(curl_exec($ch), true);
    unset($response[2]);
    unset($response[3]);
    curl_close($ch);
}
/*if (strlen($purl) >= 3 && $_GET["c"] != "all") {
    if ($_GET["t"] != "p") {
        $dev = true;
        include "../../database.php";
    } else {
        $purl = urlencode($_GET["q"]);
    }
    $ch = curl_init(
        "https://" .
            $_ENV["SITEPLUG_CUSTOMER_KEY"] .
            ".siteplug.com/sssapi?o=" .
            $_ENV["SITEPLUG_CUSTOMER_KEY"] .
            "&s=" .
            $_ENV["SITEPLUG_SITE_ID"] .
            "&kw=" .
            $purl .
            "&itype=ss&f=json&i=1&it=1&is=36x36&cc=" .
            $_GET["c"] .
            "&n=2&af=0&subid=PriEco"
    );
    curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);
    curl_setopt(
        $ch,
        CURLOPT_USERAGENT,
        "Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13"
    );
    curl_setopt($ch, CURLOPT_TIMEOUT, 3);
    $sponsored = json_decode(curl_exec($ch), true);
    if ($sponsored != null) {
        $sponsored[0] = $sponsored["ads"];
        unset($sponsored["ads"]);
        $response = array_merge_recursive($sponsored, $response);
    }
    curl_close($ch);
}*/
header("Content-Type: application/json");
echo json_encode($response);
