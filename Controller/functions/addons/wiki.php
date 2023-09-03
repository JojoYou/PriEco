<?php
function wiki($infoboxData, $wikiTxt, $ddgObj, $mysql, $hideQueryCopy)
{
    $tmp = ($_COOKIE['Language'] == 'all') ? 'en' : $_COOKIE['Language'];
    $answer = '';
    $ansImg = '';
    
    if(!isset($wikiTxt) || str_ends_with($wikiTxt, 'may refer to: ')){return;}

    //title
    $answer .= '<div class="answer" id="answer"><a href="https://'.$tmp.'.wikipedia.org/wiki/' . str_replace('+','_',urlencode(ucwords($infoboxData['title']))) . '"';if(isset($_COOKIE['new'])){$answer.= 'target="_blank"';}$answer.=' style="color: unset;text-decoration: unset;"><h2>' . $infoboxData['title'].'</h2></a><br>';

    if(!isset($_COOKIE['datasave']) && isset($infoboxData['images'])){
        $answer .= $sum . '<div style="display: flex;
    flex-wrap: wrap;
    flex-direction: row;
    justify-content: space-around;
    align-items: center;
    background-color: #00000007;
    padding: 10px;
    border-radius: 20px;">';
    }
    //images
    $i=0;
    if(!isset($_COOKIE['datasave'])){
    foreach ($infoboxData['images'] as $imageUrl) {
        if($i == 2){break;}
        if($ansImg == ''){$ansImg = substr($imageUrl, 2);}
        $answer .= '<img alt="" src="/Controller/functions/proxy.php?q=' . substr($imageUrl, 2) . '" style="max-width: 50%;border-radius: 30px;max-height: 200px;height:auto;width: auto;"><br>';
        ++$i;
    }
}
if(!isset($_COOKIE['datasave']) && isset($infoboxData['images'])){$answer .='</div>';}

//Website
 if(isset($infoboxData['Website'])){
        
    $wurl = trim(html_entity_decode($infoboxData['Website']));
    
    $answer.= '<a style="color: var(--linkColor);text-decoration: none;"href="https://'.$infoboxData['Website'].'"';
    if(isset($_COOKIE['new'])){$answer.= 'target="_blank"';}
    $answer .='>';
    $answer .= '🔗 '.str_replace('www.','', parse_url('https://'.$wurl)['host']);
    $answer .='</a>';
}

 //Description
 $answer.='<br><br><p style="background-color: #00000007;padding: 15px;border-radius: 20px;">' . substr($wikiTxt, 0, 500) . '...' . 
 '<a style="color: var(--linkColor);text-decoration: none;" href="https://'.$tmp.'.wikipedia.org/wiki/' . str_replace('+','_',urlencode(ucwords($infoboxData['title']))). '"';if(isset($_COOKIE['new'])){$answer.= 'target="_blank"';}$answer.='>Wikipedia</a>
 </p><br>
 <div style="display: flex;padding-left: 10px;padding-right: 10px;">';

//Summarized
if(isset($wikiTxt)){
    $wikiTxt = substr($wikiTxt,0, 600);
    $summary = summarizeText($wikiTxt, 2);
    foreach($summary as &$su){
      $Tsum .= ' '.$su;
    }
    if(strlen($Tsum) >= 200 && strlen($Tsum) <= 850){
    $answer .= '<input type="checkbox" id="sumMoreCheck" style="display:none">
    <label class="sumMore" for="sumMoreCheck" style="margin-right: 10px;"><p><b style="font-size: 13px;">Summarized</b></p><p style="margin-top:10px;font-size:13px;">'.$Tsum.'</p></label>';
    $loaded[0] = true;
    }
}

    //Infobox
    $answer .= '<input type="checkbox" id="wikiMoreCheck" style="display:none">
    <label class="wikiMore" for="wikiMoreCheck"><p><b style="font-size: 13px;">Infobox</b></p>';
foreach ($infoboxData as $name => $data) {
    if($name == 'images'){continue;}
    $answer .= '<div style="margin-top:10px;font-size:13px;"><p style="font-weight: bold;font-size: 12px;">'.$name . '</p><p>' . $data . '</p></div>';
} 
$answer .= '</label></div>';

    if($answer!=''){
        $answer .= '<p style="font-weight: bold;font-size: 12px;">Profiles</p>';
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