<?php
function lyrics($data, $artist, $first_url){
if(isset($_GET['album'])){
  if(isset($data[0])){
  $album_cover= $data[0]['cover_art_url'];
  $album_name= $data[0]['name'];
  $data = $data[0]['songs'];
  }
  else{
  $album_cover= $data['cover_art_url'];
  $album_name= $data['name'];
  $data = $data['songs'];
  }
}

$parsedUrl = parse_url("http://$_SERVER[HTTP_HOST]$_SERVER[REQUEST_URI]");
parse_str($parsedUrl['query'] ?? '', $queryParams);
$isAlbum = isset($queryParams['album']) && $queryParams['album'] === 'true';
unset($queryParams['lyrics_artist']);
unset($queryParams['album']);
unset($queryParams['albums']);
unset($queryParams['songname']);
$baseUrl = str_replace('album%3A+', '', $parsedUrl['path'] . '?' . http_build_query($queryParams));

if(strpos($baseUrl, 'lyrics') === false){$baseUrl .= '+lyrics';}

  $lyrics = '<input type="checkbox" id="lyricsBtn" class="absolute expandBtn opacity0">';
  if(!isset($_GET['album'])){
  $lyrics .= '<div id="lyrics">'.renderLyrics($data[0], $data, $artist, $first_url, $baseUrl, $isAlbum, "", "", 0, 0).'</div>';
  }
  else{
  $lowercaseTitles = array_map('strtolower', array_column($data, 'title'));
  $pos = array_search(strtolower($_GET['songname']), $lowercaseTitles);
  if ($pos === false) {
    $pos = 0;
  }

  $i=0;
  if(is_array($data)){$count = count($data)-1;}
    foreach($data as $item){
      $lyrics .= '<div id="lyrics"><input type="radio" name="lyricsRadio" id="lyricsRadio'.$i.'" class="lyricsRadio none"';if($i == $pos){$lyrics.=' checked';}$lyrics.='><div class="lyricsBox">'.renderLyrics($item, $data, $artist, $first_url, $baseUrl, $isAlbum, $album_cover, $album_name, $count, $i).'</div></div>';
      ++$i;
      }
  }
  $lyrics .= expand();
  return $lyrics;
}

