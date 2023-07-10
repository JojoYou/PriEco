<?php

function related($rsArray, $simImg){
    $rs = $rsArray;
    $rs = $rs[1];

    $rPrint = false;
$i=0;
$related = '<style>
.relSea{
--rel-img: url("'.$simImg.'");
} </style>';
$related .= '<p class="sectionTitle">🔗 Related searches</p>

<div class="relSea output" style="border-radius: 20px;margin-bottom:15px;
" id="output">';
foreach ($rs as &$item) { 
    if($i==0){
        ++$i;
        continue;
    } 
                $rPrint = true;
                if(!isset($_COOKIE['hQuery'])){
                $related .= '<a href="?q='.urlencode($item).'">';
                }
                else{
                    $related .= '<form method="POST" action="">
                    <input type="hidden" name="q" value="'. $item .'">';
                }
                
                $related .= '<button class="socialBtn" style="color:#3391ff; padding: 10px;float: left;margin-top: 10px;display:flex;align-items: center;">';
                if(!isset($_COOKIE['datasave'])){
                $related .= '<img loading="lazy" alt="" src="../View/icon/search.webp"class="rsImg" style="width: 15px;height:15px;margin-right: 5px;">';
                }
                $related .= '<p>'.$item.'</p></button>';
                
                if(!isset($_COOKIE['hQuery'])){
                   $related .= '</a>';
                }
                else{
                    $related .= '</form>';
                }
                
            }

          $related .= '</div>';
          if($rPrint){
          return $related;
          }
}