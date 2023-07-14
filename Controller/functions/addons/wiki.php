<?php
function wiki($infoboxData, $wikiTxt, $ddgObj, $mysql, $hideQueryCopy)
{
    $tmp = ($_COOKIE['Language'] == 'all') ? 'en' : $_COOKIE['Language'];
    $answer = '';
    $ansImg = '';
    
    //title
    $answer .= '<div class="answer" id="answer"><h2>' . $infoboxData['title'] . '</h2><br>';

    $wikiTxt = substr($wikiTxt,0, 600);
    if(isset($wikiTxt)){
        $summary = summarizeText($wikiTxt, 2);
        $sum = '';
        foreach($summary as &$su){
          $sum .= ' '.$su;
        }
        if(strlen($sum) >= 200 && strlen($sum) <= 850){
        $sum = '<div style="border-radius:20px 20px 0 0;padding: 10px 1% 10px 1%;"><p><b style="opacity:0.7;font-size:16px;">📑 Summarized</b></p>
        <p style="padding: 10px;border-radius: 20px;">'.$sum.'</p></div>';
        $loaded[0] = true;
        }
        else{$sum = '';}
    }

    $answer .= $sum . '<div style="display: flex;
    flex-wrap: wrap;
    flex-direction: row;
    justify-content: space-around;
    align-items: center;
    background-color: #0001;
    padding: 10px;
    border-radius: 20px;">';

    //images
    $i=0;
    if(!isset($_COOKIE['datasave'])){
    foreach ($infoboxData['images'] as $imageUrl) {
        if($i == 0){$ansImg = 'https://'.substr($imageUrl, 2);}
        if($i == 2){break;}
        $answer .= '<img alt="" src="/Controller/functions/proxy.php?q=' . substr($imageUrl, 2) . '" style="max-width: 50%;border-radius: 30px;max-height: 200px;height:auto;width: auto;"><br>';
        ++$i;
    }
}
    $answer .='</div><br>';

    //Website
    if(isset($infoboxData['Website'])){
        
        $wurl = trim(html_entity_decode($infoboxData['Website']));
        
        $answer.= '<a style="color: #3391ff;text-decoration: none;"href="https://'.$infoboxData['Website'].'"';
        if(isset($_COOKIE['new'])){$answer.= 'target="_blank"';}
        $answer .='>';
        $answer .= '🔗 '.str_replace('www.','', parse_url('https://'.$wurl)['host']);
        $answer .='</a><br>';
    }

    //Description
    $answer.='<p style="font-weight: bold;font-size: 12px;margin-top:15px;">Description:</p>
    <p>' . substr($wikiTxt, 0, 500) . '...' . 
    '<a style="color: #3391ff;text-decoration: none;" href="https://'.$tmp.'.wikipedia.org/wiki/' . str_replace('+','_',urlencode(ucwords($infoboxData['title']))) . '" target="_blank">Wikipedia</a>
    </p><br>';

    //Infobox
    $answer .= '<input type="checkbox" id="wikiMoreCheck" style="display:none">
    <label class="wikiMore" for="wikiMoreCheck">';
foreach ($infoboxData as $name => $data) {
    if($name == 'images'){continue;}
    $answer .= '<div style="margin-top:10px;"><p style="font-weight: bold;font-size: 12px;">'.$name . '</p><p>' . $data . '</p></div>';
} 
$answer .= '</label>';

    if($answer!=''){
        $answer .= '<br><p style="font-weight: bold;font-size: 12px;margin-top:15px;">Profiles</p>';
        $answer .= '<a href="https://'.$tmp.'.wikipedia.org/wiki/' . $infoboxData['title'].'"'; if (isset($_COOKIE['new'])) {
            $answer .= 'target="_blank"';
        } $answer .='><button class="socialBtn"><div>';
        if(!isset($_COOKIE['datasave'])) {$answer.='<img alt="‎" src="./View/icon/profiles/wiki.svg" class="profileIcon">';}
        $answer .= '<p>Wikipedia</p></div></button></a>';

        $twitter='';
        $facebook='';
        $imdb='';
        $tomato='';
        $spotify='';
        $apple='';
        if(!$mysql && gettype($ddgObj) == 'string'){
            $ddgObj = explode(' ', $ddgObj);
            $twitter = $ddgObj[0];
            $facebook = $ddgObj[1];
            $imdb = $ddgObj[2];
            $tomato = $ddgObj[3];
            $spotify = $ddgObj[4];
            $apple = $ddgObj[5];
        }
        else{
            if(isset($ddgObj['Infobox']['content'])){
            foreach($ddgObj['Infobox']['content'] as &$item){
                if($item['data_type'] == 'twitter_profile'){
                    $twitter = $item['value'];
                }
                if($item['data_type'] == 'facebook_profile'){
                    $facebook = $item['value'];
                }
                if($item['data_type'] == 'imdb_id'){
                    $imdb = $item['value'];
                }
                if($item['data_type'] == 'rotten_tomatoes'){
                    $tomato = $item['value'];
                }
                if($item['data_type'] == 'spotify_artist_id'){
                    $spotify = $item['value'];
                }
                if($item['data_type'] == 'itunes_artist_id'){
                    $apple = $item['value'];
                }
            }
        }
        }

        if($twitter != ''){
            $answer .= '<a href="https://twitter.com/'.$twitter.'"'; if (isset($_COOKIE['new'])) {
                $answer .= 'target="_blank"';
            } $answer .='><button class="socialBtn"><div>';
            if(!isset($_COOKIE['datasave'])) {$answer.='<img alt="‎" src="./View/icon/profiles/twitterlogo.svg" class="profileIcon">';}
            $answer .= '<p>Twitter</p></div></button></a>';
        }

        if($facebook != ''){
            $answer .= '<a href="https://www.facebook.com/'.$facebook.'"'; if (isset($_COOKIE['new'])) {
                $answer .= 'target="_blank"';
            } $answer .='><button class="socialBtn"><div>';
            if(!isset($_COOKIE['datasave'])) {$answer.='<img alt="‎" src="./View/icon/profiles/facebook.svg" class="profileIcon">';}
            $answer .= '<p>Facebook</p></div></button></a>';
        }
            
        if($imdb != ''){
            $answer .= '<a href="https://www.imdb.com/name/'.$imdb.'"'; if (isset($_COOKIE['new'])) {
                $answer .= 'target="_blank"';
            } $answer .='><button class="socialBtn"><div>';
            if(!isset($_COOKIE['datasave'])) {$answer.='<img alt="‎" src="./View/icon/profiles/imdb.svg" class="profileIcon">';}
            $answer .= '<p>IMDb</p></div></button></a>';
        }
        if($tomato != ''){
            $answer .= '<a href="https://www.rottentomatoes.com/'.$tomato.'"'; if (isset($_COOKIE['new'])) {
                $answer .= 'target="_blank"';
            } $answer .='><button class="socialBtn"><div>';
            if(!isset($_COOKIE['datasave'])) {$answer.='<img alt="‎" src="./View/icon/profiles/tomato.svg" class="profileIcon">';}
            $answer .= '<p>Rotten Tomatoes</p></div></button></a>';
            }

        if($spotify != ''){
            $answer .= '<a href="https://open.spotify.com/artist/'.$spotify.'"'; if (isset($_COOKIE['new'])) {
                $answer .= 'target="_blank"';
            } $answer .='><button class="socialBtn"><div>';
            if(!isset($_COOKIE['datasave'])) {$answer.='<img alt="‎" src="./View/icon/profiles/spotify.svg" class="profileIcon">';}
            $answer .= '<p>Spotify</p></div></button></a>';
        }

        if($apple != ''){
            $answer .= '<a href="https://music.apple.com/artist/'.$apple.'"'; if (isset($_COOKIE['new'])) {
                $answer .= 'target="_blank"';
            } $answer .='><button class="socialBtn"><div>';
            if(!isset($_COOKIE['datasave'])) {$answer.='<img alt="‎" src="./View/icon/profiles/apple.svg" class="profileIcon">';}
            $answer .= '<p>Apple Music</p></div></button></a>';
        }
    
    $answer .= '<br>'. $hideQueryCopy .'</div>';// Place for ad after <br>
   
    $ret[] = $answer;
    $ret[1] = $ansImg;
    return $ret;
    }
}