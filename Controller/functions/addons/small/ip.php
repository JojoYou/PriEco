<?php
 if (strpos(strtolower($purl), 'ip') !== false) {
    echo '<div class="output" style="margin-bottom: 15px;border-radius: 20px;padding: 10px;">
      <p style="text-align:center;border-bottom:solid gray 1px;">Your Public IP Address is: <b>',$_SERVER['REMOTE_ADDR'],'</b></p><br>
      <div style="display:flex;justify-content: space-between;flex-wrap: wrap;">
      <div>
      <p><b>Info</b></p>
     <p class="ipInfo">Country: ',$ipObj['country_name'],'</p>
     <p class="ipInfo">City: ',$ipObj['city'],'</p>
     <p class="ipInfo">Domain: ',$ipObj['country_tld'],'</p>
     <br>
     <p class="ipInfo">Version: ',$ipObj['version'],'</p>
     <p class="ipInfo">ISP: ',$ipObj['org'],'</p>
     </div>
     <img loading="lazy" decoding="async" src="https://maps.geoapify.com/v1/staticmap?style=osm-bright&width=400&height=150&zoom=8&apiKey=4983a79c24d749c2a19bca9a0e925a8f&center=lonlat:',$ipObj['longitude'],',',$ipObj['latitude'],'" style="border-radius: 20px;max-width:100%;">
     </div>
    </div>';
}