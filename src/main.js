(function () {
  "use strict";

  function dlog(level, message) {
    if (window.LumaDebugLog) window.LumaDebugLog.record(level, message);
  }

  var NAV_TRANSITION_MS = 280;

  var mainReadySignaled = false;

  function revealBody() {
    document.body.classList.add("luma-ready");
    var overlay = document.getElementById("page-transition-overlay");
    if (overlay) {
      overlay.classList.add("is-hidden");
      overlay.style.opacity = "0";
      setTimeout(function () {
        overlay.style.display = "none";
      }, NAV_TRANSITION_MS);
    }

    // Tells the Rust side it's safe to show the (until-now hidden) main
    // window - see show_main_window()'s "luma://main-ready" gate. Only the
    // main window is created hidden this way; the spotlight has its own
    // earlier "frontend-ready" signal above.
    if (!isSpotlight && !mainReadySignaled) {
      mainReadySignaled = true;
      var tauri = getTauriBridge();
      if (tauri) {
        try {
          tauri.event.emit("luma://main-ready", {});
        } catch (e) {
          dlog("warn", "emitting luma://main-ready failed: " + e);
        }
      }
    }
  }

  function navigateTo(url) {
    var overlay = document.getElementById("page-transition-overlay");
    if (!overlay) {
      window.location.href = url;
      return;
    }
    overlay.style.display = "block";
    overlay.classList.remove("is-hidden");
    void overlay.offsetWidth;
    overlay.style.opacity = "1";
    setTimeout(function () {
      window.location.href = url;
    }, NAV_TRANSITION_MS);
  }

  function tt(key, fallback) {
    return window.LumaI18n ? window.LumaI18n.t(key, fallback) : fallback;
  }
  function ff(key, params, fallback) {
    return window.LumaI18n ? window.LumaI18n.format(key, params, fallback) : fallback;
  }

  function getTauriBridge() {
    return window.__TAURI__ && window.__TAURI__.core ? window.__TAURI__ : null;
  }

  function showFatalBanner(message) {
    try {
      if (document.getElementById("luma-fatal-banner")) return;
      var el = document.createElement("div");
      el.id = "luma-fatal-banner";
      el.style.cssText =
        "position:fixed;top:0;left:0;right:0;z-index:99999;background:#4a0d0d;" +
        "color:#fff;font-family:monospace;font-size:12px;line-height:1.4;" +
        "padding:10px 16px;border-bottom:2px solid #ff4d4d;white-space:pre-wrap;";
      el.textContent = "LUMA: " + message;
      document.body.appendChild(el);
    } catch (e) {

    }
  }

  function showUpdateBanner(invoke) {
    try {
      if (document.getElementById("luma-update-banner")) return;

      var el = document.createElement("div");
      el.id = "luma-update-banner";
      el.style.cssText =
        "position:fixed;left:50%;bottom:22px;transform:translateX(-50%);z-index:9998;" +
        "display:flex;align-items:center;gap:12px;max-width:calc(100% - 40px);" +
        "background:rgba(18,10,28,.96);color:#fff;font-family:monospace;font-size:13px;" +
        "padding:10px 14px;border-radius:8px;border:1px solid rgba(207,89,230,.4);" +
        "box-shadow:0 10px 30px rgba(0,0,0,.5);";

      var text = document.createElement("span");
      text.textContent = tt("update_banner.new_version_available", "A new version of LUMA is available.");

      var updateBtn = document.createElement("button");
      updateBtn.type = "button";
      updateBtn.textContent = tt("update_banner.update_now", "Update now");
      updateBtn.style.cssText =
        "font-family:monospace;font-size:13px;padding:5px 12px;border-radius:5px;" +
        "border:1px solid rgba(207,89,230,.6);background:rgba(255,255,255,.06);" +
        "color:#fff;cursor:pointer;flex-shrink:0;";

      var laterBtn = document.createElement("button");
      laterBtn.type = "button";
      laterBtn.textContent = tt("update_banner.later", "Later");
      laterBtn.style.cssText =
        "font-family:monospace;font-size:12px;padding:5px 10px;border-radius:5px;" +
        "border:1px solid transparent;background:transparent;color:#c4c4c4;" +
        "cursor:pointer;opacity:.75;flex-shrink:0;";

      var bridge = getTauriBridge();
      if (bridge && bridge.event) {
        bridge.event.listen("luma://update-progress", function (event) {
          var stage = event.payload && event.payload.stage;
          if (stage === "downloading") text.textContent = tt("settings.updates.status_downloading", "Downloading the update…");
          else if (stage === "installing") text.textContent = tt("settings.updates.status_installing", "Installing - LUMA will restart itself…");
          else if (stage === "checking") text.textContent = tt("settings.updates.status_checking", "Checking the release…");
        });
      }

      updateBtn.addEventListener("click", function () {
        dlog("info", "update banner: user clicked Update now");
        text.textContent = tt("update_banner.starting", "Starting the update…");
        updateBtn.disabled = true;
        laterBtn.remove();
        invoke("apply_update").catch(function (err) {

          dlog("error", "apply_update invoke failed: " + err);
          text.textContent = ff("update_banner.failed", [err], "Update failed: {0}");
          updateBtn.disabled = false;
        });
      });

      laterBtn.addEventListener("click", function () {
        el.remove();
      });

      el.appendChild(text);
      el.appendChild(updateBtn);
      el.appendChild(laterBtn);
      document.body.appendChild(el);
    } catch (e) {
      dlog("warn", "showUpdateBanner failed: " + e);
    }
  }

  var params = new URLSearchParams(window.location.search);
  var isSpotlight = params.get("mode") === "spotlight";

  if (isSpotlight) {
    document.body.classList.add("spotlight-mode");

    document.documentElement.classList.add("spotlight-mode");
  }

  function restartAnimation(el, cls, otherCls) {
    if (otherCls) el.classList.remove(otherCls);
    el.classList.remove(cls);
    void el.offsetWidth;
    el.classList.add(cls);
  }

  function applyAnimationSetting(disabled) {
    document.body.classList.toggle("no-animations", !!disabled);
  }

  function applyBrandingSetting(show) {
    var el = document.getElementById("spotlight-brand");
    if (el) el.hidden = !show;
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

    if (isSpotlight) {
      try {
        tauri.event.emit("luma://frontend-ready", {});
      } catch (e) {
        dlog("warn", "emitting luma://frontend-ready failed: " + e);
      }
    }

    var settingsLink = document.getElementById("open-settings");
    if (settingsLink) {
      settingsLink.addEventListener("click", function (e) {
        e.preventDefault();
        navigateTo("settings.html");
      });
    }

    var config, engineConfig;
    try {
      config = await invoke("get_config");
      if (window.LumaDebugLog) window.LumaDebugLog.setEnabled(!!(config.general && config.general.debug_logging));
      dlog("info", "config loaded: browser_mode=" + config.general.browser_mode + " theme=" + config.appearance.theme);
    } catch (err) {
      dlog("error", "get_config invoke failed: " + err);
      config = { general: { default_engine: "Google" }, appearance: { theme: "luma-default", custom_css: "" } };
    }

    if (window.LumaI18n) {
      try {
        await window.LumaI18n.init(invoke, config.general && config.general.locale);
      } catch (err) {
        dlog("error", "LumaI18n.init failed: " + err);
      }
    }

    try {
      var themeCss = await invoke("get_theme_css", { themeId: config.appearance.theme });
      applyThemeCss(themeCss);
      dlog("info", "theme css applied (" + themeCss.length + " chars)");
    } catch (err) {
      dlog("error", "get_theme_css invoke failed: " + err);
    }
    applyCustomCss(config.appearance.custom_css);
    applyAnimationSetting(config.general && config.general.disable_animations);
    if (isSpotlight) applyBrandingSetting(config.general && config.general.show_spotlight_branding);

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

      placeholderOverride: isSpotlight ? tt("search.spotlight_placeholder", "search the universe") : null,
      onSearch: function (result) {

        if (result.local) {
          dlog("info", "search submitted -> local, engine=" + result.engine + " query=" + result.query);
          if (result.engine === "Open") {
            invoke("open_app", { query: result.query })
              .then(function () {
                dlog("info", "open_app invoke resolved OK");
              })
              .catch(function (err) {

                if (err === "__LUMA_APP_NOT_FOUND__") {
                  dlog("info", "open_app: \"" + result.query + "\" not found, opening file picker");
                  invoke("pick_app_for", { name: result.query }).catch(function (pickErr) {
                    dlog("error", "pick_app_for invoke failed: " + pickErr);
                  });
                } else {
                  dlog("error", "open_app invoke failed: " + err);
                }
              });
          } else {
            invoke("search_local", { query: result.query })
              .then(function () {
                dlog("info", "search_local invoke resolved OK");
              })
              .catch(function (err) {
                dlog("error", "search_local invoke failed: " + err);
              });
          }
          if (isSpotlight) invoke("hide_spotlight");
          return;
        }
        dlog("info", "search submitted -> resolved url=" + result.url + " engine=" + result.engine);
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
    revealBody();

    tauri.event.listen("luma://config-changed", async function () {
      try {
        var freshConfig = await invoke("get_config");
        if (window.LumaDebugLog) window.LumaDebugLog.setEnabled(!!(freshConfig.general && freshConfig.general.debug_logging));
        if (window.LumaI18n && freshConfig.general && freshConfig.general.locale !== window.LumaI18n.getCurrentLocale()) {
          await window.LumaI18n.init(invoke, freshConfig.general.locale);
        }
        var freshThemeCss = await invoke("get_theme_css", { themeId: freshConfig.appearance.theme });
        applyThemeCss(freshThemeCss);
        applyCustomCss(freshConfig.appearance.custom_css);
        applyAnimationSetting(freshConfig.general && freshConfig.general.disable_animations);
        if (isSpotlight) applyBrandingSetting(freshConfig.general && freshConfig.general.show_spotlight_branding);
        dlog("info", (isSpotlight ? "spotlight" : "main") + ": re-applied theme '" + freshConfig.appearance.theme + "' after config change");
      } catch (err) {
        dlog("error", "config-changed handler failed: " + err);
      }

      try {
        var freshEngines = await invoke("get_engines");
        if (freshConfig && freshConfig.general && freshConfig.general.default_engine) {
          freshEngines.defaultEngine = freshConfig.general.default_engine;
        }
        deck.reset(freshEngines);
        ui.reloadEngines();
        dlog("info", (isSpotlight ? "spotlight" : "main") + ": reloaded " + (freshEngines.engines || []).length + " engines after config change");
      } catch (err) {
        dlog("error", "reloading engines after config-changed failed: " + err);
      }
    });

    if (!isSpotlight && (!config.general || config.general.check_for_updates !== false)) {
      invoke("check_for_update")
        .then(function (status) {
          if (status.checked_ok && status.available) {
            dlog("info", "update available: " + status.current_commit + " -> " + status.latest_commit);
            showUpdateBanner(invoke);
          } else if (!status.checked_ok) {
            dlog("warn", "update check failed: " + status.error);
          } else {
            dlog("info", "no update available (running " + status.current_commit + ")");
          }
        })
        .catch(function (err) {
          dlog("warn", "check_for_update invoke failed: " + err);
        });
    }

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
        if (!document.body.classList.contains("no-animations")) {
          restartAnimation(document.body, "spotlight-fade-in", "spotlight-fade-out");
        }
      });

      tauri.event.listen("luma://spotlight-hiding", function () {
        if (!document.body.classList.contains("no-animations")) {
          restartAnimation(document.body, "spotlight-fade-out", "spotlight-fade-in");
        }
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
      revealBody();
      showFatalBanner(
        "internal bridge didn't start in this window - search/open/save won't work " +
          "right now. Try restarting LUMA. If it keeps happening, open Settings and " +
          "hold Shift+L, then send the debug log."
      );
      return;
    }

    if (attempt > 1) {
      dlog("info", "window.__TAURI__ became available on attempt " + attempt);
    }

    boot(tauri).catch(function (err) {
      dlog("error", "boot() threw: " + (err && err.message ? err.message : err));
      revealBody();
      showFatalBanner("failed to start up (" + (err && err.message ? err.message : err) + ") - open Settings and hold Shift+L for the debug log.");
    });
  }

  setTimeout(revealBody, 4000);

  init();
})();
