<?php
function etsy($EtsyFile)
{
    $rPrint = false;
    
    if(!isset($EtsyFile['results'])){return;}

    $etsy = '<p style="float: left;
    clear: left;font-weight: bold;
    font-size: 16px;
    margin-left: calc(9vw + 20px);
    margin-top: 10px;padding-bottom: 10px;color:#616366;">👕 Products</p>
    
    <div class="output" style="border-radius: 20px;margin-bottom:15px;background:none;
    display:flex;overflow-x:auto;overflow-y:hidden;height:200px;
    " id="output">';

    foreach ($EtsyFile['results'] as &$item) {  
        $rPrint = true;
        $etsy .= '
                    <div class="imgoutdiv" style="width:auto;height:100%;margin-right:15px;">
                    <a href="'.$item['url'].'"'; 
                    if (isset($_COOKIE['new'])) {
                        $etsy .=  'target="_blank"';
                    }
                    $etsy .= '>
            <div class="imgoutlink videossearch" style="width:175px;height:100%;margin-top:0px; border-radius:20px;padding: 10px;">
            <p style="font-size:12px;color:#3391ff;line-height: 18px;font-weight: 510;padding-top: 10px;padding-bottom: 10px;">'.  substr($item['title'], 0, 50) . '...</p>
        <p style="font-size:12px;line-height:16px;">'.substr(strip_tags($item['description']), 0, 120) . '...</p>
      </div>
        </a>
        </div>
              ';
                }

              $etsy .= '</div>';
    if($rPrint){
        return $etsy;
    }
}