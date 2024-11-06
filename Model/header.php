<!--  SPDX-FileCopyrightText: 2022, 2022-2022 Roman  Láncoš <jojoyou@jojoyou.org> -->
<!-- -->
<!--  SPDX-License-Identifier: AGPL-3.0-or-later -->

<!DOCTYPE html>
<html lang="en">
<head>
  <title><?php if($purl != '' && $purl != null && !isset($_COOKIE['hQuery'])){echo str_replace('q=','',str_replace('&q=','',urldecode($purl))),' | PriEco';}else{echo'PriEco';}?></title>
  <meta name="description" content="PriEco, the Private, Secure and Ecofriendly search engine.">
  <meta charset="UTF-8">
  <meta http-equiv="X-UA-Compatible" content="IE=edge">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">

  <meta http-equiv="X-XSS-Protection" content="1; mode=block">

  <meta http-equiv="onion-location" content="http://priecovk7jsuh3tvkh62c6j4oep3l5bldigpzmay26rdpqz357t5dmad.onion/" />

  <meta name="msvalidate.01" content="F8A35372CFCA71418F60B9D549FFD676" />

  <link rel="icon" href="./favicon.ico?1">
  <?php
    $currentDomain = $_SERVER['HTTP_HOST'];
      if ($currentDomain === 'prieco.net') {
        echo '<link rel="search" type="application/opensearchdescription+xml" title="PriEco" href="osd.xml">';
      }
      else{
        echo '<link rel="search" type="application/opensearchdescription+xml" title="PriEco (.onion)" href= "http://priecovk7jsuh3tvkh62c6j4oep3l5bldigpzmay26rdpqz357t5dmad.onion/onion-osd.xml">';
      }
    ?>
  <link rel="manifest" crossorigin="anonymous" href="manifest.json">
</head>

<body>
