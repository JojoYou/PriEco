(function () {
  const { el, formatNumber, offer } = window.PriEcoWidgets;

  const UNITS = {
    Length: {
      mm: [0.001, "Millimeter", "millimetre|millimeters|millimetres"],
      cm: [0.01, "Centimeter", "centimetre|centimeters|centimetres"],
      m: [1, "Meter", "metre|meters|metres"],
      km: [1000, "Kilometer", "kilometre|kilometers|kilometres|kms"],
      in: [0.0254, "Inch", "inches|\""],
      ft: [0.3048, "Foot", "feet|'"],
      yd: [0.9144, "Yard", "yards"],
      mi: [1609.344, "Mile", "miles"],
      nmi: [1852, "Nautical mile", "nautical miles"],
    },
    Mass: {
      mg: [1e-6, "Milligram", "milligrams"],
      g: [0.001, "Gram", "grams|gramme"],
      kg: [1, "Kilogram", "kilograms|kilo|kilos|kgs"],
      t: [1000, "Tonne", "tonnes|ton|tons|metric ton|metric tons"],
      oz: [0.028349523125, "Ounce", "ounces"],
      lb: [0.45359237, "Pound", "pounds|lbs"],
      st: [6.35029318, "Stone", "stones"],
    },
    Volume: {
      ml: [0.001, "Milliliter", "millilitre|milliliters|millilitres"],
      cl: [0.01, "Centiliter", "centilitre|centiliters|centilitres"],
      dl: [0.1, "Deciliter", "decilitre|deciliters|decilitres"],
      l: [1, "Liter", "litre|liters|litres"],
      "m³": [1000, "Cubic meter", "m3|cubic meters|cubic metre|cubic metres"],
      tsp: [0.00492892159375, "Teaspoon", "teaspoons"],
      tbsp: [0.01478676478125, "Tablespoon", "tablespoons"],
      "fl oz": [0.0295735295625, "Fluid ounce", "floz|fluid ounces"],
      cup: [0.2365882365, "Cup", "cups"],
      pt: [0.473176473, "Pint", "pints"],
      qt: [0.946352946, "Quart", "quarts"],
      gal: [3.785411784, "Gallon", "gallons"],
    },
    Temperature: {
      "°C": [null, "Celsius", "c|centigrade|degc|ºc"],
      "°F": [null, "Fahrenheit", "f|degf|ºf"],
      K: [null, "Kelvin", "kelvins"],
    },
    Speed: {
      "m/s": [1, "Meter per second", "mps|meters per second|metres per second"],
      "km/h": [1 / 3.6, "Kilometer per hour", "kmh|kph|kmph|kilometers per hour|kilometres per hour"],
      mph: [0.44704, "Mile per hour", "miles per hour|mi/h"],
      kn: [1852 / 3600, "Knot", "knots|kt"],
      "ft/s": [0.3048, "Foot per second", "fps|feet per second"],
    },
    Time: {
      ms: [0.001, "Millisecond", "milliseconds"],
      s: [1, "Second", "sec|secs|seconds"],
      min: [60, "Minute", "mins|minutes"],
      h: [3600, "Hour", "hr|hrs|hours"],
      d: [86400, "Day", "days"],
      wk: [604800, "Week", "weeks"],
      mo: [2629800, "Month", "months"],
      yr: [31557600, "Year", "years"],
    },
    Data: {
      bit: [0.125, "Bit", "bits"],
      B: [1, "Byte", "bytes"],
      KB: [1e3, "Kilobyte", "kilobytes"],
      MB: [1e6, "Megabyte", "megabytes"],
      GB: [1e9, "Gigabyte", "gigabytes"],
      TB: [1e12, "Terabyte", "terabytes"],
      KiB: [1024, "Kibibyte", "kibibytes"],
      MiB: [1048576, "Mebibyte", "mebibytes"],
      GiB: [1073741824, "Gibibyte", "gibibytes"],
      TiB: [1099511627776, "Tebibyte", "tebibytes"],
    },
    Area: {
      "cm²": [1e-4, "Square centimeter", "cm2|sq cm|square centimeters"],
      "m²": [1, "Square meter", "m2|sq m|square meters|square metre|square metres"],
      "km²": [1e6, "Square kilometer", "km2|sq km|square kilometers"],
      ha: [1e4, "Hectare", "hectares"],
      ac: [4046.8564224, "Acre", "acres"],
      "in²": [0.00064516, "Square inch", "in2|sq in|square inches"],
      "ft²": [0.09290304, "Square foot", "ft2|sq ft|square feet"],
      "yd²": [0.83612736, "Square yard", "yd2|sq yd|square yards"],
      "mi²": [2589988.110336, "Square mile", "mi2|sq mi|square miles"],
    },
  };

  const ALIASES = new Map();
  for (const [category, units] of Object.entries(UNITS)) {
    for (const [key, [, name, aliases]] of Object.entries(units)) {
      const ref = { category, key };
      for (const a of [key, name, ...aliases.split("|")]) ALIASES.set(a.toLowerCase(), ref);
    }
  }

  function findUnit(raw) {
    let s = raw
      .trim()
      .toLowerCase()
      .replace(/^(a|an|one)\s+/, "")
      .replace(/^degrees?\s+/, "")
      .replace(/^°\s*/, "°")
      .replace(/\s+/g, " ");
    if (s.startsWith("°") && s.length === 2) s = s.slice(1);
    return ALIASES.get(s) || ALIASES.get(s.replace(/s$/, "")) || ALIASES.get(s.replace(/es$/, "")) || null;
  }

  function toBase(category, key, value) {
    if (category !== "Temperature") return value * UNITS[category][key][0];
    if (key === "°C") return value + 273.15;
    if (key === "°F") return ((value - 32) * 5) / 9 + 273.15;
    return value;
  }

  function fromBase(category, key, value) {
    if (category !== "Temperature") return value / UNITS[category][key][0];
    if (key === "°C") return value - 273.15;
    if (key === "°F") return ((value - 273.15) * 9) / 5 + 32;
    return value;
  }

  function parseConversion(q) {
    const m = q.match(/^(?:convert\s+)?(-?\d+(?:[.,]\d+)?(?:e[+-]?\d+)?)?\s*(.+?)\s+(?:to|in|into|as|=|->)\s+(.+?)\??$/i);
    if (!m) return null;
    const from = findUnit(m[2]);
    const to = findUnit(m[3]);
    if (!from || !to || from.category !== to.category || from.key === to.key) return null;
    const amount = m[1] ? parseFloat(m[1].replace(",", ".")) : 1;
    return { category: from.category, from: from.key, to: to.key, amount };
  }

  function render(state) {
    const category = el("select", { class: "iw_con_topic", "aria-label": "Category" });
    for (const name of Object.keys(UNITS)) category.appendChild(el("option", { value: name, text: name }));
    const amount = el("input", { type: "number", step: "any", "aria-label": "Amount" });
    const result = el("input", { type: "text", "aria-label": "Result", readonly: "" });
    const fromSel = el("select", { "aria-label": "From unit" });
    const toSel = el("select", { "aria-label": "To unit" });

    function fillUnits() {
      for (const sel of [fromSel, toSel]) {
        sel.textContent = "";
        for (const [key, [, name]] of Object.entries(UNITS[category.value])) {
          sel.appendChild(el("option", { value: key, text: name + " (" + key + ")" }));
        }
      }
    }

    function update() {
      const value = parseFloat(amount.value);
      if (isNaN(value)) {
        result.value = "";
        return;
      }
      const base = toBase(category.value, fromSel.value, value);
      result.value = formatNumber(fromBase(category.value, toSel.value, base));
    }

    category.value = state.category;
    fillUnits();
    fromSel.value = state.from;
    toSel.value = state.to;
    amount.value = String(state.amount);

    category.addEventListener("change", function () {
      fillUnits();
      fromSel.selectedIndex = 0;
      toSel.selectedIndex = Math.min(1, toSel.options.length - 1);
      update();
    });
    for (const node of [amount, fromSel, toSel]) node.addEventListener("input", update);
    update();

    return el("div", { class: "iw_card iw_unit" }, [
      category,
      el("div", { class: "iw_con_wrap" }, [
        el("div", { class: "iw_convert" }, [amount, fromSel]),
        el("div", { class: "iw_convert" }, [result, toSel]),
      ]),
    ]);
  }

  offer(function (q, lower) {
    const conversion = parseConversion(q);
    if (conversion) return conversion;
    if (/^(unit )?(converter|conversion|convert units?)$/.test(lower)) {
      return { category: "Length", from: "m", to: "ft", amount: 1 };
    }
    return null;
  }, render);
})();
