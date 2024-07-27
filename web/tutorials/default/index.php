<!DOCTYPE html>
<html lang="en">

<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Default | PriEco</title>
    <link rel="stylesheet" href="../../../css/web.css">
    <?php
    $beforePathStyle = '../../../';
    include('../../../Model/style.php');
    ?>
</head>

<body>
    <div class="headerPriEco headerDefault"></div>

    <h1 class="width100P mt-15 flex flexDRow justConC alignC">Set <img class="ml-10 mb-10 height40" src="../../../View/img/PriEco.webp">PriEco in your<span class="defaultTitle"></span></h1>

    <div class="width100P flex alignC flexDColumn mt-150">

    <div class="max-width700 widthAuto ml-10 mr-10 centerTxt whiteAblackBg borderRadius overflowHidden Pointer">
            <a href="chrome.php" class="clearLink">
                <img class="width100P zoom"
                    src="../../../View/img/web/tutorials/default/chrome/chromeBanner.webp">
                <div class="paddingT20 paddingB20 paddingL20 paddingR20">
                    <h3>Chromium browsers</h3>
                        <p>Google Chrome<br>
                        Microsoft Edge<br>
                        Brave browser<br>
                        Vivaldi<br>
                        Opera</p>
                </div>
            </a>
        </div>

        <div class="max-width700 widthAuto ml-10 mr-10 centerTxt whiteAblackBg borderRadius overflowHidden Pointer mt-150">
            <a href="firefox.php" class="clearLink">
                <img class="width100P zoom"
                    src="../../../View/img/web/tutorials/default/firefox/firefoxBanner.webp">
                <div class="paddingT20 paddingB20 paddingL20 paddingR20">
                    <h3>Firefox browsers</h3>
                        <p>Firefox<br>
                        Tor browser<br>
                        Waterfox</p>
                </div>
            </a>
        </div>

    </div>

    <?php include('../../../Model/footer.php'); ?>

    <script src="../../../View/js/web.js"></script>
</body>

</html>