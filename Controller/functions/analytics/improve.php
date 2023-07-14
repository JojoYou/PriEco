<?php
if($ImpProfiles){
    $twitter='';
    $facebook='';
    $imdb='';
    $tomato='';
    $spotify='';
    $apple='';

    if(isset($ddgObj['Infobox']['content'])){
    foreach($ddgObj['Infobox']['content'] as &$item){
        if($item['data_type'] == 'twitter_profile'){
            $twitter = $item['value'];
        }
        if($item['data_type'] == 'facebook_profile'){
                $facebook = $item['value'];
        }   
        if($item['data_type'] == 'imdb_id'){
                $imdb = $item['value'];
        }   
        if($item['data_type'] == 'rotten_tomatoes'){
                $tomato = $item['value'];
        }   
        if($item['data_type'] == 'spotify_artist_id'){
                $spotify = $item['value'];
        }
        if($item['data_type'] == 'itunes_artist_id'){
                $apple = $item['value'];
        }
    }

    $conn->query("INSERT INTO `profiles` (`Name`, `Twitter`, `Facebook`,`IMDb`,`Tomatoes`,`Spotify`,`Apple`) VALUES ('$name', '$twitter', '$facebook', '$imdb','$tomato','$spotify','$apple');");
  }
}

if((($obj != '' && str_starts_with(json_encode($obj), '{"kind":"'))or isset($g2obj)) && $ImpGoogle && !isset($_COOKIE['safe']) && !isset($_COOKIE['time'])){

$purl_escaped = strtolower(mysqli_real_escape_string($conn, $purl));

if(!isset($g2obj)){$gCache = json_encode($obj);}
else{$gCache = json_encode($g2obj);}
$gCache_escaped = mysqli_real_escape_string($conn, $gCache);     

if(isset($_COOKIE['Language'])){$lang_escaped = $_COOKIE['Language'];}
else{$lang_escaped = 'all';}
if(isset($_COOKIE['Location'])){$loc_escaped = $_COOKIE['Location'];}
else{$loc_escaped = 'all';}

if(!isset($g2obj)){
        $sql = "INSERT INTO `googleCache`(`query`, `results`, `lang`, `loc`, `count`, `official`) VALUES ('$purl_escaped','$gCache_escaped', '$lang_escaped', '$loc_escaped', 1, 1);";
}
else{
        $sql = "INSERT INTO `googleCache`(`query`, `results`, `lang`, `loc`, `count`, `official`) VALUES ('$purl_escaped','$gCache_escaped', '$lang_escaped', '$loc_escaped', 1, 0);";
}
$conn->query($sql);

}
elseif(!$ImpGoogle && !isset($_COOKIE['safe']) && !isset($_COOKIE['time'])){
        $purl_escaped = mysqli_real_escape_string($conn, $purl);
        $conn->query("UPDATE `googleCache` SET `count` = `count` + 1 WHERE `query` = '$purl_escaped'");
}

if($simImg != ''){
        $purl_escaped = mysqli_real_escape_string($conn, $purl);
        $simI_escaped = mysqli_real_escape_string($conn, $simImg);
        $conn->query("UPDATE `suggestions` SET `img` = '$simI_escaped' WHERE `name` = '$purl_escaped'");   
}