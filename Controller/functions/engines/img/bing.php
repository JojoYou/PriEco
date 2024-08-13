<?php
if (!$dev) {
    function imgCall(
        $Bpurl,
        $page,
        $imgsize,
        $imgcolor,
        $imgtype,
        $imgtime,
        $imgright
    ) {
        $Qimg[0] = '{"status":"error","data":{"error_code":24}}';

        $apis = [];
        if (!file_exists("disBing.txt")) {
            $apis[] =
                "https://api.qwant.com/v3/search/images/?count=75&offset=" .
                $page * 82 .
                "&uiv=1&locale=en_US&size=" .
                $imgsize .
                "&color=" .
                $imgcolor .
                "&imagetype=" .
                $imgtype .
                "&freshness=" .
                $imgtime .
                "&license=" .
                $imgright .
                "&q=" .
                $Bpurl;
        }
        if (!file_exists("disBing2.txt")) {
            $apis[] =
                "https://obunic.net/tests/prieco/?s=b&img=true&api=". $_ENV['Index2'] .'&page='.
                $page * 82 .
                "&uiv=1&locale=en_US&imgsize=" .
                $imgsize .
                "&imgcolor=" .
                $imgcolor .
                "&imgtype=" .
                $imgtype .
                "&imgtime=" .
                $imgtime .
                "&imgright=" .
                $imgright .
                "&q=" .
                $Bpurl;
        }

        $imgUrl = "";
        if (count($apis) > 1) {
            $imgUrl = $apis[array_rand($apis)];
        } else {
            $imgUrl = $apis[0];
        }

        $qCh = curl_init();
        curl_setopt($qCh, CURLOPT_URL, $imgUrl);
        curl_setopt(
            $qCh,
            CURLOPT_USERAGENT,
            "Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13"
        );
        curl_setopt($qCh, CURLOPT_CONNECTTIMEOUT, 2.5);
        curl_setopt($qCh, CURLOPT_RETURNTRANSFER, true);

        $qResponse = curl_exec($qCh);

        curl_close($qCh);

        $Qimg[0] = $qResponse;
        $Qimg[1] = $imgUrl;

        return $Qimg;
    }

    if (
        !isset(
            $_SESSION[
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
        $Qimg = imgCall(
            $Bpurl,
            $page,
            $imgsize,
            $imgcolor,
            $imgtype,
            $imgtime,
            $imgright
        );
    } else {
        $Qimg =
            $_SESSION[
                $Bpurl .
                    $page .
                    $imgsize .
                    $imgcolor .
                    $imgtype .
                    $imgtime .
                    $imgright .
                    ":-:imgBing"
            ];
    }

    if (
        $Qimg[0] == '{"status":"error","data":{"error_code":24}}' ||
        $Qimg[0] == '{"status":"error","data":{"error_code":20}}' ||
        $Qimg[0] == '{"statusCode":500,"message":"Internal server error"}' ||
        !isset($Qimg[0])
    ) {
        if (
            !file_exists("disBing.txt") &&
            parse_url($Qimg[1])["host"] == "api.qwant.com"
        ) {
            file_put_contents("disBing.txt", time());
        }

        $Qimg = imgCall(
            $Bpurl,
            $page,
            $imgsize,
            $imgcolor,
            $imgtype,
            $imgtime,
            $imgright
        );
    }
} else {
    $Qimg = file_get_contents("./Controller/dev/img.json");
}
if (
    !isset(
        $_SESSION[
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
    $_SESSION[
        $Bpurl .
            $page .
            $imgsize .
            $imgcolor .
            $imgtype .
            $imgtime .
            $imgright .
            ":-:imgBing"
    ] = $Qimg[0];
    $Qimg = json_decode($Qimg[0], true);
} else {
    $Qimg = json_decode($Qimg, true);
}

echo '<div class="imgContainer">';
foreach ($Qimg["data"]["result"]["items"] as &$item) {
    if (!isset($item["media"]) or !isset($item["media_preview"])) {
        continue;
    }

    $domain = str_replace("www.", "", parse_url($item["url"])["host"]);
    echo '
           <div class="imgoutdiv">
           <div tabindex="0" class="imgoutbtn">
                <img src="Controller/functions/proxy.php?q=http',
        urldecode(
            str_replace(
                "&q=0&b=1&p=0&a=0",
                "",
                explode("?u=http", $item["thumbnail"])[1]
            )
        ),
        '" class="imgout">
                <a class="imgoutTxt link" href="',
        $item["url"],
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
            <img src ="Controller/functions/proxy.php?q=http',
        urldecode(
            str_replace(
                "&q=0&b=1&p=0&a=0",
                "",
                explode("?u=http", $item["thumbnail"])[1]
            )
        ),
        '" data-src="/Controller/functions/proxy.php?q=',
        $item["media"],
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
        $item["media"],
        '"> <button class="imgtoolsOption">Go to image</button></a></div>
            <button class="mobile-visible mt-20 imgtoolsOption">Close</button>
            </div>
            </div>
            ';
}
echo "</div></div>";

if (!isset($_COOKIE["DisHImg"])) {
    echo '<script src="View/js/highImg.js"></script>';
}
//echo nextPage($purl, $page, $imgsize, $imgcolor,$imgtype, $imgtime, $imgright);
