<?php
$imageUrl = $_GET['q'];

function resizeImage($imagePath, $width, $height = null)
{
    $image = imagecreatefromstring(file_get_contents($imagePath));

    if ($height === null) {
        $aspectRatio = imagesx($image) / imagesy($image);
        $height = $width / $aspectRatio;
    }

    $newImage = imagecreatetruecolor($width, $height);

    imagecopyresampled($newImage, $image, 0, 0, 0, 0, $width, $height, imagesx($image), imagesy($image));

    return $newImage;
}

$extension = pathinfo($imageUrl, PATHINFO_EXTENSION);

switch ($extension) {
    case 'jpg':
    case 'jpeg':
        header('Content-Type: image/jpeg');
        break;
    case 'png':
        header('Content-Type: image/png');
        break;
    case 'gif':
        header('Content-Type: image/gif');
        break;
    case 'svg':
        header('Content-Type: image/svg+xml');
        break;
    case 'webp':
        header('Content-Type: image/webp');
        break;
    default:
        header('Content-Type: text/plain');
        echo 'Invalid or unsupported image type';
        exit;
}

$resizedImage = resizeImage($imageUrl, 200);

switch ($extension) {
    case 'jpg':
    case 'jpeg':
        imagejpeg($resizedImage);
        break;
    case 'png':
        imagepng($resizedImage);
        break;
    case 'gif':
        imagegif($resizedImage);
        break;
    case 'svg':
        readfile($imageUrl);
        break;
    case 'webp':
        imagewebp($resizedImage);
        break;
}

imagedestroy($resizedImage);

?>
