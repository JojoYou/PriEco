<!DOCTYPE html>
<html lang="en">

<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Default | PriEco</title>
    <link rel="stylesheet" href="../css/web.css">
    <?php
    $beforePathStyle = '../';
    include('../Model/style.php');
    ?>
</head>

<body>
    <div class="headerIndex headerDefault"></div>

    <h1 class="width100P mt-15 centerTxt">Welcome to PriEco🌍Web!</h1>
    <p class="width100P mt-15 centerTxt">All in one place to stay updated with PriEco.</p>

    <div class="width100P flex alignC flexDColumn mt-150">

    <div class="max-width700 widthAuto ml-10 mr-10 centerTxt whiteAblackBg borderRadius overflowHidden Pointer">
            <a href="tutorials/default" class="clearLink">
                <img class="width100P zoom"
                    src="../View/img/web/tutorials.webp">
                <div class="paddingT20 paddingB20 paddingL20 paddingR20">
                    <h2>Tutorials</h2>
                        <p>How to set up PriEco in your web browser?<br>
                        We got you covered.</p>
                </div>
            </a>
        </div>

        <div class="max-width700 widthAuto ml-10 mr-10 centerTxt whiteAblackBg borderRadius overflowHidden Pointer mt-150">
            <a href="changelog" class="clearLink">
                <img class="width100P zoom"
                    src="../View/img/web/changelog.webp">
                <div class="paddingT20 paddingB20 paddingL20 paddingR20">
                    <h2>Changelog</h2>
                        <p>See changes in PriEco version by version.</p>
                </div>
            </a>
        </div>

    </div>

    <?php include('../Model/footer.php'); ?>
</body>

</html>