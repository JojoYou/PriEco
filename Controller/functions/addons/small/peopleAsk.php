<?php
function peopleAsk($title){
    $people = '<div class="peopleAskFor output" data="' . $title.'">
    <p id="peopleAskTitle"><b>People also ask</b></p>
    
    <div class="peopleAskDiv">
    <div class="lds-ellipsis"><div></div><div></div><div></div><div></div></div>
    <input id="peopleAsk0" class="peopleAsk none" type="checkbox">
    <label id="peopleAskQuestion0" class="peopleAskLabel" for="peopleAsk0"></label>

    <p class="peopleAskP0 peopleAskP"></p>

    </div class="peopleAskDiv">

    <div class="peopleAskDiv">
    <div class="lds-ellipsis"><div></div><div></div><div></div><div></div></div>
        <input id="peopleAsk1" class="peopleAsk none" type="checkbox">
        <label id="peopleAskQuestion1" class="peopleAskLabel" for="peopleAsk1"></label>
    
        <p class="peopleAskP1 peopleAskP"></p>
    
    </div>

    <div class="peopleAskDiv">
    <div class="lds-ellipsis"><div></div><div></div><div></div><div></div></div>
    <input id="peopleAsk2" class="peopleAsk none" type="checkbox">
    <label id="peopleAskQuestion2" class="peopleAskLabel" for="peopleAsk2"></label>

    <p class="peopleAskP2 peopleAskP"></p>

    </div>

    <div class="peopleAskDiv">
    <div class="lds-ellipsis"><div></div><div></div><div></div><div></div></div>
    <input id="peopleAsk3" class="peopleAsk none" type="checkbox">
    <label id="peopleAskQuestion3" class="peopleAskLabel" for="peopleAsk3"></label>

    <p class="peopleAskP3 peopleAskP"></p>

    </div>
    <i><p class="txt14 endTxt opacity5">* answers are AI generated and may be inaccurate</p></i>
</div>';

return $people;
}