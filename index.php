<?php
ini_set("session.cookie_httponly", 1);
ini_set("session.cookie_domain", ".jojoyou.org");
session_set_cookie_params(0, "/", "", 1, true);
session_start();
include "Controller/simple_html_dom.php";

$gTime = microtime(true);

//Development mode (Get search results from json files in ./Controller/dev folder)
$dev = false;
//CSS version
$cssver = 2010;
//Variable, controls reloading on settings change
$reload = false;

//Values
function removeDisFile($name, $time = 3600)
{
    if (file_exists($name)) {
        $tmp = file_get_contents($name);
        if($time == 10){
          if(time() > strtotime('+1 day', strtotime(date('Y-m-d 10:00:00', $saved_time)))){
            unlink($name);
          }
        }
        elseif (time() > $tmp + $time) {
            unlink($name);
        }
    }
}
$sumPath = "";

removeDisFile("disGoogle.txt", 10);
removeDisFile("disGoogle2.txt", 1440);
removeDisFile("disBing.txt", 1440);
removeDisFile("disBing2.txt", 1440);
removeDisFile("disBrave.txt");
removeDisFile("disMojeek.txt", 30);

//Get data from $PromoFile
$promoobj = json_decode(
    file_get_contents("./Controller/value/data.json"),
    true
);
//Prepare for search request with search engine APIs
include "Controller/database.php";
include "Controller/functions/func.php";
//Function to get string between characters
include "Controller/functions/getsearch.php";

//Check for img search and for empty query
if (!isset($_COOKIE["hQuery"])) {
    if (isset($_GET["image"])) {
        $type = "image";
    }
    if (isset($_GET["video"])) {
        $type = "video";
    }
    if (isset($_GET["news"])) {
        $type = "news";
    }
    if (isset($_GET["shop"])) {
        $type = "shop";
    }
}

if (isset($_GET["page"])) {
    $page = $_GET["page"];
}
if (isset($_GET["imgsize"])) {
    $imgsize = $_GET["imgsize"];
}
if (isset($_GET["imgcolor"])) {
    $imgcolor = $_GET["imgcolor"];
}
if (isset($_GET["imgtype"])) {
    $imgtype = $_GET["imgtype"];
}
if (isset($_GET["imgtime"])) {
    $imgtime = $_GET["imgtime"];
}
if (isset($_GET["imglicence"])) {
    $imgright = $_GET["imglicence"];
}
if (isset($_GET["pixabay"])) {
    $pixabay = true;
}

$page = isset($page) ? $page : 0;

if (isset($_POST["q"]) && $_POST["q"] != $purl && !isset($_COOKIE["hQuery"])) {
    header(
        "Location: ./?" .
            $type .
            (isset($_GET["tabs"]) ? "&tabs" : "") .
            (isset($_GET["tab"]) ? "&tab=" . $_GET["tab"] : "") .
            "&q=" .
            urlencode($_POST["q"])
    );
    exit();
}

#IndexLogic
include "Controller/functions/indexLogic.php";

include "Model/header.php";

##
#Protection
##
if (!$dev) {
    include "Controller/functions/protection/cookie.php";
    include "Controller/functions/protection/shield.php";
}

//Bangs
include "Controller/functions/addons/bangs.php";

//Tabs
if (isset($_GET["tabs"])) {
    include "Controller/functions/addons/tabs.php";
}

//Set parameters for search request
$lang = isset($_COOKIE["Language"]) ? $_COOKIE["Language"] : "all";
$loc = isset($_COOKIE["Location"]) ? $_COOKIE["Location"] : "all";
$msgLang = $msgLoc = false;

if (
    (!isset($_COOKIE["Location"]) || !isset($_COOKIE["Language"])) &&
    ($_SERVER["HTTP_HOST"] == "search.jojoyou.org" ||
        $_SERVER["HTTP_HOST"] == "prieco.net")
) {
    if (
        !$dev &&
        !str_starts_with((isset($_SERVER['HTTP_CF_CONNECTING_IP']) ? $_SERVER['HTTP_CF_CONNECTING_IP'] : $_SERVER['REMOTE_ADDR']), "192.") &&
        !str_starts_with((isset($_SERVER['HTTP_CF_CONNECTING_IP']) ? $_SERVER['HTTP_CF_CONNECTING_IP'] : $_SERVER['REMOTE_ADDR']), "127.")
    ) {
        $ch = curl_init();

        curl_setopt($ch, CURLOPT_RETURNTRANSFER, 1);

        curl_setopt(
            $ch,
            CURLOPT_URL,
            "http://127.0.0.1:8000/api/ip2loc?q=" .
                ip2long((isset($_SERVER['HTTP_CF_CONNECTING_IP']) ? $_SERVER['HTTP_CF_CONNECTING_IP'] : $_SERVER['REMOTE_ADDR']))
        );
        curl_setopt(
            $ch,
            CURLOPT_USERAGENT,
            "Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US) AppleWebKit/525.13 (KHTML, like Gecko) Chrome/0.A.B.C Safari/525.13"
        );

        $geo = json_decode(curl_exec($ch), true);
        if (!isset($_COOKIE["Location"])) {
            if ($geo[0]["code"] != "") {
                setcookie(
                    "Location",
                    $geo[0]["code"] . "_3",
                    time() + 604800,
                    "/",
                    null,
                    true,
                    true
                );
            } else {
                setcookie(
                    "Location",
                    "all",
                    time() + 604800,
                    "/",
                    null,
                    true,
                    true
                );
            }
        }
        if (!isset($_COOKIE["Language"])) {
            if ($geo[0]["lang"] != "") {
                setcookie(
                    "Language",
                    $geo[0]["lang"] . "_3",
                    time() + 604800,
                    "/",
                    null,
                    true,
                    true
                );
            } else {
                setcookie(
                    "Language",
                    "all",
                    time() + 604800,
                    "/",
                    null,
                    true,
                    true
                );
            }
        }

        $reload = true;
    } else {
        setcookie("Language", "all", time() + 604800, "/", null, true, true);
        setcookie("Location", "all", time() + 604800, "/", null, true, true);
        $reload = true;
    }
}

