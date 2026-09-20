LumaPlugin.register({
  handle: function (bangWord, query) {
    var q = (query || "").trim();
    if (bangWord === "calc") {
      if (!q) return { text: "Type an expression, e.g. 12 * (4 + 1)" };
      try {
        return { text: formatNumber(evalExpr(q)) };
      } catch (e) {
        return { text: "Couldn't parse that expression." };
      }
    }
    if (bangWord === "convert") {
      if (!q) return { text: "e.g. 10 km to miles, or 98.6 f to c" };
      return { text: convert(q) };
    }
    return null;
  },
});

function evalExpr(input) {
  var pos = 0;

  function peek() {
    return input[pos];
  }
  function skipSpace() {
    while (pos < input.length && /\s/.test(input[pos])) pos++;
  }
  function parseNumber() {
    skipSpace();
    var start = pos;
    while (pos < input.length && /[0-9.]/.test(input[pos])) pos++;
    if (pos === start) throw new Error("expected number");
    return parseFloat(input.slice(start, pos));
  }
  function parseFactor() {
    skipSpace();
    if (peek() === "(") {
      pos++;
      var v = parseExpr();
      skipSpace();
      if (peek() !== ")") throw new Error("expected )");
      pos++;
      return v;
    }
    if (peek() === "-") {
      pos++;
      return -parseFactor();
    }
    if (peek() === "+") {
      pos++;
      return parseFactor();
    }
    return parseNumber();
  }
  function parsePower() {
    var base = parseFactor();
    skipSpace();
    if (peek() === "^") {
      pos++;
      return Math.pow(base, parsePower());
    }
    return base;
  }
  function parseTerm() {
    var v = parsePower();
    for (;;) {
      skipSpace();
      var op = peek();
      if (op === "*" || op === "/" || op === "%") {
        pos++;
        var rhs = parsePower();
        if (op === "*") v *= rhs;
        else if (op === "/") v /= rhs;
        else v %= rhs;
      } else {
        break;
      }
    }
    return v;
  }
  function parseExpr() {
    var v = parseTerm();
    for (;;) {
      skipSpace();
      var op = peek();
      if (op === "+" || op === "-") {
        pos++;
        var rhs = parseTerm();
        v = op === "+" ? v + rhs : v - rhs;
      } else {
        break;
      }
    }
    return v;
  }

  var result = parseExpr();
  skipSpace();
  if (pos !== input.length) throw new Error("unexpected trailing input");
  if (isNaN(result)) throw new Error("not a number");
  return result;
}

function formatNumber(n) {
  if (!isFinite(n)) return "Error";
  var rounded = Math.round(n * 1e6) / 1e6;
  return String(rounded);
}

function normalizeUnit(u) {
  return u.toLowerCase().replace(/[.\s]/g, "");
}

var UNIT_GROUPS = [
  {
    units: {
      mm: 0.001, millimeter: 0.001, millimeters: 0.001,
      cm: 0.01, centimeter: 0.01, centimeters: 0.01,
      m: 1, meter: 1, meters: 1, metre: 1, metres: 1,
      km: 1000, kilometer: 1000, kilometers: 1000, kilometre: 1000, kilometres: 1000,
      in: 0.0254, inch: 0.0254, inches: 0.0254,
      ft: 0.3048, foot: 0.3048, feet: 0.3048,
      yd: 0.9144, yard: 0.9144, yards: 0.9144,
      mi: 1609.344, mile: 1609.344, miles: 1609.344,
      nmi: 1852, nauticalmile: 1852, nauticalmiles: 1852,
    },
  },
  {
    units: {
      mg: 0.001, milligram: 0.001, milligrams: 0.001,
      g: 1, gram: 1, grams: 1,
      kg: 1000, kilogram: 1000, kilograms: 1000,
      oz: 28.349523125, ounce: 28.349523125, ounces: 28.349523125,
      lb: 453.59237, lbs: 453.59237, pound: 453.59237, pounds: 453.59237,
      st: 6350.29318, stone: 6350.29318, stones: 6350.29318,
      t: 1000000, tonne: 1000000, tonnes: 1000000, ton: 1000000,
    },
  },
  {
    units: {
      ml: 1, milliliter: 1, milliliters: 1, millilitre: 1, millilitres: 1,
      l: 1000, liter: 1000, liters: 1000, litre: 1000, litres: 1000,
      tsp: 4.92892, teaspoon: 4.92892, teaspoons: 4.92892,
      tbsp: 14.7868, tablespoon: 14.7868, tablespoons: 14.7868,
      floz: 29.5735,
      cup: 236.588, cups: 236.588,
      pt: 473.176, pint: 473.176, pints: 473.176,
      qt: 946.353, quart: 946.353, quarts: 946.353,
      gal: 3785.41, gallon: 3785.41, gallons: 3785.41,
    },
  },
  {
    units: {
      ms: 0.001, millisecond: 0.001, milliseconds: 0.001,
      s: 1, sec: 1, secs: 1, second: 1, seconds: 1,
      min: 60, mins: 60, minute: 60, minutes: 60,
      h: 3600, hr: 3600, hrs: 3600, hour: 3600, hours: 3600,
      day: 86400, days: 86400,
      week: 604800, weeks: 604800,
    },
  },
];

var TEMP_UNITS = { c: "c", celsius: "c", f: "f", fahrenheit: "f", k: "k", kelvin: "k" };

var CURRENCY_LIKE = [
  "usd", "eur", "gbp", "jpy", "cad", "aud", "chf", "cny", "inr",
  "dollar", "dollars", "euro", "euros", "yen", "rupee", "rupees",
];

function convertTemp(v, from, to) {
  if (from === to) return v;
  var celsius = from === "c" ? v : from === "f" ? ((v - 32) * 5) / 9 : v - 273.15;
  if (to === "c") return celsius;
  if (to === "f") return (celsius * 9) / 5 + 32;
  return celsius + 273.15;
}

function convert(query) {
  var m = query.match(/^\s*(-?[\d.]+)\s*([a-zA-Z]+)\s+(?:to|in|as)\s+([a-zA-Z]+)\s*$/);
  if (!m) return "Try: 10 km to miles";

  var value = parseFloat(m[1]);
  var fromRaw = normalizeUnit(m[2]);
  var toRaw = normalizeUnit(m[3]);

  if (TEMP_UNITS[fromRaw] && TEMP_UNITS[toRaw]) {
    var tempResult = convertTemp(value, TEMP_UNITS[fromRaw], TEMP_UNITS[toRaw]);
    return formatNumber(tempResult) + "°" + TEMP_UNITS[toRaw].toUpperCase();
  }

  for (var i = 0; i < UNIT_GROUPS.length; i++) {
    var group = UNIT_GROUPS[i];
    if (group.units[fromRaw] != null && group.units[toRaw] != null) {
      var base = value * group.units[fromRaw];
      var result = base / group.units[toRaw];
      return formatNumber(result) + " " + m[3];
    }
  }

  if (CURRENCY_LIKE.indexOf(fromRaw) !== -1 || CURRENCY_LIKE.indexOf(toRaw) !== -1) {
    return "Live currency rates need internet access, which plugins can't use. Try a physical unit instead (length, mass, volume, time, or temperature).";
  }

  return "Don't know how to convert " + m[2] + " to " + m[3] + ".";
}
