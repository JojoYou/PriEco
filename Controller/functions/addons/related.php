<?php

function related($rsArray)
{
    $rPrint = false;
    $i = 0;
    $related = '
<p class="sectionTitle">🔗 Related searches</p>
<div class="relSea output" id="output">';

    foreach ($rsArray as &$item) {
        if ($i == 0) {
            ++$i;
            continue;
        }
        if($i >=5){break;}

        $rPrint = true;
        if (!isset($_COOKIE['hQuery'])) {
            $related .= '<a href="?q=' . urlencode($item['name']) . '">';
        } else {
            $related .= '<form method="POST" action="">
                    <input type="hidden" name="q" value="' . $item['name'] . '">';
        }

        $related .= '<button class="relBtn">';
        if (!isset($_COOKIE['datasave'])) {
                $related .= '<img loading="lazy" src="../View/icon/search.webp" class="filterImage">';
        }
        $related .= '<p>' . $item['name'] . '</p></button>';

        if (!isset($_COOKIE['hQuery'])) {
            $related .= '</a>';
        } else {
            $related .= '</form>';
        }
        ++$i;
    }

    $related .= '</div>';
    if ($rPrint) {
        return $related;
    }
}