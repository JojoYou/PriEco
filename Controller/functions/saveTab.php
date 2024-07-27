<?php

$tabs = explode(',',$_COOKIE['tabs']);
$tabs[$_GET['tab']]=urlencode($_GET['url']);
setcookie('tabs', implode(',', $tabs), time() + 31536000, '/');
header('Location: https://prieco.net/dev/proxy/?url='.$_GET['url'], true);
exit();