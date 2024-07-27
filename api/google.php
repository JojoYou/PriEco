<?php
$loc = isset($_GET['loc']) ? $_GET['loc'] : '';
$lang = isset($_GET['lang']) ? $_GET['lang'] : '';

if (!isset($_GET['q']) || $loc == '' || $lang == '') {
    exit;
}
header("Content-type: application/json; charset=utf-8");

$dev = false;
include '../Controller/database.php';

$name = filter_input(INPUT_GET, 'q', FILTER_SANITIZE_STRING);

$stmt = $pdo->prepare("SELECT * FROM googleCache WHERE MATCH (query) AGAINST (:name IN BOOLEAN MODE)");
$stmt->execute(['name' => $name]);

if ($stmt->rowCount() > 0) {
    while ($row = $stmt->fetch(PDO::FETCH_ASSOC)) {
        if (strtolower($name) == strtolower($row['query']) && strtolower($loc) == strtolower($row['loc']) && strtolower($lang) == strtolower($row['lang'])) {
            $tmp = json_encode($row);
            break;
        }
    }
}
print_r($tmp);