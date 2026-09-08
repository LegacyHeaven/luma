/**
 * Luma desktop app - shared bootstrap for both the normal "browser-style"
 * main window and the floating spotlight window (index.html is used for
 * both; ?mode=spotlight is what tells them apart).
 */
(function () {
  "use strict";

  function dlog(level, message) {
    if (window.LumaDebugLog) window.LumaDebugLog.record(level, message);
  }

  function getTauriBridge() {
    return window.__TAURI__ && window.__TAURI__.core ? window.__TAURI__ : null;
  }

  // If this never shows up, nothing that needs `invoke()` will work - the
  // search box, opening results, and (on the settings page) saving are
  // all IPC calls through this bridge. Rather than let the whole script
  // silently die on `tauri.core.invoke` if it's missing, wait a couple
  // seconds for it to appear (Tauri injects it before page scripts run,
  // so this should be instant, but this makes a real absence visible
  // instead of a blank nothing-happens) and put a message on screen if it
  // truly never does.
  function showFatalBanner(message) {
    try {
      if (document.getElementById("luma-fatal-banner")) return;
      var el = document.createElement("div");
      el.id = "luma-fatal-banner";
      el.style.cssText =
        "position:fixed;top:0;left:0;right:0;z-index:99999;background:#4a0d0d;" +
        "color:#fff;font-family:monospace;font-size:12px;line-height:1.4;" +
        "padding:10px 16px;border-bottom:2px solid #ff4d4d;white-space:pre-wrap;";
      el.textContent = "Luma: " + message;
      document.body.appendChild(el);
    } catch (e) {
      /* if even this fails, there's nothing more we can do client-side */
    }
  }

  var params = new URLSearchParams(window.location.search);
  var isSpotlight = params.get("mode") === "spotlight";

  if (isSpotlight) {
    document.body.classList.add("spotlight-mode");
  }

  function applyThemeCss(css) {
    var style = document.getElementById("luma-theme-style");
    if (!style) {
      style = document.createElement("style");
      style.id = "luma-theme-style";
      document.head.appendChild(style);
    }
    style.textContent = css;
  }

  function applyCustomCss(css) {
    var style = document.getElementById("luma-custom-style");
    if (!style) {
      style = document.createElement("style");
      style.id = "luma-custom-style";
      document.head.appendChild(style);
    }
    style.textContent = css || "";
  }

  async function boot(tauri) {
    var invoke = tauri.core.invoke;
    dlog("info", (isSpotlight ? "spotlight" : "main") + " window: boot() starting");

    var settingsLink = document.getElementById("open-settings");
    if (settingsLink) {
      settingsLink.addEventListener("click", function (e) {
        e.preventDefault();
        window.location.href = "settings.html";
      });
    }

    var config, engineConfig;
    try {
      config = await invoke("get_config");
      dlog("info", "config loaded: browser_mode=" + config.general.browser_mode + " theme=" + config.appearance.theme);
    } catch (err) {
      dlog("error", "get_config invoke failed: " + err);
      config = { general: { default_engine: "Google" }, appearance: { theme: "luma-default", custom_css: "" } };
    }

    try {
      var themeCss = await invoke("get_theme_css", { themeId: config.appearance.theme });
      applyThemeCss(themeCss);
      dlog("info", "theme css applied (" + themeCss.length + " chars)");
    } catch (err) {
      dlog("error", "get_theme_css invoke failed: " + err);
    }
    applyCustomCss(config.appearance.custom_css);

    try {
      engineConfig = await invoke("get_engines");
      dlog("info", "engines loaded: " + (engineConfig.engines || []).length + " engines");
    } catch (err) {
      dlog("error", "get_engines invoke failed: " + err);
      engineConfig = { defaultEngine: "Google", engines: [{ name: "Google", action: "https://www.google.com/search", param: "q", bang: "g" }] };
    }
    if (config.general && config.general.default_engine) {
      engineConfig.defaultEngine = config.general.default_engine;
    }

    var deck = new window.BangDeckModule.BangDeck(engineConfig);

    var ui = window.LumaUI.mount({
      deck: deck,
      particles: !isSpotlight,
      autofocus: true,
      onSearch: function (result) {
        dlog("info", "search submitted -> resolved url=" + result.url + " engine=" + (result.engine && result.engine.name));
        invoke("open_result", { url: result.url, fromSpotlight: isSpotlight })
          .then(function () {
            dlog("info", "open_result invoke resolved OK");
          })
          .catch(function (err) {
            dlog("error", "open_result invoke failed: " + err);
          });
      },
    });

    dlog("info", "UI mounted, ready for input");

    if (isSpotlight) {
      var closeOnBlur = !config.general || config.general.close_spotlight_on_blur !== false;

      document.addEventListener("keydown", function (e) {
        if (e.key === "Escape") {
          var input = document.getElementById("search-input");
          if (!input.value) {
            invoke("hide_spotlight");
          }
        }
      });

      tauri.event.listen("luma://spotlight-shown", function () {
        ui.clear();
        ui.focusInput();
      });

      if (closeOnBlur) {
        tauri.window.getCurrentWindow().onFocusChanged(function (event) {
          if (!event.payload) {
            invoke("hide_spotlight");
          }
        });
      }
    }
  }

  function init(attempt) {
    attempt = attempt || 1;
    var tauri = getTauriBridge();

    if (!tauri) {
      dlog("warn", "window.__TAURI__ not ready yet (attempt " + attempt + "/20)");
      if (attempt < 20) {
        setTimeout(function () { init(attempt + 1); }, 100);
        return;
      }
      dlog(
        "error",
        "window.__TAURI__ never became available after 20 attempts (2s) - " +
          "the Tauri IPC bridge did not initialize in this window. Search, " +
          "opening results, and settings-save will not work this launch."
      );
      showFatalBanner(
        "internal bridge didn't start in this window - search/open/save won't work " +
          "right now. Try restarting Luma. If it keeps happening, open Settings and " +
          "hold Shift+L, then send the debug log."
      );
      return;
    }

    if (attempt > 1) {
      dlog("info", "window.__TAURI__ became available on attempt " + attempt);
    }

    boot(tauri).catch(function (err) {
      dlog("error", "boot() threw: " + (err && err.message ? err.message : err));
      showFatalBanner("failed to start up (" + (err && err.message ? err.message : err) + ") - open Settings and hold Shift+L for the debug log.");
    });
  }

  init();
})();
