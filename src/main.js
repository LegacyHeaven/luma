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
      el.textContent = "LUMA: " + message;
      document.body.appendChild(el);
    } catch (e) {
      /* if even this fails, there's nothing more we can do client-side */
    }
  }

  // A small "Update available" toast for the main window - see
  // check_for_update/apply_update in src-tauri/src/updater.rs. Styled
  // inline rather than through theme.css since this is chrome, not
  // themable page content (same reasoning as showFatalBanner above).
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
      text.textContent = "A new version of LUMA is available.";

      var updateBtn = document.createElement("button");
      updateBtn.type = "button";
      updateBtn.textContent = "Update now";
      updateBtn.style.cssText =
        "font-family:monospace;font-size:13px;padding:5px 12px;border-radius:5px;" +
        "border:1px solid rgba(207,89,230,.6);background:rgba(255,255,255,.06);" +
        "color:#fff;cursor:pointer;flex-shrink:0;";

      var laterBtn = document.createElement("button");
      laterBtn.type = "button";
      laterBtn.textContent = "Later";
      laterBtn.style.cssText =
        "font-family:monospace;font-size:12px;padding:5px 10px;border-radius:5px;" +
        "border:1px solid transparent;background:transparent;color:#c4c4c4;" +
        "cursor:pointer;opacity:.75;flex-shrink:0;";

      var bridge = getTauriBridge();
      if (bridge && bridge.event) {
        bridge.event.listen("luma://update-progress", function (event) {
          var stage = event.payload && event.payload.stage;
          if (stage === "downloading") text.textContent = "Downloading the update…";
          else if (stage === "installing") text.textContent = "Installing - LUMA will restart itself…";
          else if (stage === "checking") text.textContent = "Checking the release…";
        });
      }

      updateBtn.addEventListener("click", function () {
        dlog("info", "update banner: user clicked Update now");
        text.textContent = "Starting the update…";
        updateBtn.disabled = true;
        laterBtn.remove();
        invoke("apply_update").catch(function (err) {
          // A genuine failure resolves here; success instead just ends
          // with this whole window going away as the app restarts, so
          // there's no matching "it worked" branch to handle.
          dlog("error", "apply_update invoke failed: " + err);
          text.textContent = "Update failed: " + err;
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

  // Shown once on launch if the *previous* run's auto-update failed to
  // finish - see updater::take_last_update_failure. On Windows the
  // process that kicks off an update has to exit before the file swap
  // can happen, so a failure there can't report back to a running
  // window; this is what makes that failure visible instead of the app
  // just quietly staying on the old version with no explanation.
  function showUpdateFailedBanner(message) {
    try {
      if (document.getElementById("luma-update-failed-banner")) return;

      var el = document.createElement("div");
      el.id = "luma-update-failed-banner";
      el.style.cssText =
        "position:fixed;left:50%;bottom:22px;transform:translateX(-50%);z-index:9998;" +
        "display:flex;align-items:center;gap:12px;max-width:calc(100% - 40px);" +
        "background:rgba(43,26,4,.96);color:#fff;font-family:monospace;font-size:13px;" +
        "padding:10px 14px;border-radius:8px;border:1px solid rgba(230,160,50,.5);" +
        "box-shadow:0 10px 30px rgba(0,0,0,.5);";

      var text = document.createElement("span");
      text.textContent = message;

      var dismissBtn = document.createElement("button");
      dismissBtn.type = "button";
      dismissBtn.textContent = "Dismiss";
      dismissBtn.style.cssText =
        "font-family:monospace;font-size:12px;padding:5px 10px;border-radius:5px;" +
        "border:1px solid transparent;background:transparent;color:#e0c9a6;" +
        "cursor:pointer;opacity:.85;flex-shrink:0;";
      dismissBtn.addEventListener("click", function () {
        el.remove();
      });

      el.appendChild(text);
      el.appendChild(dismissBtn);
      document.body.appendChild(el);
    } catch (e) {
      dlog("warn", "showUpdateFailedBanner failed: " + e);
    }
  }

  var params = new URLSearchParams(window.location.search);
  var isSpotlight = params.get("mode") === "spotlight";

  if (isSpotlight) {
    document.body.classList.add("spotlight-mode");
    // Also on <html>, not just <body> - see index.html's
    // `html.spotlight-mode { background: transparent }` rule for why:
    // html's own background paints through regardless of body's.
    document.documentElement.classList.add("spotlight-mode");
  }

  // Re-adds `cls` even if it's already present, forcing the CSS animation
  // named in that class's rule to restart from frame 0 - a bare
  // classList.add() on a class that's already there is a no-op, which
  // would otherwise leave a stale fade playing (or none at all) on a fast
  // hide-then-show. The offsetWidth read forces layout, flushing the
  // removal before the class goes back on.
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
      // The bang engine indicator is hidden in spotlight mode (no room,
      // and it'd give away which engine you're on when the point of the
      // pill is that you don't have to think about it) - so a per-engine
      // placeholder like "search the web" or "search wikipedia" is
      // misleading there with nothing on screen to explain it. Pin it to
      // one fixed, engine-agnostic placeholder instead.
      placeholderOverride: isSpotlight ? "search the universe" : null,
      onSearch: function (result) {
        // `local` engines (see vendor/engine/bangdeck.js) hand off to the
        // OS instead of resolving to a web address - which command they
        // hand off to depends on which local engine this was, so this
        // can't just always call search_mypc now that !open exists too.
        if (result.local) {
          dlog("info", "search submitted -> local, engine=" + result.engine + " query=" + result.query);
          if (result.engine === "Open") {
            invoke("open_app", { query: result.query })
              .then(function () {
                dlog("info", "open_app invoke resolved OK");
              })
              .catch(function (err) {
                // Sentinel from commands::APP_NOT_FOUND - nothing on this
                // PC matched the name, so ask the user to point at it
                // themselves instead of just showing an error.
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
            invoke("search_mypc", { query: result.query })
              .then(function () {
                dlog("info", "search_mypc invoke resolved OK");
              })
              .catch(function (err) {
                dlog("error", "search_mypc invoke failed: " + err);
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

    // The spotlight window is created once and then just shown/hidden by
    // toggle_spotlight - it never reloads index.html - so without this,
    // changing the theme in Settings would only be visible in the spotlight
    // after quitting and relaunching Luma. save_config emits this once the
    // new config is written; re-fetching and re-applying the theme/custom
    // CSS here is what makes the spotlight pill re-color live instead.
    tauri.event.listen("luma://config-changed", async function () {
      try {
        var freshConfig = await invoke("get_config");
        var freshThemeCss = await invoke("get_theme_css", { themeId: freshConfig.appearance.theme });
        applyThemeCss(freshThemeCss);
        applyCustomCss(freshConfig.appearance.custom_css);
        applyAnimationSetting(freshConfig.general && freshConfig.general.disable_animations);
        if (isSpotlight) applyBrandingSetting(freshConfig.general && freshConfig.general.show_spotlight_branding);
        dlog("info", (isSpotlight ? "spotlight" : "main") + ": re-applied theme '" + freshConfig.appearance.theme + "' after config change");
      } catch (err) {
        dlog("error", "config-changed handler failed: " + err);
      }
    });

    // Only the main window checks for updates on launch - the spotlight is
    // a small floating search bar, and it's the one window guaranteed to
    // exist right after startup (the spotlight/browser windows are created
    // lazily). See src-tauri/src/updater.rs for why this isn't Tauri's
    // official updater plugin, and settings.js for the manual "Check for
    // updates" button that calls the same commands.
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

      // Windows-only in practice (see take_last_update_failure) - a
      // no-op Ok(None) everywhere else. Checked once per launch so a
      // failed background swap from last time doesn't just go unnoticed.
      invoke("take_last_update_failure")
        .then(function (message) {
          if (message) {
            dlog("warn", "last update failed: " + message);
            showUpdateFailedBanner(message);
          }
        })
        .catch(function (err) {
          dlog("warn", "take_last_update_failure invoke failed: " + err);
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

      // Backend fires this right when it starts the 220ms grace period
      // before actually hiding the window (see
      // window::schedule_spotlight_hide) - this is what plays during that
      // window. With animations off there's nothing to play, so skip it
      // rather than adding a class that'd just sit there doing nothing.
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
      showFatalBanner("failed to start up (" + (err && err.message ? err.message : err) + ") - open Settings and hold Shift+L for the debug log.");
    });
  }

  init();
})();
