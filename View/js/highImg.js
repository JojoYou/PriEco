const lazyImages = document.querySelectorAll("img[data-src]");

    const observer = new IntersectionObserver((entries, observer) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting) {
          const lazyImage = entry.target;
    
          lazyImage.src = lazyImage.dataset.src;
    
          lazyImage.removeAttribute("data-src");
    
          observer.unobserve(lazyImage);
    
          lazyImage.addEventListener("load", () => {
            lazyImage.classList.add("bigimage_loaded");
          });
        }
      });
    });
    
    // Start observing each lazy image
    lazyImages.forEach((lazyImage) => {
      observer.observe(lazyImage);
    });