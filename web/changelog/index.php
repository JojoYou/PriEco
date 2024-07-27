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
    <div class="headerChangelog headerDefault"></div>

    <h1 class="width100P mt-15 flex flexDRow justConC alignC">Changelog</h1>

    <div class="width100P flex alignC flexDColumn mt-150">

    <?php
    $logs = array_reverse(glob('*.php'));
    unset($logs[0]);
    $imgs = array_reverse(glob('../../View/img/web/changelog/*.webp'));
    $i=0;
    foreach($logs as &$log) {
        $img = isset($imgs[$i]) ? $imgs[$i] : '../../View/icon/img.svg';
        echo '
    <div class="max-width700 widthAuto ml-10 mr-10 centerTxt whiteAblackBg borderRadius overflowHidden Pointer mt-150">
            <a href="',$log,'" class="clearLink">
                <img class="width100P zoom"
                    src="',$img,'">
                <div class="paddingT20 paddingB20 paddingL20 paddingR20">
                    <h3>',str_replace('.php','',$log),'</h3>
                        <p>PriEco web release!</p>
                </div>
            </a>
        </div>';
        ++$i;
    }
    ?>
        

    </div>

    <?php include('../../Model/footer.php'); ?>

</body>

</html>