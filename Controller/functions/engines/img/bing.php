<?php
if (!$dev) {
    function imgCall($Bpurl,$page,$imgsize,$imgcolor,$imgtype,$imgtime,$imgright) {
      $united_url_part= $page * 82 ."&uiv=1&locale=en_US&size=" .$imgsize . "&color=" .   $imgcolor . "&imagetype=" . $imgtype . "&freshness=" . $imgtime . "&license=" . $imgright . "&q=";
        $Qimg[0] = '{"status":"error","data":{"error_code":24}}';

        $apis = [];
        if (!file_exists("disBing.txt")) {
            $apis[] = 'https://api.qwant.com/v3/search/images/?count=75&offset=' .$united_url_part.$Bpurl;
        }
        if (!file_exists("disBing2.txt")) {
            $apis[] = 'https://obunic.net/tests/prieco/?s=b&img=true&api='. $_ENV['Index2'] .'&page='. $united_url_part . $Bpurl;
        }
        if(!file_exists('disBing3.txt')){
          $apis[] = 'https://prieco.jojoyou.org/?s=b&img=true&api=' . $_ENV['Index2'] . '&q=' . $Bpurl;
        }
        if(!file_exists('disKarma.txt')){
          $apis[] = 'https://api.karmasearch.org/search/images?adultFilter=moderate&market=en-US&userLanguage=en&country=US&pageNumber=1&searchTerm=' . $Bpurl;
        }

        $randomUserAgent = generateRandomUserAgent();

        // Initialize cURL session
        $imgUrl = $apis[array_rand($apis)];

        $qCh = curl_init($imgUrl);

        curl_setopt($qCh, CURLOPT_RETURNTRANSFER, true);
        if(strpos($imgUrl,'karmasearch.org') === false && strpos($imgUrl,'prieco.jojoyou.org') === false){
          curl_setopt($qCh, CURLOPT_HTTPHEADER, [
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
                  ]);
        }

        $qResponse = curl_exec($qCh);

        curl_close($qCh);

        $Qimg[0] = $qResponse;
        $Qimg[1] = $imgUrl;

        return $Qimg;
    }

    if(isset($_SESSION[$Bpurl.$page.$imgsize.$imgcolor.$imgtype.$imgtime.$imgright.":-:imgBing"])){
      include "Model/imgset.php";
      $Qimg=$_SESSION[$Bpurl.$page.$imgsize.$imgcolor.$imgtype.$imgtime.$imgright.":-:imgBing"];
      $Qimg = json_decode($Qimg, true);
      print_bing($Qimg);
    }
    else if(isset($_SESSION[$Bpurl.$page.$imgsize.$imgcolor.$imgtype.$imgtime.$imgright.":-:imgKarma"])){
      $Qimg=$_SESSION[$Bpurl.$page.$imgsize.$imgcolor.$imgtype.$imgtime.$imgright.":-:imgKarma"];
      $Qimg = json_decode($Qimg, true);
      print_karma($Qimg);
    }
    else {
      $Qimg=imgCall($Bpurl,$page,$imgsize,$imgcolor,$imgtype,$imgtime,$imgright);
      if(strpos(($Qimg[1]), "karmasearch.org") !== false){
        $_SESSION[ $Bpurl . $page . $imgsize . $imgcolor . $imgtype . $imgtime . $imgright . ":-:imgKarma" ] = $Qimg[0];
        $Qimg = json_decode($Qimg[0], true);
        print_karma($Qimg);
      }
      else{
        include "Model/imgset.php";
        $_SESSION[ $Bpurl . $page . $imgsize . $imgcolor . $imgtype . $imgtime . $imgright . ":-:imgBing" ] = $Qimg[0];
        $Qimg = json_decode($Qimg[0], true);
        print_bing($Qimg);
	}
    }
  }
  else {
    $Qimg = file_get_contents("./Controller/dev/img.json");
    print_bing($Qimg);
  }

