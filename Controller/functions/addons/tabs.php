<b><label for="showTabs"
        class="borderRadius borderNone paddingCircle5 colorWhite lightShadow Pointer bottom15 mr-20 right0 fixed bgTop z-index1">
        <?php echo isset ($_COOKIE['tabs']) ? count(explode(',', $_COOKIE['tabs'])) : 1; ?>
    </label></b>
<input class="none" type="checkbox" id="showTabs">

<div class="fixed top0 whiteAblackBg width100V height100V" id="tabContainer">
    <div class="wrap scrollDown height100P alignConS">

        <?php
        $i = 0;
        $tabs = explode(',', $_COOKIE['tabs']);
        ++$i;


        //Update current tab
        if ($purl != $tabs[$_GET['tab']]) {
            $tabs[$_GET['tab']] = $purl;
            setcookie('tabs', implode(',', $tabs), -1, '/');
            $reload = true;
        }

        foreach ($tabs as &$tab) {
            echo '<div class="margin10 centerTxt width50P">
        <form method="POST">
            <input class="none" name="tabID" value="', $i, '">
            <label for="tabClose"><img src="View/icon/cross.svg" class="Pointer borderRadius bgWhite filterImage padding4 width20 relative left50PM35 top30"></label>
            <input type="submit" class="none" id="tabClose" name="tabClose">
        </form>';
            if (filter_var(urldecode($tab), FILTER_VALIDATE_URL)) {
                echo '<a href="proxy?url=', urldecode($tab), '" target="_blank">';
            } else {
                echo '<a href="/?tabs&tab=', $i, '&q=', ($tab != '' ? $tab : ''), '">';
            }
            echo '<div class="paddingL20 paddingR20 height200">';
            if (filter_var(urldecode($tab), FILTER_VALIDATE_URL)) {
                $ch = curl_init();

                // Set cURL options
                curl_setopt($ch, CURLOPT_URL, 'https://obunic.com/api/screenshot-webpage/?width=1280&height=720&url=' . urldecode($tab));
                curl_setopt($ch, CURLOPT_RETURNTRANSFER, 1);

                $remote_png = curl_exec($ch);
                /* $png = imagecreatefromstring($remote_png);
                 $width = imagesx($png);
                 $height = imagesy($png);
                 $file = fopen('image.txt', 'w');
                 for ($y = 0; $y < $height; $y++) {
                     for ($x = 0; $x < $width; $x++) {
                         $rgb = imagecolorat($png, $x, $y);
                         $colors = imagecolorsforindex($png, $rgb);
                         fwrite($file, implode(',', $colors) . "\n");
                     }
                 }
                 fclose($file);
                 imagedestroy($png);*/

                echo '<img src="https://obunic.com/api/screenshot-webpage/?width=1280&height=720&url=' . urldecode($tab) . '" class="lightShadow borderRadius imgCover width100P height100P">';
            } else {
                echo '<img src="View/img/PriEcoTab.webp" class="lightShadow borderRadius imgCover width100P height100P">';
            }
            echo '</div>
        </a>
        <b><p>', ($tab != '' ? str_replace('/', '', str_replace('www.', '', str_replace('https://', '', str_replace('http://', '', urldecode($tab))))) : 'Home'), '</p></b>
    </div>';
            ++$i;
        }

        ?>
    </div>
    <div class="flex absolute bottom0 width100V">
        <label for="showTabs"
            class="borderNone borderRadius paddingCircle5 colorWhite lightShadow Pointer bottom15 mr-20 right0 absolute bgTop z-index1"><img
                src="View/icon/dropdown.svg" class="width10 invert"></label>
        <form method="POST">
            <input type="submit"
                class="borderNone borderRadius paddingCircle5 colorWhite lightShadow Pointer bottom15 ml-20 left0 absolute bgTop z-index1"
                name="addTab" value="+">
        </form>
    </div>
</div>