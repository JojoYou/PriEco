<?php
echo '<div class="redditCon output">
<b><p id="rnNum" class="centerTxt txt32">',rand(1, 10),'</p></b>';

if(!isset($_COOKIE['DisWid'])){
    echo '
    <div class="flex borderTop">
    <input class="rnInp" type="number" placeholder="From" id="from" min="0" max="1000000000" step="1">
    
    <input class="rnInp" type="number" placeholder="To" id="to" min="0" max="1000000000" step="1">
    
    <button class="width30P rnInp" id="rnGen">🎲</button>
        </div>
        <script src="View/js/rand.js"></script>
        ';
}
echo '</div>';