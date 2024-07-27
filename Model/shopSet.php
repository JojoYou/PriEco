<?php

echo '<div class="imgtools">
<form method="post" action="">
<div class="scroll">
<div class="shopSettings">
<p>Price:</p>
<input value="',$_GET['shopMin'],'" name="shopPriceMin" type="number" min="0" placeholder="Min">
<input value="',$_GET['shopMax'],'" name="shopPriceMax" type="number" min="0" placeholder="Max">
</div>
 </div>
 <input class="imgSave imgtoolsOption" type="submit" name="shopToolsSave" value="Save">

 </form>
 </div>
 ';