<?php
   if (isset($defWords) && isset($WordnikObj[0]['text'])) {
        echo '<div class="output" style="margin-bottom: 15px;border-radius: 20px;padding: 10px;"><p><b>',str_replace('%20', ' ', $defWords),'</b> | ',$WordnikObj[0]['partOfSpeech'],'</p><br>';
               foreach($WordnikObj as $defObj){
                   if(isset($defObj['text'])){
                       echo '<p style="font-size:16px;">• '.$defObj['text'].'</p><br>';
                   }
            }
        echo '</div>';
    }