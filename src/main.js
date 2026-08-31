/**
 * Luma desktop app - shared bootstrap for both the normal "browser-style"
 * main window and the floating spotlight window (index.html is used for
 * both; ?mode=spotlight is what tells them apart).
 */
(function () {
  "use strict";

  var tauri = window.__TAURI__;
  var invoke = tauri.core.invoke;

  var params = new URLSearchParams(window.location.search);
  var isSpotlight = params.get("mode") === "spotlight";

  if (isSpotlight) {
    document.body.classList.add("spotlight-mode");
  }

  var settingsLink = document.getElementById("open-settings");
  if (settingsLink) {
    settingsLink.addEventListener("click", function (e) {
      e.preventDefault();
      window.location.href = "settings.html";
    });
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

  async function boot() {
    var config, engineConfig;
    try {
      config = await invoke("get_config");
    } catch (err) {
      console.error("luma: failed to load config", err);
      config = { general: { default_engine: "Google" }, appearance: { theme: "luma-default", custom_css: "" } };
    }

    try {
      var themeCss = await invoke("get_theme_css", { themeId: config.appearance.theme });
      applyThemeCss(themeCss);
    } catch (err) {
      console.error("luma: failed to load theme, staying unstyled", err);
    }
    applyCustomCss(config.appearance.custom_css);

    try {
      engineConfig = await invoke("get_engines");
    } catch (err) {
      console.error("luma: failed to load engines.json", err);
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
        invoke("open_result", { url: result.url, fromSpotlight: isSpotlight }).catch(function (err) {
          console.error("luma: open_result failed", err);
        });
      },
    });

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

  boot();
})();
