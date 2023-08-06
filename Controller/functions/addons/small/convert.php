<div class="output"style="margin-bottom: 15px;border-radius: 20px;padding: 10px;">
    <select id="conTopic">
      <option value="temp">Temperature</option>
    </select>
    <div style="display: flex;flex-wrap: wrap;">
    <div class="convert">
      <input type="number" id="convertInp" value="1">
      <select id="conInpUnit">
        <option value="c">Celsius</option>
        <option value="f">Fahrenheit</option>
        <option value="k">Kelvin</option>
      </select>
    </div>

    <div class="convert">
      <input type="number" id="convertOut" value="1">
      <select id="conOutUnit">
        <option value="c">Celsius</option>
        <option value="f">Fahrenheit</option>
        <option value="k">Kelvin</option>
      </select>
    </div>
  </div>
</div>
<script>
  let topi = document.getElementById('conTopic');

  let inp = document.getElementById('convertInp');
  let inpUnit = document.getElementById('conInpUnit');
  
  let out = document.getElementById('convertOut');
  let outUnit = document.getElementById('conOutUnit');

  function conv(topic,input, output, inUnit, ouUnit){
    let val = parseFloat(input.value.trim());
    if(isNaN(val)){return;}


    switch(topic){
        case "temp":
            val = (inUnit === "f") ? ((val - 32) * 5 / 9) :
            (inUnit === "k") ? (val - 273.15) :
            (inUnit === "r") ? ((val - 491.67) * 5 / 9) :
            val;

            val = (ouUnit === "f") ? ((val * 9 / 5) + 32) :
            (ouUnit === "k") ? (val + 273.15) :
            (ouUnit === "r") ? ((val * 9 / 5) + 491.67) :
            val;
            break;
    }

    output.value = val.toFixed(2);
  }
  inp.addEventListener("input", (event) => {
    conv(topi.options[topi.selectedIndex].value,inp, out, inpUnit.options[inpUnit.selectedIndex].value, outUnit.options[outUnit.selectedIndex].value);
});
out.addEventListener("input", (event) => {
    conv(topi.options[topi.selectedIndex].value,out, inp, outUnit.options[outUnit.selectedIndex].value, inpUnit.options[inpUnit.selectedIndex].value);
});

</script>