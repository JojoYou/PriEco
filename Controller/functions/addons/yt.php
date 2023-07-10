<?php
function youtube($YoutubeObj){
    $i =0;    
    $rPrint = false;
    $yt = '<p class="sectionTitle">📷 Videos</p>
    
    <div class="output" style="border-radius: 20px;margin-bottom:15px;background:none;
    display:flex;overflow:auto hidden; height:300px;" id="output">';
    foreach ($YoutubeObj['items'] as &$item) {  
        $rPrint = true;
        if($i>6){break;}   
        $yt .= '
                    <div class="imgoutdiv" style="width:auto;min-width:unset;margin-right:10px;padding:0;">
                    <a href="https://www.youtube.com/watch?v=' . $item['id']['videoId'] . '"'; 
                    if (isset($_COOKIE['new'])) {
                        $yt .=  'target="_blank"';
                    }
                    $yt .= '>
                    <button title="YouTube video button" class="ytvideobtn"';
            if(!isset($_COOKIE['datasave'])) {
                $yt .= 'style="background-image: url(/Controller/functions/proxy.php?q='.$item['snippet']['thumbnails']['medium']['url'].');"';
            }
            $yt .= '></button>
            <div class="imgoutlink videossearch">
              <div style="display: flex;align-items: center;padding: 3px;flex-direction: row;justify-content: space-between;">
                <div style="display:flex;align-items: center;">';
                if(!isset($_COOKIE['datasave'])) {
                  $yt .= '<img alt="" style="width: 20px;height: 20px;border-radius: 20px;"src="View/icon/profiles/youtube.svg">';
                }
                $yt .= '<p style="font-size:10px;padding-left:5px;">YouTube</p></div>
                <p style="font-size:10px;padding-right:5px;">';
                $currentDate = new DateTime();
                $specifiedDate = new DateTime($item['snippet']['publishTime']);
                $yt .=$currentDate->diff($specifiedDate)->format('%a').' days ago</p>
              </div>
                <p class="ytTitle">'.substr($item['snippet']['title'], 0, 47).'...</p>
        <p style="font-size:10px;padding: 0 5px 0px 5px;
        display: -webkit-box;
        -webkit-line-clamp: 3;
        line-height:14px;
        -webkit-box-orient: vertical;
        overflow: hidden;">'.substr(strip_tags($item['snippet']['description']), 0, 120) . '...</p>
        </div>
        </a>
        </div>
              ';
              ++$i;
                }

              $yt .= '</div>';
    if($rPrint){
    return $yt;
    }
}