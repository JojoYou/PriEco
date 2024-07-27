        ////
        //Create Map
        ////
        var map;
        function PriEcoMap(x, y, zoom) {
            if (!map) {
                // Initialize the map and marker
                map = L.map('map').setView([x, y], zoom);
                L.tileLayer('https://tile.openstreetmap.org/{z}/{x}/{y}.png', {
                    maxZoom: 19,
                    attribution: '&copy; <a href="http://www.openstreetmap.org/copyright">OpenStreetMap</a>'
                }).addTo(map);
                marker = L.marker([x, y]).addTo(map);
            } else {
                map.setView([x, y], zoom);
                marker.setLatLng([x, y]);
            }
        }
        ////
        //Search locations
        ////
        var inputBox = document.querySelector("#search-input");
        const suggBox = document.querySelector("#suggestions-container");

        inputBox.onfocus = () => {
            const autocompleteUrl = apiUrl + encodeURIComponent(inputBox.value);
                    fetch(autocompleteUrl)
                        .then(response => response.json())
                        .then(data => {
                            handleSuggestions(data);
                        })
                        .catch(error => {
                            console.error('Error:', error);
                        });        }
        inputBox.addEventListener("keypress", function (event) {
            if (event.keyCode === 13) {
                suggBox.style.visibility = "hidden";
                geocodeAddress(inputBox.value);
            }
        });
        inputBox.onblur = () => {
            setTimeout(() => {
                suggBox.style.visibility = "hidden";
            }, 200);

        };
        function searchLoc() {
            inputBox = document.querySelector("#search-input");
            suggBox.style.visibility = "hidden";
            geocodeAddress(inputBox.value);
        }



        const apiUrl = 'https://nominatim.openstreetmap.org/search?format=json&q=';

            // Function to handle autocomplete suggestions
            function handleSuggestions(data) {
                const suggestions = data;
                const suggestionsContainer = document.getElementById('suggestions-container');
                suggestionsContainer.innerHTML = '';
                suggestionsContainer.style.visibility="visible";
                suggestions.forEach(function (suggestion) {
                    const displayName = suggestion.display_name;
                    const suggestionElement = document.createElement('div');
                    suggestionElement.textContent = displayName;
                    suggestionElement.style = 'padding:5px; cursor:pointer; border-radius:20px;';
                    
                    suggestionElement.addEventListener('click', function () {
                        // Update input value with selected suggestion
                        document.getElementById('search-input').value = displayName;
                        // Perform geocoding request
                        geocodeAddress(displayName);
                        // Clear suggestions container
                        suggestionsContainer.innerHTML = '';
                        suggestionsContainer.style.visibility = "hidden";
                    });
                    suggestionsContainer.appendChild(suggestionElement);
                });
            }

            // Function to perform geocoding request
            function geocodeAddress(address) {
                const formattedAddress = encodeURIComponent(address);
                const geocodeUrl = apiUrl + formattedAddress;
                fetch(geocodeUrl)
                    .then(response => response.json())
                    .then(data => {
                        const latitude = data[0].lat;
                        const longitude = data[0].lon;
                        PriEcoMap(latitude, longitude, 18);
                    })
                    .catch(error => {
                        console.error('Error:', error);
                    });
            }

            document.getElementById('search-input').addEventListener('input', function () {
                const searchTerm = this.value;
                if (searchTerm.length >= 3) {
                    const autocompleteUrl = apiUrl + encodeURIComponent(searchTerm);
                    fetch(autocompleteUrl)
                        .then(response => response.json())
                        .then(data => {
                            handleSuggestions(data);
                        })
                        .catch(error => {
                            console.error('Error:', error);
                        });
                }
            });
        





        ////
        //Get cords from location
        ////
        function getCoordinates(location) {
            // Format the location string for the API request
            const formattedLocation = encodeURIComponent(location);

            // Prepare the API request URL
            const apiUrl = `https://nominatim.openstreetmap.org/search?q=${formattedLocation}&format=json`;

            // Send the API request and retrieve the response
            return fetch(apiUrl)
                .then(response => response.json())
                .then(data => {
                    // Parse the response and extract the latitude and longitude
                    const coordinates = {};
                    if (data && data.length > 0) {
                        coordinates.lat = data[0].lat;
                        coordinates.lon = data[0].lon;
                    }
                    return coordinates;
                })
                .catch(error => {
                    console.error('Failed to retrieve coordinates:', error);
                    return null;
                });
        }

        <?php 
        if(isset($_GET['q'])){
            echo 'geocodeAddress("',$_GET['q'],'");';
        }
        else{
            echo 'PriEcoMap(51.5073359, -0.12765, 2);';
        }
        
        ?>