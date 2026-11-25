(function () {
  const { el, formatNumber, offer } = window.PriEcoWidgets;

  const FUNCS = {
    sin: Math.sin,
    cos: Math.cos,
    tan: Math.tan,
    asin: Math.asin,
    acos: Math.acos,
    atan: Math.atan,
    sqrt: Math.sqrt,
    abs: Math.abs,
    ln: Math.log,
    log: Math.log10,
    exp: Math.exp,
  };
  const CONSTS = { pi: Math.PI, e: Math.E };

  function tokenize(src) {
    const s = src
      .toLowerCase()
      .replace(/×/g, "*")
      .replace(/÷/g, "/")
      .replace(/−/g, "-")
      .replace(/π/g, "pi")
      .replace(/√/g, "sqrt")
      .replace(/,/g, ".")
      .replace(/\*\*/g, "^");
    const tokens = [];
    let i = 0;
    while (i < s.length) {
      const c = s[i];
      if (c === " ") {
        i++;
      } else if (/[0-9.]/.test(c)) {
        const m = s.slice(i).match(/^(\d+\.?\d*|\.\d+)(e[+-]?\d+)?/);
        if (!m) throw new Error("number");
        tokens.push({ t: "num", v: parseFloat(m[0]) });
        i += m[0].length;
      } else if (/[a-z]/.test(c)) {
        const m = s.slice(i).match(/^[a-z]+/)[0];
        if (m in FUNCS) tokens.push({ t: "fn", v: m });
        else if (m in CONSTS) tokens.push({ t: "num", v: CONSTS[m] });
        else throw new Error("name");
        i += m.length;
      } else if ("+-*/^()!%".includes(c)) {
        tokens.push({ t: c });
        i++;
      } else {
        throw new Error("char");
      }
    }
    return tokens;
  }

  function factorial(n) {
    if (n < 0 || !Number.isInteger(n) || n > 170) return NaN;
    let r = 1;
    for (let i = 2; i <= n; i++) r *= i;
    return r;
  }

  function evaluate(src) {
    const tokens = tokenize(src);
    let pos = 0;
    const peek = () => tokens[pos];
    const next = () => tokens[pos++];

    function expr() {
      let v = term();
      while (peek() && (peek().t === "+" || peek().t === "-")) {
        const op = next().t;
        const r = term();
        v = op === "+" ? v + r : v - r;
      }
      return v;
    }

    function term() {
      let v = unary();
      for (;;) {
        const p = peek();
        if (!p) break;
        if (p.t === "*" || p.t === "/") {
          next();
          const r = unary();
          v = p.t === "*" ? v * r : v / r;
        } else if (p.t === "num" || p.t === "fn" || p.t === "(") {
          v *= unary();
        } else {
          break;
        }
      }
      return v;
    }

    function unary() {
      const p = peek();
      if (p && (p.t === "-" || p.t === "+")) {
        next();
        const v = unary();
        return p.t === "-" ? -v : v;
      }
      return power();
    }

    function power() {
      const base = postfix();
      if (peek() && peek().t === "^") {
        next();
        return Math.pow(base, unary());
      }
      return base;
    }

    function postfix() {
      let v = primary();
      while (peek() && (peek().t === "!" || peek().t === "%")) {
        v = next().t === "!" ? factorial(v) : v / 100;
      }
      return v;
    }

    function primary() {
      const p = next();
      if (!p) throw new Error("end");
      if (p.t === "num") return p.v;
      if (p.t === "(") {
        const v = expr();
        if (peek() && peek().t === ")") next();
        return v;
      }
      if (p.t === "fn") return FUNCS[p.v](power());
      throw new Error("token");
    }

    const result = expr();
    if (pos !== tokens.length) throw new Error("trailing");
    return result;
  }

  function isMathExpression(q) {
    if (!/^[\d\s+\-*/^().,%!×÷−π√a-z]+$/i.test(q)) return false;
    if (!/\d/.test(q)) return false;
    if (!/[+\-*/^%!×÷−√]|\b(sin|cos|tan|sqrt|ln|log)\b/i.test(q)) return false;
    try {
      return isFinite(evaluate(q));
    } catch (_) {
      return false;
    }
  }

  function render(initial) {
    const screen = el("input", {
      class: "iw_calc_screen",
      type: "text",
      inputmode: "decimal",
      "aria-label": "Calculator input",
      autocomplete: "off",
      spellcheck: "false",
      placeholder: ":)",
    });

    let justEvaluated = false;

    function run() {
      const src = screen.value.trim();
      if (!src) return;
      let result;
      try {
        result = formatNumber(evaluate(src));
      } catch (_) {
        result = "Error";
      }
      screen.value = result === "Error" ? "" : result;
      screen.placeholder = result === "Error" ? "Error" : ":)";
      justEvaluated = true;
    }

    const INSERT = { "x!": "!", "x²": "^2", sin: "sin(", cos: "cos(", tan: "tan(", log: "log(", "√": "√(" };
    const CLASSES = { "=": "iw_calc_green", CE: "iw_calc_orange", AC: "iw_calc_red" };

    function half(rows, cls) {
      const box = el("div", { class: "iw_calc_half " + cls });
      for (const row of rows) {
        const line = el("div");
        for (const label of row) {
          const mobileOnly = label.endsWith("*");
          const text = mobileOnly ? label.slice(0, -1) : label;
          const btn = el("button", { type: "button", text });
          if (CLASSES[text]) btn.classList.add(CLASSES[text]);
          if (mobileOnly) btn.classList.add("iw_calc_mobile");
          btn.addEventListener("click", function () {
            screen.placeholder = ":)";
            if (text === "=") {
              run();
              return;
            }
            if (text === "AC") {
              screen.value = "";
            } else if (text === "CE") {
              screen.value = screen.value.slice(0, -1);
            } else {
              const insert = INSERT[text] || text;
              if (justEvaluated && !/^[\^!%÷×\-+]/.test(insert)) screen.value = "";
              screen.value += insert;
            }
            justEvaluated = false;
          });
          line.appendChild(btn);
        }
        box.appendChild(line);
      }
      return box;
    }

    const fx = half(
      [
        ["x!", "%", "CE*", "AC*"],
        ["sin", "π"],
        ["cos", "log"],
        ["tan", "√"],
        ["e", "x²"],
      ],
      "iw_calc_fx"
    );
    const num = half(
      [
        ["(", ")", "CE", "AC"],
        ["7", "8", "9", "÷"],
        ["4", "5", "6", "×"],
        ["1", "2", "3", "-"],
        ["0", ".", "=", "+"],
      ],
      "iw_calc_num"
    );

    const fxRadio = el("input", { type: "radio", id: "iw_calc_fx", name: "iw_calc_group", class: "iw_none" });
    const numRadio = el("input", {
      type: "radio",
      id: "iw_calc_123",
      name: "iw_calc_group",
      class: "iw_none",
      checked: "",
    });
    const toggle = el("div", { class: "iw_calc_toggle" }, [
      el("label", { for: "iw_calc_123", text: "123" }),
      el("label", { for: "iw_calc_fx", text: "Fx" }),
    ]);

    screen.addEventListener("keydown", function (e) {
      if (e.key === "Enter") {
        e.preventDefault();
        run();
      } else if (e.key === "Escape") {
        screen.value = "";
      }
    });
    screen.addEventListener("input", function () {
      justEvaluated = false;
      screen.placeholder = ":)";
    });

    if (initial) {
      screen.value = initial;
      run();
    }

    return el("div", { class: "iw_card iw_calc" }, [
      screen,
      el("div", { class: "iw_calc_body" }, [fxRadio, fx, numRadio, num, toggle]),
    ]);
  }

  offer(function (q, lower) {
    if (/^(calc|calculator|scientific calculator|online calculator)$/.test(lower)) return "";
    if (isMathExpression(q)) return q;
    return null;
  }, render);
})();
