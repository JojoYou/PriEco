<?php
function wiki($purl, $infoboxData, $wikiTxt, $ddgObj, $mysql, $hideQueryCopy, $PriEcoImg)
{
    $tmp = isset($_COOKIE['Language']) ? $_COOKIE['Language'] : 'en';
    if($tmp == 'all'){$tmp = 'en';}
    $answer = '';
    
    if(!isset($wikiTxt) || substr_compare($wikiTxt, 'may refer to: ', -14) === 0){return;}

    //title
    $answer .= '<div class="answer" id="answer"><a href="https://'.$tmp.'.wikipedia.org/wiki/' . str_replace('+','_',urlencode(ucwords($infoboxData['title']))) . '"';if(isset($_COOKIE['new'])){$answer.= 'target="_blank"';}$answer.=' class="clearLink"><h2>' . $infoboxData['title'].'</h2></a>';

    if(!isset($_COOKIE['datasave']) && isset($infoboxData['images'])){
        $answer .= $sum . '<a aria-label="Search for images" href="/?image&q='.urlencode($purl).'"';if(isset($_COOKIE['new'])){$answer.= 'target="_blank"';}
        $answer.='><div class="wikiImages">';
    }
    //images
    $i=0;
    if(!isset($_COOKIE['datasave'])){
        if(isset($infoboxData['images'][0])){
            $answer .= '<img src="/Controller/functions/proxy.php?q=' . substr($infoboxData['images'][0], 2) . '" class="mr-10 width50P">';
        }
        if(isset($infoboxData['images'][1])){
            $answer .= '
            <div>
            <img src="/Controller/functions/proxy.php?q=' . $PriEcoImg[8]['url'] . '">
            <img src="/Controller/functions/proxy.php?q=' . $PriEcoImg[9]['url'] . '">
            <img src="/Controller/functions/proxy.php?q=' . $PriEcoImg[10]['url'] . '">
            <img src="/Controller/functions/proxy.php?q=' . $PriEcoImg[11]['url'] . '">
            </div>';
        }
}
if(!isset($_COOKIE['datasave']) && isset($infoboxData['images'])){$answer .='</div></a>';}

//Website
 if(isset($infoboxData['Website'])){
        
    $wurl = trim(html_entity_decode($infoboxData['Website']));
    
    $answer.= '<a class="link"href="https://'.$infoboxData['Website'].'"';
    if(isset($_COOKIE['new'])){$answer.= 'target="_blank"';}
    $answer .='>';
    $answer .= '🔗 '.str_replace('www.','', parse_url('https://'.$wurl)['host']);
    $answer .='</a>';
}

 //Description
 $answer.='<p>' .cutString($wikiTxt,300) . 
 '<a class="link" href="https://'.$tmp.'.wikipedia.org/wiki/' . str_replace('+','_',urlencode(ucwords($infoboxData['title']))). '"';if(isset($_COOKIE['new'])){$answer.= 'target="_blank"';}$answer.='>Wikipedia</a>
 </p>';

//Summarized
if(isset($wikiTxt)){
   // $summary = summarizeText($wikiTxt, 2);
    $wikiTxt = substr($wikiTxt,0, 600);
    foreach($summary as &$su){
      $Tsum .= ' '.$su;
    }
    if(strlen($Tsum) >= 200 && strlen($Tsum) <= 850){
    $answer .= '<input type="checkbox" id="sumMoreCheck" class="none">
    <label class="sumMore" for="sumMoreCheck"><p><b>Summarized</b></p><p>'.$Tsum.'</p></label>';
    $loaded[0] = true;
    }
}

    //Infobox
    $answer .= '<input type="checkbox" id="wikiMoreCheck" class="none">
    <label class="notSelectable wikiMore" for="wikiMoreCheck">';
foreach ($infoboxData as $name => $data) {
    if($name == 'images'){continue;}
    $answer .= '<div class="mb-10"><p><b>'.$name . '</b></p><p>' . $data . '</p></div>';
} 
$answer .= '</label>';

    if($answer!=''){
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

        if(is_string($ddgObj)){$ddgObj=json_decode($ddgObj,true);}

        if(is_array($ddgObj)){
            $twitter = $ddgObj['Twitter'];
            $facebook = $ddgObj['Facebook'];
            $imdb = $ddgObj['IMDb'];
            $tomato = $ddgObj['Tomatoes'];
            $spotify = $ddgObj['Spotify'];
            $apple = $ddgObj['Apple'];
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
    
    $answer .= $hideQueryCopy .'</div>';
   
    return $answer;
    }
}