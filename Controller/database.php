<?php
//SPDX-FileCopyrightText: 2022, 2022-2022 Roman  Láncoš <jojoyou@jojoyou.org>
//
//SPDX-License-Identifier: AGPL-3.0-or-later


//Database login information
include_once 'envFunc.php';
use DevCoder\DotEnv;

(new DotEnv(__DIR__ . '/.env'))->load();
/*
if (!$dev) {
    if ($_ENV['HOST_NAME'] != "" or $_ENV['AUTH_NAME'] != "" or $_ENV['DATABASE_PASS'] != "" or $_ENV['DATABASE_NAME'] != "") {
        $dsn = 'mysql:host=' . $_ENV['HOST_NAME'] . ';dbname=' . $_ENV['DATABASE_NAME'] . ';charset=utf8mb4';
        $options = [
            PDO::ATTR_ERRMODE => PDO::ERRMODE_EXCEPTION,
            PDO::ATTR_DEFAULT_FETCH_MODE => PDO::FETCH_ASSOC,
            PDO::ATTR_EMULATE_PREPARES => false,
        ];
        $pdo = new PDO($dsn, $_ENV['AUTH_NAME'], $_ENV['DATABASE_PASS'], $options);
    }
}*/