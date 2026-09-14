const fs = require("fs");
const path = require("path");

const JS_PATTERNS = [
  /\beval\s*\(/,
  /new\s+Function\s*\(/,
  /\bfetch\s*\(/,
  /XMLHttpRequest/,
  /WebSocket/,
  /\bimport\s*\(/,
  /\brequire\s*\(/,
  /document\.cookie/,
  /localStorage/,
  /sessionStorage/,
  /__TAURI__/,
  /__TAURI_INTERNALS__/,
];

const CSS_PATTERNS = [
  /@import/i,
  /expression\s*\(/i,
  /url\s*\(\s*['"]?\s*javascript:/i,
  /-moz-binding\s*:/i,
];

function walk(dir, out) {
  for (const name of fs.readdirSync(dir)) {
    const full = path.join(dir, name);
    const stat = fs.statSync(full);
    if (stat.isDirectory()) walk(full, out);
    else out.push(full);
  }
}

function scan(dir) {
  const files = [];
  walk(dir, files);
  const hits = [];
  for (const file of files) {
    const patterns = file.endsWith(".js") ? JS_PATTERNS : file.endsWith(".css") ? CSS_PATTERNS : null;
    if (!patterns) continue;
    const content = fs.readFileSync(file, "utf8");
    for (const re of patterns) {
      if (re.test(content)) hits.push(file + ": matches " + re);
    }
  }
  return hits;
}

const hits = scan(path.join(__dirname, "..", "marketplace"));
if (hits.length) {
  console.error("marketplace scan found disallowed patterns:");
  hits.forEach(function (h) {
    console.error("  " + h);
  });
  process.exit(1);
}
console.log("marketplace scan clean");