function renderLyrics($item, $data, $artist, $first_url, $baseUrl, $isAlbum, $album_cover, $album_name, $count, $z){
$lyrics = '
<div class="output width100P float-unset">
  <div class="flex wrap alignC">
    <div class="scroll">
    <label for="bigLyricsBtn'.$z.'">
      <img src="/Controller/functions/proxy.php?q='.$item['cover'].'" class="height60 width60 borderRadius10 Pointer">
    </label>
    <div class="ml-20 mr-20">
      <h3 class="txtNowrap">'.$item['title'].'</h3>
      <p class="txtNowrap">'.$item['artist'].'</p>
    </div>
    </div>
    <div class="scroll alignC">';
    $lyrics .= '<a href="' . $baseUrl . '" class="lyricsMenu clearLink'; if(!isset($_GET['album']) && !isset($_GET['lyrics_artist']) && !isset($_GET['albums'])){$lyrics.=' bgTop';} $lyrics.='" ' . (isset($_COOKIE['new']) ? 'target="_blank"' : '') . '>Lyrics</a><a href="';
    switch ($_COOKIE['lyrProvider']) {
        case 'y':
          $lyrics.='https://www.youtube.com/results?search_query=';
          break;
        case 'ym':
          $lyrics.='https://music.youtube.com/search?q=';
          break;
        case 'a':
          $lyrics.='https://music.apple.com/us/search?term=';
          break;
        case 's':
          $lyrics.='https://soundcloud.com/search?q=';
          break;
        default:
          $lyrics.='https://open.spotify.com/search/';
          break;
      }

    $lyrics .= $item['title']. ' ' . $item['artist'].'" class="lyricsMenu clearLink" ' . (isset($_COOKIE['new']) ? 'target="_blank"' : '') . '>Listen</a>';
    if((!empty($item['albums'][0]['album_id']) || isset($_GET['album']))){
      if(!is_array($item['albums'])){
        if(isset($_GET['albums'])){
          $lyrics.='<a href="' . $baseUrl . '&albums=true&songname='.urlencode($_GET['songname']).'" class="lyricsMenu clearLink'; if(isset($_GET['album'])){$lyrics.=' bgTop';} $lyrics.='" ' . (isset($_COOKIE['new']) ? 'target="_blank"' : '') . '>Albums</a>';
        }
        else{
        $lyrics.='<a href="' . $baseUrl . '" class="lyricsMenu clearLink'; if(isset($_GET['album'])){$lyrics.=' bgTop';} $lyrics.='" ' . (isset($_COOKIE['new']) ? 'target="_blank"' : '') . '>Album</a>';
        }
        }
      elseif(count($item['albums']) == 1){$lyrics.='<a href="' . $baseUrl . '&album='.$item['albums'][0]['album_id'].'&songname='.$item['title'].'" class="lyricsMenu clearLink'; if(isset($_GET['album'])){$lyrics.=' bgTop';} $lyrics.='" ' . (isset($_COOKIE['new']) ? 'target="_blank"' : '') . '>Album</a>';
    }
    elseif(count($item['albums']) > 1){$lyrics.='<a href="' . $baseUrl . '&albums=true&songname='.$item['title'].'" class="lyricsMenu clearLink'; if(isset($_GET['albums'])){$lyrics.=' bgTop';} $lyrics.='" ' . (isset($_COOKIE['new']) ? 'target="_blank"' : '') . '>Albums</a>';}
    }

    $lyrics.='<a href="' . $baseUrl . '&lyrics_artist='.urlencode($item['artist']).'" class="lyricsMenu clearLink'; if(isset($_GET['lyrics_artist'])){$lyrics.=' bgTop';} $lyrics.='" ' . (isset($_COOKIE['new']) ? 'target="_blank"' : '') . '>Artist</a>
    </div>';
      if(isset($data['header_image_url'])){
     $lyrics .= '<img src="/Controller/functions/proxy.php?q='.$data['header_image_url'].'" class="borderRadius imgCover">';
     }
    $lyrics .= '</div>';

$lyrics .='<div class="flex wrap '; if(isset($_GET['lyrics_artist']) || isset($_GET['albums']) || isset($_GET['album'])){$lyrics .= 'mflexCR';} $lyrics.='">
  <p class="lyricsTxt line-height24 txt16 paddingR10 ml-80D mt-10">
  '.str_replace(']','</span>', str_replace('[', '<span class="opacity7 txt14">', str_replace("\n",'<br>',$item['lyrics']))).'<br><br><span class="opacity7 txt14">Lyrics sourced from Genius.com. All rights to the lyrics are owned by their respective copyright holders.</span>
  </p>
  <div class="mt-20 mobile_width100P mr-10">';
  if(isset($_GET['lyrics_artist'])){
  $maxLength = 400;
    if (strlen($artist['description']) > $maxLength) {
     $lastPeriodPos = strrpos(substr($artist['description'], 0, $maxLength), '.');

     if ($lastPeriodPos === false) {
         $artist['description']= substr($artist['description'], 0, $maxLength).'<a href="'.$artist['url'].'" ';if (isset($_COOKIE['new'])) {$artist['description'] .= 'target="_blank"';}
         $artist['description'] .=' class="link"> Read more on genius</a>';
     }
     else{
     $artist['description'] = substr($artist['description'], 0, $lastPeriodPos + 1).'<a href="'.$artist['url'].'" ';if (isset($_COOKIE['new'])) {$artist['description'] .= 'target="_blank"';}
     $artist['description'] .=' class="link"> Read more on genius</a>';
     }
     }
    $lyrics .= '<div class="lyricsArtist">
    <img src="/Controller/functions/proxy.php?q='.$artist['header_image_url'].'" class="height150 imgCover borderRadiusTop">
    <div class="txt16 bgAnswer padding10 borderRadiusBottom">'.$artist['description'].'
    <div class="flex alignC mt-20">
      <img src="/Controller/functions/proxy.php?q='.$artist['image_url'].'" class="borderRadius100P height30">
      <h4 class="paddingL10 txt16">'.$artist['name'].'</h4>
    </div>
    </div>

    </div>';
  }
  if(isset($data[1]) && !isset($_GET['lyrics_artist'])){
  if(isset($_GET['album'])){$lyrics.='<div class="flex mb-15 alignC justConSpace paddingR20"><div class="flex alignC width100P">
   <img src="/Controller/functions/proxy.php?q='.$album_cover.'" class="height30 borderRadius100P">
   <div class="ml-10 txt16">
     <b><p>'.$album_name.'</p></b>
   </div>
   </div>
   </div>
     <ol class="paddingL30">';
   for ($i = 0; $i <= $count; $i++) {
     if(!isset($data[$i])){continue;}
     if(isset($_GET['album'])){;
            $lyrics .= '<li class="ml-10"><label class="Pointer no-highlight" for="lyricsRadio'.$i.'">
       <div class="flex mb-15 alignC justConSpace paddingR20">';
         $lyrics .= '<div class="flex width100P">
         <div class="txt16">
           <p>'.(empty($data[$i]['title']) ? 'Coming soon' : $data[$i]['title']).'</p>
         </div>
         </div>
         </div></label></li>';
           }
   }
   $lyrics .='</ol>';
   }
   elseif($first_url != "" && !isset($_GET['lyrics_id']) && !isset($_GET['album']) && !isset($_GET['albums'])){
    $first_url = explode(' – ', explode('<--->', $first_url)[1]);
    $first_url[1] = substr(str_replace('Lyrics', '', str_replace('Lyrics | Genius Lyrics', '',  str_replace('Lyrics - Genius', '' ,$first_url[1]))), 0, -1);

    if($item['title'] != $first_url[1] || $item['artist'] != $first_url[0]){
    $lyrics .= '<p class="mb-10">... or did you mean ...</p>
    <a href="./?q=song: '.$first_url[1].' artist: '.$first_url[0].'" class="clearLink"';if (isset($_COOKIE['new'])) {
        $lyrics .= 'target="_blank"';
    }

    $lyrics .='>
      <div class="flex mb-15 alignC justConSpace paddingR20">
        <div class="flex width100P">
          <div class="ml-10 txt16">
            <b><p>'.$first_url[1].'</p></b>
            <p>'.$first_url[0].'</p>
          </div>
        </div>
        <img src="View/icon/dropdown.svg" class="filterImage width15 rotate270 ml-20">
      </div>
    </a>';
    }
    }
    elseif(isset($_GET['albums'])){
    $lyrics .= '<a href="./?q=song: '.$first_url[1].' artist: '.$first_url[0].'" class="clearLink"';if (isset($_COOKIE['new'])) {
        $lyrics .= 'target="_blank"';
    }
    $lyrics .='>';
    foreach($item['albums'] as $al){
      $lyrics .= '<a href="'.$baseUrl.'&albums=true&album='.$al['album_id'].'&songname='.urlencode($_GET['songname']).'" class="clearLink">
      <div class="flex mb-15 alignC justConSpace paddingR20">
        <div class="flex width100P alignC">
          <img src="/Controller/functions/proxy.php?q='.$al['album_cover_art_url'].'" class="height40 borderRadius10">
          <div class="ml-10 txt16">
          <p>'.$al['album_name'].'</p>
          </div>
        </div>
        </div>
    </a>';
    }
    }
    }
  $lyrics .= '</div></div>
