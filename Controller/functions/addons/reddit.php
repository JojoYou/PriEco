<?php
function search_reddit($results) {
        $rPrint = false;
        $conversations = '';
        $conversations.='<p class="sectionTitle">💬 Discussions</p>

        <div class="output" style="border-radius: 20px;margin-bottom:15px;" id="output">';
        foreach($results['data']['children'] as &$res){        
            $rPrint = true;
            $conversations.='<div style="width:100%;overflow:hidden;">';  
            $conversations.= '<div class="OutSideImg"';
            if(filter_var($res['data']['thumbnail'], FILTER_VALIDATE_URL)){
                if(!isset($_COOKIE['datasave'])) {
                    $conversations .= 'style="background-image: url(/Controller/functions/proxy.php?q='. $res['data']['thumbnail']. ');background-position: center;"';
                }
            }
            $conversations .= '></div><div style="display:flex;width: calc(100% - 102px);">';
            if(!isset($_COOKIE['datasave'])) {
            $conversations.='<img alt="" class="Outfavicon" loading="lazy" src="/View/img/reddit.webp">  ';
            }          
           $conversations .=' <a ';
            if (isset($_COOKIE['new'])) {
                $conversations.='target="_blank"';
            }
            $conversations.= 'href="https://www.reddit.com'. $res['data']['permalink']. '" style="padding-bottom:unset;">';
            $conversations.= '<p class="OutTitle" style="margin-left: 0px;width: 100%;">'.substr($res['data']['title'], 0, 50). '...</p></a></div>
            <section style="display:inline-block;color:#747684;font-size:12px;width: calc(100% - 132px);margin-left: 30px;padding:10px 0 10px 0;">r/'. $res['data']['subreddit'].' ⋮ '.$res['data']['num_comments'].' 💬 ⋮ '.$res['data']['ups'].' 🔼 ⋮ '.$res['data']['upvote_ratio']*100 .'% 👍 ⋮ Author: <p style="font-weight:bold;display:inline;">'.$res['data']['author'].'</p></section>';
            if($res['data']['selftext'] != ''){
                $conversations.= '<p class="snippet" style="margin-left:30px;">'.  substr($res['data']['selftext'], 0, 100). '...</p>';
            }
            $conversations.='</div>';
        }
        $conversations.='</div>';
        if($rPrint){
        return $conversations;
        }
    }