if (!isset($_COOKIE["DisHImg"])) {
    echo '<script src="View/js/highImg.js"></script>';
}
//echo nextPage($purl, $page, $imgsize, $imgcolor,$imgtype, $imgtime, $imgright);
function generateRandomUserAgent() {
    $browsers = ['Chrome', 'Firefox', 'Safari', 'Edge', 'Opera'];
    $os = ['Windows NT 10.0', 'Macintosh; Intel Mac OS X 10_15_7', 'Linux; Android 10', 'iPhone; CPU iPhone OS 14_0 like Mac OS X'];
    $browser = $browsers[array_rand($browsers)];
    $operatingSystem = $os[array_rand($os)];

    $version = rand(70, 100) . '.' . rand(0, 100) . '.' . rand(0, 5000);

    return "Mozilla/5.0 ($operatingSystem) AppleWebKit/537.36 (KHTML, like Gecko) $browser/$version Safari/537.36";
}

function print_bing($Qimg){
  echo '<div class="imgContainer">';
  foreach ($Qimg["data"]["result"]["items"] as &$item) {
    if (!isset($item["media"]) or !isset($item["media_preview"])) {
          continue;
    }

    $domain = str_replace("www.", "", parse_url($item["url"])["host"]);
    echo '<div class="imgoutdiv">
             <div tabindex="0" class="imgoutbtn">
                <img src="Controller/functions/proxy.php?q=http', urldecode(str_replace("&q=0&b=1&p=0&a=0","",explode("?u=http", $item["thumbnail"])[1])),'" class="imgout">
                <a class="imgoutTxt link" href="',$item["url"],'"';if (isset($_COOKIE["new"])) {echo 'target="_blank';}echo '>
                  <img class="Outfavicon curve height20" alt="&lrm;" loading="lazy" src="/Controller/functions/proxy.php?q=https://judicial-peach-octopus.b-cdn.net/' .$domain .'">
                  <p class="colorWhite">' .$domain .'</p>
                </a>
              </div>

            <div class="bigimgout">
              <img src ="Controller/functions/proxy.php?q=http',urldecode(str_replace("&q=0&b=1&p=0&a=0","",explode("?u=http", $item["thumbnail"])[1])),'" data-src="/Controller/functions/proxy.php?q=',$item["media"],'"';if (!isset($_COOKIE["DisHImg"])) {echo 'class="blur-5"';}echo '>
              <br>
              <h3>',$item["title"],'</h3><br>
              <p>From website: ',$item["url"],'</p><br>
              <div class="bigimgbtn"><a href="',$item["url"],'">
                <button class="imgtoolsOption">Go to website</button></a><br>
                <a href="',$item["media"],'"> <button class="imgtoolsOption">Go to image</button></a>
              </div>
              <button class="mobile-visible mt-20 imgtoolsOption">Close</button>
              </div>
              </div>
              ';
}
echo "</div></div>";
}

function print_karma($Qimg){
  echo '<div class="imgContainer">';
  foreach ($Qimg['results'] as &$item) {
    $domain = str_replace("www.", "", parse_url($item['url'])['host']);
    echo '<div class="imgoutdiv">
             <div tabindex="0" class="imgoutbtn">
                <img src="Controller/functions/proxy.php?q=', urldecode($item['thumbnail']['src']),'" class="imgout">
                <a class="imgoutTxt link" href="',$item["url"],'"';if (isset($_COOKIE["new"])) {echo 'target="_blank';}echo '>
                  <img class="Outfavicon curve height20" alt="&lrm;" loading="lazy" src="/Controller/functions/proxy.php?q=https://judicial-peach-octopus.b-cdn.net/' .$domain .'">
                  <p class="colorWhite">' .$domain .'</p>
                </a>
              </div>

            <div class="bigimgout">
              <img src ="Controller/functions/proxy.php?q=',urldecode($item['thumbnail']['src']),'" data-src="/Controller/functions/proxy.php?q=',urldecode($item['properties']['url']),'"';if (!isset($_COOKIE["DisHImg"])) {echo 'class="blur-5"';}echo '>
              <br>
              <h3>',$item["title"],'</h3><br>
              <p>From website: ',$item["url"],'</p><br>
              <div class="bigimgbtn"><a href="',$item["url"],'">
                <button class="imgtoolsOption">Go to website</button></a><br>
                <a href="',$item['properties']['url'],'"> <button class="imgtoolsOption">Go to image</button></a>
              </div>
              <button class="mobile-visible mt-20 imgtoolsOption">Close</button>
              </div>
              </div>
              ';
}
echo "</div></div>";
}
