<!DOCTYPE html>
<html lang="en">

<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Default | PriEco</title>
    <link rel="stylesheet" href="../../css/web.css">
    <?php
    $beforePathStyle = '../../';
    include('../../Model/style.php');
    ?>
</head>

<body>
    <img class="imgCover headerDefault" src="../../View/img/web/changelog/1.0.4.webp">

    <h1 class="width100P mt-15 centerTxt">PriEco version 1.0.4</h1>
    <div class="width100P flex alignC flexDColumn mt-150">
        <div class="max-width700 width95P ml-10 mr-10 whiteAblackBg borderRadius overflowHidden">
            <div class="paddingT20 paddingB20 paddingL20 paddingR20">
                <h2 class="centerTxt">Additions</h2>
                <ul class="listInside">
                    <li>Top infobox bar.</li>
                    <li>Brave search official API.</li>
                    <li>Social media redirects.</li>
                    <li>PriEco web</li>
                    <ul class="ml-20">
                        <li>Tutorials</li>
                        <li>New changelog</li>
                    </ul>
                    <li>Archive list to additional info of results.</li>
                </ul>
            </div>
        </div>
    </div>
    <div class="width100P flex alignC flexDColumn mt-150">
        <div class="max-width700 width95P ml-10 mr-10 whiteAblackBg borderRadius overflowHidden">
            <div class="paddingT20 paddingB20 paddingL20 paddingR20">
                <h2 class="centerTxt">Speed</h2>
                <ul class="listInside">
                    <li>Decreased number of requests.</li>
                    <li>Finally migrated databases to new faster local storage.</li>
                    <ul class="ml-20">
                        <li>Added <a class="link" href="https://search.jojoyou.org/pri" target="_blank">/pri</a> version
                        </li>
                    </ul>
                    <li>Added img proxy with compression for large images.</li>
                    <li>Set max count of news in news tab.</li>
                </ul>
            </div>
        </div>
    </div>
    <div class="width100P flex alignC flexDColumn mt-150">
        <div class="max-width700 width95P ml-10 mr-10 whiteAblackBg borderRadius overflowHidden">
            <div class="paddingT20 paddingB20 paddingL20 paddingR20">
                <h2 class="centerTxt">Design</h2>
                <ul class="listInside">
                    <li>Improved landing page.</li>
                    <li>Fixed design of no results found message.</li>
                    <li>Fixed landing page design issues when Hide query was on.</li>
                    <li>Decreased landing page PriEco logo and title size.</li>
                    <li>Added border margin for landing page search bar and web results.</li>
                    <li>Improved design of settings with a help of <a href="https://www.instagram.com/elias_kofler09" target="_blank" class="link">E. Kofler</a></li>
                    <li>Settings added dropdown for JS and proxies.</li>
                    <li>Added default image if remote image fails to load.</li>
                </ul>
            </div>
        </div>
    </div>
    <div class="width100P flex alignC flexDColumn mt-150">
        <div class="max-width700 width95P ml-10 mr-10 whiteAblackBg borderRadius overflowHidden">
            <div class="paddingT20 paddingB20 paddingL20 paddingR20">
                <h2 class="centerTxt">Bug fixes</h2>
                <ul class="listInside">
                    <li>Query containing: '.</li>
                    <li>Weather appearing on reload.</li>
                </ul>
            </div>
        </div>
    </div>

    <?php include('../../Model/footer.php'); ?>
</body>

</html>