<html lang="en">

<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>PriEco | Map</title>

    <link rel="stylesheet" href="https://unpkg.com/leaflet@1.9.4/dist/leaflet.css"
        integrity="sha256-p4NxAoJBhIIN+hmNHrzRCf9tD/miZyoHS5obTRR9BMY=" crossorigin="" />
        <script src="https://unpkg.com/leaflet@1.9.4/dist/leaflet.js" integrity="sha256-20nQCchB9co0qIjJZRGuk2/Z9VM+kNiyxNV1lvTlZBo=" crossorigin=""></script>

</head>

<body>
    <div class="height100P width100P">
        <a class="absolute z-index999 ml-50 mt-20" href="/?q=<?php echo $_GET['q']?>"><button class="goBackMap mt-5 borderNone paddingCircle1215 borderRadius Pointer whiteAblackBg"><</button></a>
        <div class="searchBarMap" tabindex="0">
            <button onclick="searchLoc();" class="searchButton"><img alt="icnSearch" src="./View/icon/search.webp" class="width10 height10"></button>
            <input type="text" id="search-input" class="searchBox" placeholder="Search Map" value="<?php echo $_GET['q'] ?>">
            <div id="suggestions-container" class="autocom-boxMap autocom-box"></div>
        </div>
        <div id="map" class="height100P width100P"></div>
    </div>
    <?php
        $cssver = 55;
        include 'Model/style.php';
    ?>
    <script src="View/js/map.php"></script>

</body>

</html>