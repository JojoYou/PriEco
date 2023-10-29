<?php
function shop($ShopObj)
{
    $i =0;    
    $rPrint = false;
    if(!isset($ShopObj['offers'])){return;}
   
    usort($ShopObj['offers'], function ($a, $b) {
      $amountA = (float) $a['estimatedCpc']['amount'];
      $amountB = (float) $b['estimatedCpc']['amount'];
  
      if ($amountA == $amountB) {
          return 0;
      }
  
      return ($amountA > $amountB) ? -1 : 1;
  });
    $shop = '<p class="sectionTitle">👕 Products</p>
    
    <div class="output" style="border-radius: 20px;margin-bottom:15px;background:none;
    display:flex;overflow:auto hidden; height:270px;padding-left:0;" id="output">';
    foreach ($ShopObj['offers'] as &$item) {  
        $rPrint = true;
        if($i>6){break;}   
        $shop .= '
                    <div class="imgoutdiv" style="width:auto;min-width:unset;margin-right:30px;padding:0;">
                    <a href="'.$item['clickUrl'].'"'; 
                    if (isset($_COOKIE['new'])) {
                        $shop .=  'target="_blank"';
                    }
                    $shop .= '>
                    <button title="News button" class="ytvideobtn"';
            if(!isset($_COOKIE['datasave'])) {
                $shop .= 'style="background-image: url(Controller/functions/proxy.php?q='.urlencode($item['thumbnail']['url']).');"';
            }
            $shop .= '></button>
            <div class="imgoutlink videossearch" style="height:125px;';
            
            if(!isset($_COOKIE['datasave'])) {
              $shop .= 'background-image:url(/Controller/functions/proxy.php?q='.$item['merchant']['logo']['url'].');background-position: bottom;background-repeat: repeat-x;';
            }
              $shop.='"><div style="display: flex;align-items: center;padding: 3px;flex-direction: row;justify-content: space-between;">
                

              </div>
                <p class="ytTitle">'. substr($item['title'], 0, 47).'...</p>
        <p style="font-size:10px;padding: 0 5px 0px 5px;
        display: -webkit-box;
        -webkit-line-clamp: 3;
        line-height:14px;
        -webkit-box-orient: vertical;
        overflow: hidden;">';
        if(strlen($item['description'])>30){$shop .= substr(strip_tags($item['description']), 0, 30) . '...';}
        else{$shop .= strip_tags($item['description']);}
        $shop .= '</p>
        </div>
        </a>
        </div>
              ';
              ++$i;
                }

              $shop .= '</div>';
    if($rPrint){
    return $shop;
    }
}