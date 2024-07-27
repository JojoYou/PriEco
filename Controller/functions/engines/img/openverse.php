<?php
$imgUrl = "";
if (!file_exists("disOpenVerse.txt")) {
    $imgUrl =
        "https://api.openverse.org/v1/images/?format=json&page_size=50&q=" .
        $Bpurl;
}

//Request new token
openverseToken();

if (!isset($_SESSION[$Bpurl . ":-:imgOpen"])) {
    $qCh = curl_init($imgUrl);
    curl_setopt(
        $qCh,
        CURLOPT_USERAGENT,
        "Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13"
    );
    curl_setopt($qCh, CURLOPT_CONNECTTIMEOUT, 2.5);
    curl_setopt($qCh, CURLOPT_RETURNTRANSFER, true);
    curl_setopt($qCh, CURLOPT_HTTPHEADER, [
        "Authorization: Bearer " . explode(";", $_ENV["OPENVERSE"])[0],
    ]);

    $openVerse = json_decode(curl_exec($qCh), true);
    if (isset($openVerse["results"])) {
        $_SESSION[$Bpurl . ":-:imgOpen"] = json_encode($openVerse);
    }
    curl_close($qCh);
} else {
    $openVerse = json_decode($_SESSION[$Bpurl . ":-:imgOpen"], true);
}

echo '<div class="imgContainer">';
foreach ($openVerse["results"] as &$item) {
    $domain = str_replace("www.", "", parse_url($item["url"])["host"]);
    echo '
           <div class="imgoutdiv">
           <div tabindex="0" class="imgoutbtn">
              <img src="Controller/functions/proxy.php?q=',
        $item["thumbnail"],
        '" class="imgout">
       <a class="imgoutTxt link" href="' .
            $item["url"] .
            '"';
    if (isset($_COOKIE["new"])) {
        echo 'target="_blank';
    }
    echo '>
<img class="Outfavicon curve height20" alt="&lrm;" loading="lazy" src="/Controller/functions/proxy.php?q=https://judicial-peach-octopus.b-cdn.net/' .
    $domain .
    '">
<p class="colorWhite">' .
    $domain .
    '</p>
</a>
            </div>



            <div class="bigimgout">
            <img src ="Controller/functions/proxy.php?q=',
        $item["thumbnail"],
        '" data-src="/Controller/functions/proxy.php?q=',
        $item["url"],
        '"';
    if (!isset($_COOKIE["DisHImg"])) {
        echo 'class="blur-5"';
    }
    echo '>
            <br>
            <h3>',
        $item["title"],
        '</h3><br>
            <p>From website: ',
        $item["url"],
        '</p><br>
            <div class="bigimgbtn"><a href="',
        $item["url"],
        '"><button class="imgtoolsOption">Go to website</button></a><br>
            <a href="',
        $item["url"],
        '"> <button class="imgtoolsOption">Go to image</button></a></div>
            <button class="mt-20 imgtoolsOption">Close</button>
            </div>
            </div>
            ';
}
echo "</div></div>";

if (!isset($_COOKIE["DisHImg"])) {
    echo '<script src="View/js/highImg.js"></script>';
}