if (strpos($lang, "_") !== false) {
    $lang = explode("_", $lang);
    $langNum = $lang[1] - 1;
    $lang = $lang[0];
    if ($langNum == 0) {
        setcookie("Language", $lang, time() + 604800, "/", null, true, true);
    } else {
        setcookie(
            "Language",
            $lang . "_" . $langNum,
            time() + 604800,
            "/",
            null,
            true,
            true
        );
    }
    $msgLang = true;
}
if (strpos($loc, "_") !== false) {
    $loc = explode("_", $loc);
    $locNum = $loc[1] - 1;
    $loc = $loc[0];
    if ($locNum == 0) {
        setcookie("Location", $loc, time() + 604800, "/", null, true, true);
    } else {
        setcookie(
            "Location",
            $loc . "_" . $locNum,
            time() + 604800,
            "/",
            null,
            true,
            true
        );
    }

    $msgLoc = true;
}

if ($msgLang || $msgLoc) {
    echo '<div class="revertLocLang">
        <p>Location and language have been set. </p>
        <form method="post">
        <input type="submit" name="revetToGlobal" value="Revert to global" class="white">
        </form>
      </div>';
}

$fahrenheitCountries = [
    "as",
    "bs",
    "bz",
    "ca",
    "fm",
    "gu",
    "mh",
    "mp",
    "pr",
    "pw",
    "tc",
    "us",
    "vi",
];
if (
    !isset($_COOKIE["temp"]) &&
    in_array($_COOKIE["Location"], $fahrenheitCountries)
) {
    setcookie("temp", "f", time() + 604800, "/", null, true, true);
}

if (isset($_COOKIE["safe"])) {
    $safe = $_COOKIE["safe"];
} else {
    $safe = "active";
}

if (isset($_COOKIE["new"])) {
    $new = $_COOKIE["new"];
} else {
    $new = "off";
}

if (!isset($imgsize)) {
    $imgsize = "all";
}
if (!isset($imgcolor)) {
    $imgcolor = "all";
}
if (!isset($imgtime)) {
    $imgtime = "all";
}
if (!isset($imgtype)) {
    $imgtype = "all";
}
if (!isset($imgright)) {
    $imgright = "all";
}

if (isset($_COOKIE["time"])) {
    switch ($_COOKIE["time"]):
        case "day":
            $date = "d1";
            break;
        case "week":
            $date = "w1";
            break;
        case "month":
            $date = "m1";
            break;
        case "year":
            $date = "y1";
            break;
    endswitch;
} else {
    $date = "";
}
//Get style.css file and post custom settings for light and dark mode
$beforePathStyle = "";
include "Model/style.php";
##
#Analytics
##
//if(!$dev){include('Controller/functions/analytics/analytics.php');}

if ($purl != null or $purl != "") {
    include "Controller/retrieve.php";

    //Create inputbox with settings
    include "Model/searchbox.php";
    //Print output
    $priecoTime = -1;
    $resultTime = microtime(true);
    include "Controller/functions/output.php";
    $resultTime = microtime(true) - $resultTime;

    //Improve PriEco
    if (
        !$dev &&
        !isset($_COOKIE["index"]) &&
        isset($_COOKIE["telemetry"]) &&
        $_COOKIE["telemetry"] == "on"
    ) {
        include "Controller/functions/analytics/improve.php";
    }

    if (isset($_COOKIE["showtime"])) {
        echo '<div class="showtime">
    <h4>Time to load:</h4>
    <p>Full time: ',
            microtime(true) - $gTime,
            ' s<br>
    Together results time: ',
            $resultTime,
            ' s<br>
    PriEco results time: ',
            $priecoTime,
            ' s
    </p></div>';
    }

    //Increase daily searches
    if (!$dev) {
        $currentDate = date("Y-m-d");
        $jsonFilePath = "count.json";

        if (file_exists($jsonFilePath)) {
            $jsonContent = file_get_contents($jsonFilePath);
            $usersData = json_decode($jsonContent, true);
        } else {
            $usersData = [];
        }

        $found = false;
        foreach ($usersData as &$user) {
            if ($user["day"] === $currentDate) {
                $user["searches"]++;
                $found = true;
                break;
            }
        }
        if (!$found) {
            $usersData[] = [
                "day" => $currentDate,
                "searches" => 1,
            ];
        }
        $newJsonContent = json_encode($usersData, JSON_PRETTY_PRINT);
        file_put_contents($jsonFilePath, $newJsonContent);
    }
} else {
    //Print mainpage
    include "Model/mainpage.php";
}

include "Model/footer.php";
?>
