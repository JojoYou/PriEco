<?php
//Get data from $PromoFile
$promodata = file_get_contents('./Controller/value/data.json');
$promoobj = json_decode($promodata, true);
$langName = [""];
$langVal = [""];
foreach ($promoobj['lang'][0] as $location) {
  array_push($langVal, $location);
}
foreach ($promoobj['lang'][0] as $name => $value) {
  array_push($langName, $name);
}

include 'settings.php';
?>

<div class="searchForm">
  <div class="landSearchBar">

    <img id="LandSLogo" alt="PriEcoLogo" src="./View/img/PriEco.webp" />
    <span class="LandSTitle">PriEco</span>

    <form id="searchForm" class="searchBarMain" method="post">
      <button id="searchButton" aria-label="search button" class="searchButton">
        <img src="./View/icon/search.webp" class="searchIcon" alt="Search icon"></button>
      <input list="suggestions" id="searchBox" placeholder="Search privately and ecologically" value="" class="searchBox"
        name="q" size="21" autofocus="" autocomplete="off"><button class="delQueryBtn" aria-label="Clear query button"
        type="button"><img class="delQueryIcon" src="View/icon/cross.svg" alt="Delete query button"></button>
    </form>
    <div class="autocom-box"></div>

    <div class="shortcuts">
      <?php
      $i = 0;
      /*if (($_SERVER['HTTP_HOST'] == 'search.jojoyou.org' || $_SERVER['HTTP_HOST'] == 'prieco.net') && $loc != 'all') {
        $curl_handle = curl_init();
        curl_setopt($curl_handle, CURLOPT_URL, 'https://' . $_ENV['VEVE_CUSTOMER_KEY'] . '.veve.com/qlapi?o=' . $_ENV['VEVE_CUSTOMER_KEY'] . '&s=' . $_ENV['VEVE_SITE_ID'] . '&u=search.jojoyou.org&itype=ss&f=json&i=1&is=36x36&ist=3&cc=' . $loc);
        curl_setopt($curl_handle, CURLOPT_CONNECTTIMEOUT, 2);
        curl_setopt($curl_handle, CURLOPT_RETURNTRANSFER, 1);
        $sponsored = json_decode(curl_exec($curl_handle), true);
        curl_close($curl_handle);
        if (!empty($sponsored)) {
          foreach ($sponsored['data'] as &$sp) {
            if ($i >= 5) {
              break;
            }
            echo '<div>
      <div>
      <div class="shortcutEditBtn"><img class="width15 filterImage" alt="ad" src="View/icon/ad1.svg"></div>
      <a href="', $sp['rurl'], '">
      <div tabindex="0" class="shortcutElemenet">
        <img src="/Controller/functions/proxy.php?q=' . $sp['iurl'] . '" alt="', $sp['brand'], '">
        <img src="/Controller/functions/proxy.php?q=' . $sp['impurl'] . '" class="none">
      </div>
      </a>
      <div>
      <p class="txt14">';
            if (strlen($sp['brand']) > 10) {
              echo substr($sp['brand'], 0, 7) . '...';
            } else {
              echo $sp['brand'];
            }
            echo '</p>
      </div>
      </div>
      </div>
      ';
            ++$i;
          }
        }
      }*/
      $i = 0;
      if (isset($_COOKIE['shortcuts'])) {
        $shortcuts = explode(',', urldecode($_COOKIE['shortcuts']));
        foreach ($shortcuts as &$shs) {
          ++$i;
          if ($i == 1) {
            continue;
          }
          $sh = explode('=', $shs);
          echo '<div>
<div>
<div tabindex="0" class="shortcutEditBtn', $i, ' shortcutEditBtn">•</div>
<a href="', $sh[1], '" class="shortcutBtn', $i, '">
<div tabindex="0" class="shortcutElemenet">
  <img src="/Controller/functions/proxy.php?q=https://judicial-peach-octopus.b-cdn.net/' . parse_url($sh[1])['host'] . '">
</div>
</a>
<div class="shortcutEditForm', $i, ' shortcutEditForm shortcutForm">
<form method="post">
<input type="hidden" name="shortcutID" value="', $i, '">
<input type="submit" name="shortcutDelete" value="Delete" class="redBtn DelShortcut">
</form>
<p><b>Edit</b></p>
<p>', $sh[0], '</p>
<form method="post">
<input type="hidden" name="shortcutID" value="', $i, '">
<input type="text" name="shortcutName" placeholder="Name" value="', $sh[0], '" required>
<input type="text" name="shortcutURL" placeholder="URL" value="', $sh[1], '" required>
<input type="submit" name="shortcutEdit" class="Pointer" value="Edit">
</form>
</div>
<div>
<p class="txt14">';
          if (strlen($sh[0]) > 10) {
            echo substr($sh[0], 0, 7) . '...';
          } else {
            echo $sh[0];
          }
          echo '</p>
</div>
</div>
</div>
';

        }
      }
      echo '<div tabindex="0" id="addShortcutBtn" class="shortcutElemenet">+</div>
