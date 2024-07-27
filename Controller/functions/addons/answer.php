<?php

function questionAnswer($question, $results, $wikiTxt = null, $infoboxData = null)
{
    if(gettype($infoboxData) != 'array'){return;}

    $tmp = isset($_COOKIE['Language']) ? $_COOKIE['Language'] : 'en';
    if($tmp == 'all'){$tmp = 'en';}
    
    if(isset($wikiTxt)){
        foreach ($infoboxData as $name => $data) {
            if($name == 'images'){continue;}
            $wikiTxt .=  '. '. $name.': '.$data;
        } 
    }

    if(isset($results)){
    $resultsFormated = strip_tags(implode("\n", $results));
    }
    $data = array(
        'context' => (isset($wikiTxt) ? strip_tags($wikiTxt) : $resultsFormated), 
        'question' => $question
    );

    $data = '<div class="quickAnswer output">
    <p id="answerQuestion" class="none">'. $question.'</p>
    <p id="answerData" class="none">'. $data['context'].'</p>
    <div id="answerAnswer"></div>

    <div id="answerLoading" class="width100P centerTxt"><div class="lds-ellipsis"><div></div><div></div><div></div><div></div></div></div>

    <img class="answerHide Outfavicon" loading="lazy" src="View/img/wikiLogo.webp">
    
    <a class="removeULink clearLink" href="https://'.$tmp.'.wikipedia.org/wiki/' . $infoboxData['title'].'"'; if (isset($_COOKIE['new'])) {
        $data .= 'target="_blank"';
    }
    $data .=  '><p class="answerHide mt-5 resLink">'.$tmp.'.wikipedia.org</p></a>';
    $data .= '<a ';
    if (isset($_COOKIE['new'])) {
        $data .= 'target="_blank"';
    }
    $data .= 'href="https://'.$tmp.'.wikipedia.org/wiki/' . $infoboxData['title'].'" rel="noopener noreferrer" data-sxpr-link>';
    $data .= '<p class="answerHide ml-0 mt-5 OutTitle">' . $infoboxData['title'] . ' - Wikipedia</p></a>
    
    </div>';
    echo $data;
}