</div>

<input type="checkbox" id="bigLyricsBtn'.$z.'" class="bigLyricsBtn expandBtn none">
<div class="bigLyrics">
  <img src="/Controller/functions/proxy.php?q='.$item['cover'].'" class="bgBigLyrics width100P height100P fixed z-index-1">
  <div class="bigLyricsTitle">
    <label for="bigLyricsBtn'.$z.'" class="bigLyricsClose">
      <img src="/Controller/functions/proxy.php?q='.$item['cover'].'" class="height60 borderRadius10 Pointer">
    </label>
    <div class="ml-20 mr-20">
      <h3 class="txtNowrap">'.$item['title'].'</h3>
      <p class="txtNowrap">'.$item['artist'].'</p>
    </div>
  </div>
  <b><p class="centerTxt txt32 scrollDown height100P flexDColumn padding10 paddingB200">
  '.str_replace(']','</span>', str_replace('[', '<span class="opacity7 txt14">', str_replace("\n",'<br>',$item['lyrics']))).'
  </p></b>
</div>';
return $lyrics;
}

function expand(){
  return '<label id="expandExpand" for="lyricsBtn" class="resTopImgs flex Pointer width80V">
  <span class="expandLines"></span>
  <p class="flex alignC">Expand Lyrics<img class="filterImage width16 ml-10 height15 opacity5" src="View/icon/dropdown.svg"></p>
  <span class="expandLines"></span></label>

  <label id="expandMinimize" for="lyricsBtn" class="resTopImgs flex Pointer width80V">
  <span class="expandLines"></span>
  <p class="flex alignC">Minimize Lyrics<img class="filterImage width16 ml-10 height15 opacity5 rotate180" src="View/icon/dropdown.svg"></p>
  <span class="expandLines"></span></label>';
}
