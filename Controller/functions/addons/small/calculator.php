

    <div class="calculator redditCon output">
        <div class="display">
            <input id="screen" type="text" placeholder=":)">
        </div>
        <input type="radio" id="calcFx" name="calcGroup" class="none">

          <div class="halfCalculator" id="halfCalculator2">

            <div>
                <button id="mathF" func="fact">x!</button>
                <button id="calcBtn" >%</button>
                <button id="mathF" class="orangeRedBtn colorWhite calcMobile" func="backspc">CE</button>
                <button id="mathF" class="redBtn colorWhite calcMobile" func="clearScreen">AC</button>
            </div>

            <div >
                <button id="mathF" func="sin">sin</button>
                <button id="mathF" func="pi">π</button>

            </div>

            <div >
                <button id="mathF" func="cos">cos</button>
                <button id="mathF" func="log">log</button>
            </div>

            <div >
                <button id="mathF" func="tan">tan</button>
                <button id="mathF" func="sqrt">√</button>

            </div>

            <div >
                <button id="mathF" func="e">e</button>
                <button id="mathF" func="pow">x <span class="textPower">2</span>
                </button>
            </div>
      </div>
      <input type="radio" id="calc123" name="calcGroup" class="none">

      <div class="halfCalculator" id="halfCalculator1">
            <div>
          <button id="calcBtn" >(</button>
                <button id="calcBtn" >)</button>
                <button id="mathF" func="backspc"  class="orangeRedBtn colorWhite">CE</button>
                <button id="mathF" func="clearScreen" class="redBtn colorWhite">AC</button>
  </div>
  <div>
  <button id="calcBtn" >7</button>
                <button id="calcBtn" >8</button>
                <button id="calcBtn" >9</button>
                <button id="calcBtn" >÷</button>
  </div>
  <div>
  <button id="calcBtn" >4</button>
                <button id="calcBtn" >5</button>
                <button id="calcBtn" >6</button>
                <button id="calcBtn" >×</button>
  </div>
  <div>
  <button id="calcBtn" >1</button>
                <button id="calcBtn" >2</button>
                <button id="calcBtn" >3</button>
                <button id="calcBtn" >-</button></div>
                <div>
                <button id="calcBtn" >0</button>
                <button id="calcBtn" >.</button>
                <button id="mathF" func="mathCalc" class="greenBtn colorWhite">=</button>
                <button id="calcBtn" >+</button>
  </div>
          </div>
          <div class="calcMobile"> 
            <label for="calc123" class="borderRadiusLeft">123</label>
            <label for="calcFx" class="borderRadiusRight">Fx</label>
          </div>
    </div>


<script src="View/js/calc.js"></script>
