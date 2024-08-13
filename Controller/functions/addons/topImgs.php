<?php

if($dev || !empty($PriEcoImg)){
echo '<div class="resTopImgs">

<input type="checkbox" class="none" id="topImgConBtn">
<div class="scroll topImgCon">';

$i = 0;
foreach($PriEcoImg as $img){
    if($i >= 8){break;}
    echo '<img src="/Controller/functions/proxy.php?q=', $img['url'],'" loading="lazy">';
    ++$i;
}

echo'</div>

<label id="topImgConBtnExp" for="topImgConBtn" class="flex Pointer width80V">
<span></span>
<p class="flex alignC">Expand Images<img class="filterImage width16 ml-10 height15 opacity5" src="View/icon/dropdown.svg"></p>
<span></span></label>

<label id="topImgConBtnMin" for="topImgConBtn" class="flex Pointer width80V">
<span></span>
<p class="flex alignC">Minimize Images<img class="filterImage width16 ml-10 height15 opacity5 rotate180" src="View/icon/dropdown.svg"></p>
<span></span></label>
</div>';
}
