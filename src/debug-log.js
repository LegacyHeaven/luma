/**
 * Luma desktop app - shared client-side debug log, used by both main.js
 * (main window + spotlight) and settings.js (the debug console UI).
 *
 * Why this exists: Luma's release build has no console attached (see
 * `windows_subsystem = "windows"` in src-tauri/src/main.rs), so a plain
 * `console.error` is invisible to a user who isn't running Luma from a
 * terminal. Every notable frontend event gets recorded here, which:
 *  - always writes to localStorage (shared across index.html/settings.html
 *    since they're the same origin) - this keeps working even if the
 *    Tauri IPC bridge itself is broken, which is exactly the scenario
 *    this is most useful for diagnosing.
 *  - best-effort also forwards to the Rust-side ring buffer via
 *    `log_client_event`, so it shows up merged with backend events and
 *    live-pushed to any open debug console, even one in a different
 *    window than the one that logged it.
 */
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
      // localStorage unavailable/full - nothing else we can do, the
      // console.* call in record() below still happened.
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
      /* no console in this context - fine, we already persisted it */
    }

    try {
      if (window.__TAURI__ && window.__TAURI__.core) {
        window.__TAURI__.core
          .invoke("log_client_event", { level: level, message: String(message) })
          .catch(function () {
            /* invoke itself failing is exactly the kind of thing we're
               trying to catch - the localStorage copy above still has it. */
          });
      }
    } catch (e) {
      /* ignore - same reasoning as above */
    }

    return entry;
  }

  function all() {
    return readLocal();
  }

  function clear() {
    writeLocal([]);
  }

  // Catch anything that slips past every try/catch elsewhere - an
  // uncaught exception or unhandled promise rejection anywhere in the app
  // still leaves a trace instead of just silently doing nothing.
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
