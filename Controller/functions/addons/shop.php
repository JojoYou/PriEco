<?php
function shop($ShopObj)
{
    $i =0;    
    $rPrint = false;
    if(!isset($ShopObj['offers'])){return;}
    $shop = '<a href="/?shop&q='.urlencode($_GET['q']).'"><p class="sectionTitle">👕 Products</p><img class="inv05b height25 adTitle sectionTitle" src="View/icon/ad.webp">
    </a>    
    <div class="addonShopOut addonOut output" id="output">';
    foreach ($ShopObj['offers'] as &$item) {  
        $rPrint = true;
        if($i>6){break;}   
        $shop .= '
            <div class="addonImgOut imgoutdiv">
                          <a href="'.$item['clickUrl'].'"'; 
                    if (isset($_COOKIE['new'])) {
                        $shop .=  'target="_blank"';
                    }
                    $shop .= '>
                    <button title="News button" class="ytvideobtn">';
            if(!isset($_COOKIE['datasave'])) {
                $shop .= '<img src="Controller/functions/proxy.php?q='.urlencode($item['thumbnail']['url']).'">';
            }
            $shop .= '</button>
            <div class="addonShopHeight imgoutlink videossearch">
                <p class="ytTitle">'. substr($item['title'], 0, 47).'...</p>
        <p class="addonDesc">';
        if(strlen($item['description'])>30){$shop .= substr(strip_tags($item['description']), 0, 70) . '...';}
        else{$shop .= strip_tags($item['description']);}
        $shop .= '</p>
        </div>
        </a>
        </div>
              ';
              ++$i;
                }
                $shop .= '<div class="addonImgOut imgoutdiv">
               <a href="/?shop&q='.urlencode($_GET['q']).'">
               <div class="addonArrow videossearch">
               <img class="filterImage" src="View/icon/arrow-right.svg"></div>
               </div>
                </a>      
                </div>';

              $shop .= '</div>';
    if($rPrint){
    return $shop;
    }
}