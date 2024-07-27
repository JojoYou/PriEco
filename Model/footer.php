<!--  SPDX-FileCopyrightText: 2022, 2022-2022 Roman  Láncoš <jojoyou@jojoyou.org> -->
<!-- -->
<!--  SPDX-License-Identifier: AGPL-3.0-or-later -->
<?php
$jsVersion = 8;
if (!isset($_COOKIE['DisWid'])) {
  echo '<script src="View/js/summary.js?v='.$jsVersion.'"></script>
  <script src="View/js/notify.js?v='.$jsVersion.'"></script>
  <script src="View/js/apiLoad.js?v='.$jsVersion.'"></script>';
}
if (!isset($_COOKIE['DisSugges'])) {
  echo '<script src="View/js/suggest.php"></script>';
}
if (!isset($_COOKIE['DisQue'])) {
  echo '<script src="View/js/delQuery.js"></script>';
}

$beforePathFooter = isset($beforePathFooter) ? $beforePathFooter : '';
?>

<div class="footer">
  <div class="footer-content-left">
    <h3 class="txt40">PriEco</h3>
    <p>Made for supporting Privacy &amp; Ecology</p>
    <div class="mt-20 mb-20 flex">
      <a class="mr-20" href="https://x.com/PriEcoSearch"><img src="<?php echo $beforePathFooter ?>View/img/x.webp"></a>
      <a href="https://www.tiktok.com/@priecosearch"><img src="<?php echo $beforePathFooter ?>View/img/tiktok.webp"></a>
    </div>
    <span>©2021- 2024 Roman Láncoš, JojoYou</span>
  </div>

  <div class="flex mr-50 footer-content-right">
            <div class="legal">
                <a class="clearLink" href="web/privacypolicy.php">Privacy Policy</a><br>
                <a class="clearLink" href="https://codeberg.org/JojoYou/PriEco/src/branch/master/LICENCE.md">License</a><br>
                <a class="clearLink" href="https://codeberg.org/JojoYou/PriEco/src/branch/master">Source code</a><br>
                <a class="clearLink" href="http://priecovk7jsuh3tvkh62c6j4oep3l5bldigpzmay26rdpqz357t5dmad.onion/">Tor onion domain</a><br>
            </div>
            <div class="join">
                <a class="clearLink" href="https://signal.group/#CjQKIF8X7zfpbghe6yFKUXKZppu_wkjELEXEmRtAdUJ2kqLNEhAWBjviRL1Dbw_BFvGSpJnI">Signal Group</a><br>
                <a class="clearLink" href="https://discord.gg/gmUgAVkTKx">Discord Server</a><br>
                <a class="clearLink" href="https://t.me/priecosearch">Join Telegram</a><br>
            </div>
        </div>
</div>

<a rel="me" class="none" href="https://fosstodon.org/@prieco">Mastodon</a>