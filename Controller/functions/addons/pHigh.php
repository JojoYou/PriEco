<?php
function pHigh($purl){
    $purlLow = strtolower($purl);
    $privateOpen = [
        'twitter;Mastodon;mastodon.webp;https://joinmastodon.org/;C;639',
        'instagram;Pixelfed;pixelfed.webp;https://pixelfed.org/',

        'windows,macos,mac os;Ubuntu;ubuntu.webp;https://ubuntu.com/',
        'windows,macos,mac os;VanillaOS;vanillaos.svg;https://vanillaos.org',

        'mail,email,gmail,outlook;Proton Mail;protonmail.webp;https://proton.me/mail/;A;491',
        'mail,email,gmail,outlook;Skiff Mail;skiffmail.webp;https://skiff.com/mail/',

        'google docs,notion,evernote;Skiff Pages;skiffPage.webp;https://skiff.com/pages',
        
        'chrome,google chrome,opera,edge,chromium,safari,browser;Firefox;firefox.webp;https://www.mozilla.org/en-US/firefox/new/;B;188',
        'chrome,google chrome,opera,edge,chromium,safari,browser;Brave;brave.webp;https://brave.com/;B;1487',
        'chrome,google chrome,opera,edge,chromium,safari,browser;Vivaldi;vivaldi.webp;https://vivaldi.com/;B;2371',

        'chat,messenger;Signal;signal.webp;https://www.signal.org/',
        'chat,messenger;Session;session.webp;https://getsession.org/;B;3015',
        'chat,messenger;Simplex;simplex.webp;https://simplex.chat/',
        'chat,messenger,discord;Revolt;revolt.webp;https://revolt.chat/'

        ];
    $load = false;
    $privateAltOut='<p class="sectionTitle">⚡ Alternatives</p>
    <div class="redditCon flex wrap justConC flexDRow output">';

    foreach($privateOpen as &$pO){
        $tmp = explode(';',$pO);
        if(in_array($purlLow, explode(',',$tmp[0]))){
            $load = true;
            $privateAltOut .= '<div class="centerTxt flex flexDColumn width100">
              <a href="'.$tmp[3].'"';if (isset($_COOKIE['new'])) {$privateAltOut .='target="_blank"';}$privateAltOut .='>';
              if(!isset($_COOKIE['datasave'])){
                $privateAltOut .= '<img src="View/img/privateAlt/'.$tmp[2].'" class="wh50">';
              }
              $privateAltOut .='<p>'.$tmp[1].'</p>
              </a>
              
              <a href="https://tosdr.org/en/service/'.$tmp['5'].'"';if (isset($_COOKIE['new'])) {$privateAltOut .='target="_blank"';}$privateAltOut .='>
              <p class="';
              switch($tmp[4]){
                case 'A':
                    $privateAltOut .= 'darkGreenBtn';
                    break;
                case 'B':
                    $privateAltOut .= 'greenBtn';
                    break;
                case 'C':
                    $privateAltOut .= 'orangeRedBtn';
                    break;
                case 'D':
                    $privateAltOut .= 'orangeBtn';
                    break;
                case 'E':
                    $privateAltOut .= 'redBtn';
                    break;
            }
              $privateAltOut .= ' colorWhite curve"><b>'.$tmp[4].'</b></p>
              </a>
            </div>';
        }
    }
    $privateAltOut .= '</div>';
    if($load){return $privateAltOut;}
    else{return '';}
}