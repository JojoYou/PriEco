<?php
$dev = true;
include 'Model/style.php';
include 'Controller/database.php';

if(isset($_POST['fedsubmit'])){ 
    if(!empty($_POST['h-captcha-response'])){ 
        $verifyURL = 'https://hcaptcha.com/siteverify'; 
        $token = $_POST['h-captcha-response']; 
        $data = array( 
            'secret' => $_ENV['hCaptcha_Secret'], 
            'response' => $token, 
            'remoteip' => $_SERVER['REMOTE_ADDR'] 
        ); 
        $curlConfig = array( 
            CURLOPT_URL => $verifyURL, 
            CURLOPT_POST => true, 
            CURLOPT_RETURNTRANSFER => true, 
            CURLOPT_POSTFIELDS => $data 
        ); 
        $ch = curl_init(); 
        curl_setopt_array($ch, $curlConfig); 
        $response = curl_exec($ch); 
        curl_close($ch); 
        $responseData = json_decode($response); 
        if($responseData->success){ 
            $subject = 'PriEco Feedback';
            $body = 'Email: ' . $_POST['fedEmail'] . "\nFeedback: " . $_POST['fedsug'];
            $sender = "roman@send.prieco.net";
            $recipient = "team@jojoyou.org";
            $headers = "From: $sender";

            if (mail($recipient, $subject, $body, $headers)) {
            }
              
            echo '<h1>Thank you for your feedback!';
            header('refresh:2;url=/');
                exit();
        }
}
}
?>

<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta http-equiv="X-UA-Compatible" content="IE=edge">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Feedback | PriEco</title>
</head>
<body>
<div class="width100V height100V flex justConC alignC">
<form method="POST" action="" class="padding20 curve">
<h4 class="txt32">Thank you for helping improve PriEco</h4>
<input type="text" name="fedEmail" placeholder="Email" class="fedimp curve padding10 width100P height50 borderNone">
<p class="txt12"><i><b>Not required</b> just if you want us to contact you back</i></p>
<br><textarea name="fedsug" class="fedimp curve borderNone padding10 width100P min-height100 mb-10" placeholder="Feedback*" required></textarea>

<?php
 echo '<div class="h-captcha" data-sitekey="',$_ENV['hCaptcha_Site'],'"></div>';
if(isset($_POST['fedsubmit'])){ 
 if(empty($_POST['h-captcha-response'])){ 
     echo '<p class="colorRed">Fill up hCaptcha!</p>';
 }
}
?>
<div class="centerTxt">
<input type="submit" name="fedsubmit" value="Submit" class="padding10 curve Pointer inline mt-10 borderNone whiteAblackBg">
</form>
</div>
<br>
  </div>

  <script src="https://hcaptcha.com/1/api.js" async defer></script>

</body>
</html>