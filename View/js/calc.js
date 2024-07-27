var screen = document.querySelector('#screen');
    var btn = document.querySelectorAll('#calcBtn');

    screen.addEventListener('keydown', function(event) {
if (event.key === 'Enter') {
    screen.value=evaluate(screen.value).toFixed(2);
}
else if(event.key == 'Escape'){
screen.value="";
}
});

    for (item of btn) {
        item.addEventListener('click', (e) => {
            btntext = e.target.innerText;

            if (btntext == '×') {
                btntext = '*';
            }

            if (btntext == '÷') {
                btntext = '/';
            }
            screen.value += btntext;
        });
    }


    const buttons = document.querySelectorAll('#mathF');

buttons.forEach(button => {
  const buttonId = button.id;
  const buttonFunc = button.getAttribute('func');
  button.addEventListener('click', () => {
    if (buttonFunc) {
        window[buttonFunc]();
    }
  });
});

    function clearScreen() {
        screen.value='';
    }
    function mathCalc(){
        screen.value=evaluate(screen.value).toFixed(2);
    }
    function sin() {
        screen.value = Math.sin(screen.value).toFixed(2);
    }

    function cos() {
        screen.value = Math.cos(screen.value).toFixed(2);
    }

    function tan() {
        screen.value = Math.tan(screen.value).toFixed(2);
    }

    function pow() {
        screen.value = Math.pow(screen.value, 2).toFixed(2);
    }

    function sqrt() {
        screen.value = Math.sqrt(screen.value, 2).toFixed(2);
    }

    function log() {
        screen.value = Math.log(screen.value).toFixed(2);
    }

    function pi() {
        screen.value =screen.value + 3.14159265359;
    }

    function e() {
        screen.value =screen.value + 2.71828182846;
    }

    function fact() {
        var i, num, f;
        f = 1
        num = screen.value;
        for (i = 1; i <= num; i++) {
            f = f * i;
        }

        i = i - 1;

        screen.value = f.toFixed(2);
    }

    function backspc() {
        screen.value = screen.value.substr(0, screen.value.length - 1);
    }



    let parens = /\(([0-9+\-*/\^ .]+)\)/             // Regex for identifying parenthetical expressions
let exp = /(\d+(?:\.\d+)?) ?\^ ?(\d+(?:\.\d+)?)/ // Regex for identifying exponentials (x ^ y)
let mul = /(\d+(?:\.\d+)?) ?\* ?(\d+(?:\.\d+)?)/ // Regex for identifying multiplication (x * y)
let div = /(\d+(?:\.\d+)?) ?\/ ?(\d+(?:\.\d+)?)/ // Regex for identifying division (x / y)
let add = /(\d+(?:\.\d+)?) ?\+ ?(\d+(?:\.\d+)?)/ // Regex for identifying addition (x + y)
let sub = /(\d+(?:\.\d+)?) ?- ?(\d+(?:\.\d+)?)/  // Regex for identifying subtraction (x - y)

/**
 * Evaluates a numerical expression as a string and returns a Number
 * Follows standard PEMDAS operation ordering
 * @param {String} expr Numerical expression input
 * @returns {Number} Result of expression
 */
function evaluate(expr)
{
    if(isNaN(Number(expr)))
    {
        if(parens.test(expr))
        {
            let newExpr = expr.replace(parens, function(match, subExpr) {
                return evaluate(subExpr);
            });
            return evaluate(newExpr);
        }
        else if(exp.test(expr))
        {
            let newExpr = expr.replace(exp, function(match, base, pow) {
                return Math.pow(Number(base), Number(pow));
            });
            return evaluate(newExpr);
        }
        else if(mul.test(expr))
        {
            let newExpr = expr.replace(mul, function(match, a, b) {
                return Number(a) * Number(b);
            });
            return evaluate(newExpr);
        }
        else if(div.test(expr))
        {
            let newExpr = expr.replace(div, function(match, a, b) {
                if(b != 0)
                    return Number(a) / Number(b);
                else
                    throw new Error('Division by zero');
            });
            return evaluate(newExpr);
        }
        else if(add.test(expr))
        {
            let newExpr = expr.replace(add, function(match, a, b) {
                return Number(a) + Number(b);
            });
            return evaluate(newExpr);
        }
        else if(sub.test(expr))
        {
            let newExpr = expr.replace(sub, function(match, a, b) {
                return Number(a) - Number(b);
            });
            return evaluate(newExpr);
        }
        else
        {
            return expr;
        }
    }
    return Number(expr);
}