<div class="addShortcut shortcutForm">
<p><b>Add shortcut</b></p>
<form method="POST">
<input type="text" name="shortcutName" placeholder="Name" required>
<input type="text" name="shortcutURL" placeholder="URL" required>
<input type="submit" name="shortcutSubmit" value="Add" class="Pointer">
</form>
</div>
  </div>
</div>
</div>

  <div class="landBG"></div>';

      for ($j = 0; $j < ($i - 2) / 4; $j++) {
        echo '<div class="mainSpace"></div>';
      }
if(!isset($_COOKIE['hideDesc'])){
      echo '<a href="#scroll">
        <div class="flex flexDColumn alignC">
          <img class="filterImage width40 floatAnim absolute bottom15" src="View/icon/arrow-down.png" alt="Arrow down">
        </div>
      </a>

      <div id="scroll" class="flex wrap justContSpace-Between alignC">
        <div>
          <h1 class="txt60">Hey 👋</h1>
          <div>
            <li class="padding4 mt-20">PriEco never tracks you unlike your currect search engine.</li>
            <li class="padding4">PriEco is open source with the AGPL v3 license.</li>
            <li class="padding4">PriEco is powered by all major indexes including its own.</li>
            <li class="padding4">We maximize our ecofriendliness as much as we can!</li>
            <li class="padding4">We are improving the users privacy daily!</li>
          </div>
        </div>
        <img class="max-width500 width100P floatAnim" src="View/img/people_on_mini_cloud_transparent.svg">
      </div>


      <div class="comparison" title="Last updated on May.3 in 2024">
        <table class="comparison-table">
          <thead>
            <tr>
              <th>&nbsp;</th>
              <th>PriEco</th>
              <th>Startpage</th>
              <th>DuckDuckGo</th>
              <th>Google</th>
              <th>Bing</th>
            </tr>
          </thead>
          <tbody>
            <tr class="observatory">
              <td><a target="_blank" href="https://observatory.mozilla.org/"><img class="filterImage" src="View/img/observatory.svg"></a></td>
              <td><a target="_blank" href="https://observatory.mozilla.org/analyze/prieco.net"><img src="View/img/grades/a_plus_rating.svg"></a></td>
              <td><a target="_blank" href="https://observatory.mozilla.org/analyze/www.startpage.com"><img src="View/img/grades/b_rating.svg"></a></td>
              <td><a target="_blank" href="https://observatory.mozilla.org/analyze/duckduckgo.com"><img src="View/img/grades/b_rating.svg"></a></td>
              <td><a target="_blank" href="https://observatory.mozilla.org/analyze/www.google.com"><img src="View/img/grades/b_minus_rating.webp"></a></td>
              <td><a target="_blank" href="https://observatory.mozilla.org/analyze/www.bing.com"><img src="View/img/grades/c_minus_rating.webp"></a></td>
            </tr>
            <tr class="internetnl">
              <td><a target="_blank" href="https://internet.nl/"><img src="View/img/nl.webp"></a></td>
              <td><a class="clearLink" target="_blank" href="https://internet.nl/site/prieco.net/2768137/">100%</a></td>
              <td><a class="clearLink" target="_blank" href="https://internet.nl/site/www.startpage.com/2768151/">79%</a></td>
              <td><a class="clearLink" target="_blank" href="https://internet.nl/site/duckduckgo.com/2768161/">52%</a></td>
              <td><a class="clearLink" target="_blank" href="https://internet.nl/site/www.google.com/2768167/">65%</a></td>
              <td><a class="clearLink" target="_blank" href="https://internet.nl/site/www.bing.com/2768165/">70%</a></td>
            </tr>
            <tr class="tosdr">
            <td><a target="_blank" href="https://tosdr.org/"></a></td>
              <td><a target="_blank" href="https://tosdr.org/en/service/9994"><img src="View/img/pC.svg"></a></td>
              <td><a target="_blank" href="https://tosdr.org/en/service/418"><img src="View/img/sA.svg"></a></td>
              <td><a target="_blank" href="https://tosdr.org/en/service/222"><img src="View/img/dB.svg"></a></td>
              <td><a target="_blank" href="https://tosdr.org/en/service/217"><img src="View/img/gE.svg"></a></td>
              <td><a target="_blank" href="https://tosdr.org/en/service/331"><img src="View/img/bE.svg"></a></td>
            </tr>
          </tbody>
        </table>
      </div>


      <div class="followup">
        <div>
          <input type="checkbox" id="question1" name="q" class="questions none">
          <div class="plus">+</div>
          <label for="question1" class="question">
            😷Private |🔒Secure
          </label>
          <div class="answers">
            • We have no idea who you are.
            <br>
            • We never track you, log your IP address, share nor sell your data.
            <br>
            • No data, nothing to worry about.
            <br>
            • PriEco uses top tier SSL, hashing and is suitable for modern internet.
          </div>
        </div>

        <div>
          <input type="checkbox" id="question2" name="q" class="questions none">
          <div class="plus">+</div>
          <label for="question2" class="question">
            🌲Eco-friendly
          </label>
          <div class="answers">
            Ecology is one of the 2 essential pillars PriEco stands on.<br><br>
            • Our software is very efficient.<br>
            • PriEco is 100% powered by energy from renewable sources.<br>
            • We are on our way to start donating a portion of our profits to charities.
          </div>
        </div>

        <div>
          <input type="checkbox" id="question3" name="q" class="questions none">
          <div class="plus">+</div>
          <label for="question3" class="question">
            📖Open source | 💰Free
          </label>
          <div class="answers">
            • We do not hide our code.
            <br>
            • You can see and contribute to our <a class="link" href="https://codeberg.org/JojoYou/PriEco/src/branch/master" target="_blank">source code</a>.
            <br>
            • PriEco is licensed under <a class="link" href="https://codeberg.org/JojoYou/PriEco/src/branch/master/LICENCE.md" target="_blank">AGPL v3 license</a>
            <br>
            • PriEco has been free and accessible to anyone from day 1.
          </div>
        </div>

        <div>
          <input type="checkbox" id="question4" name="q" class="questions none">
          <div class="plus">+</div>
          <label for="question4" class="question">
            😄 Mind donating?
          </label>
          <div class="answers">
            We would really appreciate a donation as we are constantly improving the service and trying to keep it
            free for anyone.
            <br><br>
            BTC: bc1q9zs9n28jk4jx2w2659mhzd84skg0mlgkq7g3rr<br><br>
            ETH: 0x61bF924B3E3Cf687D8FbE39d75daea9ACB204fE3<br><br>
            SOL: DJS1CEu1atr46Tkyg5Vq2fZKqW11gj27TmfQ9VF2fZ6h<br><br>
            Monero: 4AXxCPmYcXS1HeSaxqbkhtMFy5SrmFH5cf59NK1Pj5PThcVZ4GpD5EqbcFeugatc8xXmi3T5XuSu1axT9FPedbSc4C4TEz3
          </div>
        </div>
      </div>

      <form method="post">
        <label for="hideDesc" class="Pointer float-right paddingR20">... would you like to remove description?</label>
        <input type="submit" name="hideDesc" id="hideDesc" class="hidden">
      </form>';
}
else{
  echo '<form method="post">
    <label for="hideDesc" class="Pointer float-right paddingR20 paddingT5V">... would you like to add description?</label>
    <input type="submit" name="hideDesc" id="hideDesc" class="hidden">
  </form>';
}
