<?php
if($ImpGoogle && !isset($_COOKIE['safe']) && !isset($_COOKIE['time'])){       
        $hashedQuery = hash('md5', $purl.$lang.$loc);

        if ($obj != '') {$results = gzcompress(encrypt(json_encode($obj), $purl.$lang.$loc, $purl.$lang.$loc, $cipher));}
        elseif (isset($g2obj)) {$results = gzcompress(encrypt(json_encode($g2obj), $purl.$lang.$loc, $purl.$lang.$loc, $cipher));}
        elseif (isset($QWantObj)) {$results = gzcompress(encrypt(json_encode($QWantObj), $purl.$lang.$loc, $purl.$lang.$loc, $cipher));}
        elseif (isset($BraveObj)) {$results = gzcompress(encrypt(json_encode($BraveObj), $purl.$lang.$loc, $purl.$lang.$loc, $cipher));}
        elseif (isset($MojeekObj)) {$results = gzcompress(encrypt(json_encode($MojeekObj), $purl.$lang.$loc, $purl.$lang.$loc, $cipher));}
        else{$stop = true;}
        if(!$stop){
                if(!file_exists('cache') || is_dir('cache')) {mkdir('cache');}
                file_put_contents('cache/'.$hashedQuery.'.txt' , date('Y-m-d'). '---'. $searchId . "\n" . $results);
        }
}