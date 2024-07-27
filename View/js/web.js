const text = document.querySelector(".defaultTitle");
const words = ["Google Chrome","Safari", "Firefox", "Microsoft Edge","Brave browser","Vivaldi","Opera","Tor browser","Waterfox"];
let wordIndex = 0;
let charIndex = 1;
let isDeleting = false;
const typeEffect = () => {
    const currentWord = words[wordIndex];
    const currentChar = currentWord.substring(0,charIndex);
    text.textContent = currentChar;

    if(!isDeleting && charIndex < currentWord.length){
        charIndex++;
        setTimeout(typeEffect, 200);
    }
    else if(isDeleting && charIndex > 0){
        charIndex--;
        setTimeout(typeEffect, 100);
    }
    else{
        isDeleting = !isDeleting;
        wordIndex = !isDeleting ? (wordIndex + 1) % words.length : wordIndex;
        setTimeout(typeEffect, 600);
    }
}
typeEffect();
