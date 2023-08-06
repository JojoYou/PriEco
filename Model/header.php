<!--  SPDX-FileCopyrightText: 2022, 2022-2022 Roman  Láncoš <jojoyou@jojoyou.org> -->
<!-- -->
<!--  SPDX-License-Identifier: AGPL-3.0-or-later -->

<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta http-equiv="X-UA-Compatible" content="IE=edge">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <meta http-equiv="Cache-Control" content="no-cache">
  <meta http-equiv="Pragma" content="no-cache">
  <meta http-equiv="Expires" content="0">

  <title><?php if($purl != '' && $purl != null && !isset($_COOKIE['hQuery'])){echo str_replace('q=','',str_replace('&q=','',urldecode($purl))),' | PriEco';}else{echo'PriEco';}?></title>
  <meta name="description" content="PriEco, the Private, Secure and Ecofriendly search engine.">
  <link rel="icon" href="./favicon.ico?1">
  <link rel="search"
      type="application/opensearchdescription+xml"
      title="PriEco"
      href="osd.xml">
</head>

<body>