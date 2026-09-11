window.LumaDebugLog = (function () {
  "use strict";

  var KEY = "luma_debug_log_v1";
  var MAX = 500;

  function readLocal() {
    try {
      var raw = localStorage.getItem(KEY);
      return raw ? JSON.parse(raw) : [];
    } catch (e) {
      return [];
    }
  }

  function writeLocal(list) {
    try {
      localStorage.setItem(KEY, JSON.stringify(list.slice(-MAX)));
    } catch (e) {

    }
  }

  function record(level, message) {
    var entry = {
      ts_ms: Date.now(),
      level: level,
      source: "js",
      message: String(message),
    };

    var list = readLocal();
    list.push(entry);
    writeLocal(list);

    try {
      var fn = level === "error" ? "error" : level === "warn" ? "warn" : "log";
      console[fn]("luma: " + message);
    } catch (e) {

    }

    try {
      if (window.__TAURI__ && window.__TAURI__.core) {
        window.__TAURI__.core
          .invoke("log_client_event", { level: level, message: String(message) })
          .catch(function () {

          });
      }
    } catch (e) {

    }

    return entry;
  }

  function all() {
    return readLocal();
  }

  function clear() {
    writeLocal([]);
  }

  window.addEventListener("error", function (e) {
    record("error", "Uncaught error: " + (e && e.message ? e.message : e) +
      (e && e.filename ? " (" + e.filename + ":" + e.lineno + ")" : ""));
  });
  window.addEventListener("unhandledrejection", function (e) {
    var reason = e && e.reason;
    var msg = reason && reason.message ? reason.message : String(reason);
    record("error", "Unhandled promise rejection: " + msg);
  });

  return { record: record, all: all, clear: clear };
})();
