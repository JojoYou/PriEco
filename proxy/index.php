<?php
    if(isset($_POST['proxySubmit'])){
        if(strpos($_POST['proxyURL'],'.') !== false){
            header("Location: ?url=". $_POST['proxyURL']);
            exit();
        }
        else{
            header("Location: /?q=". $_POST['proxyURL']);
            exit(); 
        }
    }
?>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Proxy | PriEco</title>
    <?php $beforePathStyle='../';include '../Model/style.php'; ?>    
</head>
<body class="">
    <div>
        <form method="POST" class="mt-10 width100V flex height40 justConC alignC">
        <button id="searchButton" class="mt-0 height100P searchButton" name="proxySubmit" ><img alt="icnSearch" src="../View/icon/search.webp" width="10" height="10"></button>
        <input class="mt-0 height100P searchBox" type="text" placeholder="https://prieco.net" value="<?php echo $_GET['url']; ?>" name="proxyURL">
        </form>
    </div>
<iframe class="borderNone mt-10 width100V height100V borderRadius" src="iframe.php/?url=<?php echo $_GET['url'] ?>"></iframe>
</body>
</html>