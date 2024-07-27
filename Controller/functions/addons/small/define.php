<?php
   if (isset($defWords) && isset($WordnikObj[0]['text'])) {
        echo '<div class="redditCon output"><p><b>',str_replace('%20', ' ', $defWords),'</b> | ',$WordnikObj[0]['partOfSpeech'],'</p><br>';
        $i =0;       
        foreach($WordnikObj as $defObj){
                   if(isset($defObj['text'])){
                    if($i >= 2){break;}
                       echo '<p class="txt16">• '.$defObj['text'].'</p><br>';
                       ++$i;
                   }
            }
        echo '</div>';
    }