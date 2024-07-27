<?php
/*
header("Content-type: application/json; charset=utf-8");

$number = $_GET['ip'];

$stmt = $pdo->prepare("SELECT * FROM ip2loc WHERE :number BETWEEN f AND t");
$stmt->bindParam(':number', $number);
$stmt->execute();

$results = $stmt->fetchAll(PDO::FETCH_ASSOC);

echo json_encode($results);

$pdo = null;
*/