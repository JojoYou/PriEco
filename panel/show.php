<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Stats | PriEco</title>
    <?php 
    $beforePathStyle = '../';include '../Model/style.php';
    ?>
</head>
<body>
<?php

$data = json_decode(file_get_contents('../count.json'), true);

    $data = array_reverse($data);
    echo '<h1 class="centerTxt">PriEco Statics</h1>
    <h2 class="centerTxt">Daily search count</h2>';
    $i=0;
    foreach ($data as $item) {
        echo '
<div class="width100P flex alignC flexDColumn mt-20">
        <div class="max-width700 width95P ml-10 mr-10 whiteAblackBg borderRadius overflowHidden">
            <div class="paddingT20 paddingB20 paddingL20 paddingR20">
            <h2 class="centerTxt">',$item['day'],'</h2>
            <p class="mt-20">Searches: ',$item['searches'],'</p>';
            if(isset($data[$i+1]['searches']) && $data[$i+1]['searches'] != 0) {
                $inc = round(($item['searches'] / $data[$i+1]['searches']) * 100 - 100, 2);
                if($inc > 0){
                    echo '<p class="mt-20 colorGreen">Increse: ',$inc,'%</p>';
                }
                else if($inc == 0){
                    echo '<p class="mt-20">Stay: ',$inc,'%</p>';
                }
                else{
                    echo '<p class="mt-20 colorRed">Decrease: ',$inc,'%</p>';
                }
            }
            echo '</div>
            </div>
            </div>';
            ++$i;
    }

    include '../Model/footer.php';
?>
</body>
</html>