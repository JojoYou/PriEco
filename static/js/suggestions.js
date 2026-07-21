document.addEventListener("DOMContentLoaded", () => {
  const searchInputs = document.querySelectorAll("#search_box");

  searchInputs.forEach((input) => {
    const wrapper = input.closest(".search_input_wrapper");
    if (!wrapper) return;

    const container = wrapper.querySelector(".suggestions_container");
    const ghostInput = wrapper.querySelector(".ghost_box");
    const form = input.closest("form");
    let debounceTimer;
    let currentFocus = -1;

    const toggleActiveStyles = (isActive) => {
      if (isActive) {
        form.classList.add("suggestions-active");
        container.style.display = "block";
      } else {
        form.classList.remove("suggestions-active");
        container.style.display = "none";
        currentFocus = -1;
      }
    };

    const fetchAndRender = async (query) => {
      try {
        const ddgUrl = `https://duckduckgo.com/ac/?q=${encodeURIComponent(query)}&kl=wt-wt`;
        const proxyUrl = `/proxy?u=${encodeURIComponent(ddgUrl)}`;

        const response = await fetch(proxyUrl);
        if (!response.ok) throw new Error("Proxy network error");

        const data = await response.json();
        container.innerHTML = "";
        currentFocus = -1;
        ghostInput.textContent = "";

        if (data && data.length > 0) {
          // --- GHOST TEXT LOGIC ---
          const firstSuggestion = data[0].phrase.toLowerCase();
          const userQuery = input.value.toLowerCase();

          if (firstSuggestion.startsWith(userQuery) && userQuery.length > 0) {
            const typedPart = input.value;
            const remainderPart = data[0].phrase.substring(typedPart.length);

            ghostInput.innerHTML = "";
            const invisibleSpan = document.createElement("span");
            invisibleSpan.style.color = "transparent";
            invisibleSpan.textContent = typedPart;

            ghostInput.appendChild(invisibleSpan);
            ghostInput.appendChild(document.createTextNode(remainderPart));
          }
          // ------------------------

          data.forEach((item) => {
            const div = document.createElement("div");
            div.className = "suggestion_item";
            div.textContent = item.phrase;

            div.addEventListener("click", () => {
              input.value = item.phrase;
              ghostInput.textContent = "";
              toggleActiveStyles(false);
              form.submit();
            });

            container.appendChild(div);
          });
          toggleActiveStyles(true);
        } else {
          toggleActiveStyles(false);
        }
      } catch (error) {
        console.error("Failed to fetch suggestions:", error);
      }
    };

    input.addEventListener("input", (e) => {
      const query = e.target.value;
      clearTimeout(debounceTimer);

      if (query.trim().length < 2) {
        toggleActiveStyles(false);
        ghostInput.textContent = "";
        return;
      }

      if (
        ghostInput.textContent &&
        !ghostInput.textContent.toLowerCase().startsWith(query.toLowerCase())
      ) {
        ghostInput.textContent = "";
      }

      debounceTimer = setTimeout(() => {
        fetchAndRender(query.trim());
      }, 300);
    });

    input.addEventListener("keydown", (e) => {
      let items = container.querySelectorAll(".suggestion_item");

      if (!items || items.length === 0 || container.style.display === "none") {
        return;
      }

      if (e.key === "ArrowDown") {
        e.preventDefault();
        currentFocus++;
        addActive(items);
        ghostInput.textContent = "";
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        currentFocus--;
        addActive(items);
        ghostInput.textContent = "";
      } else if (e.key === "Enter") {
        if (currentFocus > -1) {
          e.preventDefault();
          items[currentFocus].click();
        }
      } else if (e.key === "Tab") {
        if (currentFocus > -1) {
          e.preventDefault();
          const newQuery = items[currentFocus].textContent;
          input.value = newQuery;
          clearTimeout(debounceTimer);
          fetchAndRender(newQuery);
        } else if (ghostInput.textContent) {
          e.preventDefault();
          input.value = ghostInput.textContent;
          ghostInput.textContent = "";
          clearTimeout(debounceTimer);
          fetchAndRender(input.value);
        }
      }
    });

    function addActive(items) {
      if (!items) return;
      removeActive(items);
      if (currentFocus >= items.length) currentFocus = 0;
      if (currentFocus < 0) currentFocus = items.length - 1;
      items[currentFocus].classList.add("active");
      items[currentFocus].scrollIntoView({ block: "nearest" });
    }

    function removeActive(items) {
      items.forEach((item) => item.classList.remove("active"));
    }

    document.addEventListener("click", (e) => {
      if (!wrapper.contains(e.target)) {
        toggleActiveStyles(false);
      }
    });

    // UPDATED: Re-show suggestions on focus, or fetch them if they don't exist yet
    input.addEventListener("focus", () => {
      const query = input.value.trim();

      if (query.length >= 2) {
        if (container.children.length > 0) {
          toggleActiveStyles(true);
        } else {
          fetchAndRender(query);
        }
      }
    });
  });
});
