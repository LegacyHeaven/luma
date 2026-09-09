/**
 * Luma desktop app - Settings page. Loads the current config.toml (via
 * get_config), lets you edit it, and writes it back with save_config.
 *
 * Also owns the debug console: hold Shift+L on this page to reveal a
 * "Debug log" section; turning it on opens a live console showing every
 * notable thing Luma does plus RAM/process info. See debug-log.js for the
 * client-side half of this and src-tauri/src/logging.rs for the backend
 * half.
 */
(function () {
  "use strict";

  function dlog(level, message) {
    if (window.LumaDebugLog) window.LumaDebugLog.record(level, message);
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
    } catch (e) {}
  }

  var backLink = document.getElementById("back-to-search");
  if (backLink) {
    backLink.addEventListener("click", function (e) {
      e.preventDefault();
      window.location.href = "index.html";
    });
  }

  var shortcutInput = document.getElementById("shortcut-input");
  var defaultEngineSelect = document.getElementById("default-engine");
  var closeOnBlurCheckbox = document.getElementById("close-on-blur");
  var spotlightWidthInput = document.getElementById("spotlight-width");
  var pickPositionBtn = document.getElementById("pick-position-btn");
  var resetPositionBtn = document.getElementById("reset-position-btn");
  var spotlightPositionStatus = document.getElementById("spotlight-position-status");
  var disableAnimationsCheckbox = document.getElementById("disable-animations");
  var showSpotlightBrandingCheckbox = document.getElementById("show-spotlight-branding");
  var startAtLoginCheckbox = document.getElementById("start-at-login");
  var customEnginesList = document.getElementById("custom-engines-list");
  var newEngineName = document.getElementById("new-engine-name");
  var newEngineUrl = document.getElementById("new-engine-url");
  var newEngineBang = document.getElementById("new-engine-bang");
  var addEngineBtn = document.getElementById("add-engine-btn");
  var engineFormStatus = document.getElementById("engine-form-status");
  var themeOptionsContainer = document.getElementById("theme-options");
  var customCssTextarea = document.getElementById("custom-css");
  var openThemesFolderBtn = document.getElementById("open-themes-folder");
  var form = document.getElementById("settings-form");
  var saveStatus = document.getElementById("save-status");
  var buildStamp = document.getElementById("build-stamp");

  var checkUpdatesEnabled = document.getElementById("check-updates-enabled");
  var checkUpdatesBtn = document.getElementById("check-updates-btn");
  var updateStatus = document.getElementById("update-status");
  var installUpdateBtn = document.getElementById("install-update-btn");

  var debugSection = document.getElementById("debug-section");
  var debugLoggingEnabled = document.getElementById("debug-logging-enabled");
  var openDebugConsoleBtn = document.getElementById("open-debug-console");
  var debugConsole = document.getElementById("debug-console");
  var debugConsoleMeta = document.getElementById("debug-console-meta");
  var debugConsoleSysinfo = document.getElementById("debug-console-sysinfo");
  var debugConsoleLog = document.getElementById("debug-console-log");
  var debugCopyBtn = document.getElementById("debug-copy");
  var debugClearBtn = document.getElementById("debug-clear");
  var debugCloseBtn = document.getElementById("debug-close");

  var currentConfig = null;
  var selectedThemeId = null;
  var invoke = null; // set once the Tauri bridge is confirmed ready

  // ----- shortcut recorder -----
  var KEY_CODE_MAP = {
    Space: "Space", Enter: "Enter", Tab: "Tab", Escape: "Escape", Backspace: "Backspace",
    ArrowUp: "Up", ArrowDown: "Down", ArrowLeft: "Left", ArrowRight: "Right",
  };

  function codeToToken(e) {
    if (KEY_CODE_MAP[e.code]) return KEY_CODE_MAP[e.code];
    if (e.code.indexOf("Key") === 0) return e.code.slice(3);
    if (e.code.indexOf("Digit") === 0) return e.code.slice(5);
    if (/^F\d{1,2}$/.test(e.code)) return e.code;
    return null;
  }

  shortcutInput.addEventListener("focus", function () {
    shortcutInput.value = "Press keys…";
  });

  shortcutInput.addEventListener("keydown", function (e) {
    e.preventDefault();
    var token = codeToToken(e);
    if (!token) return; // modifier-only keydown; wait for the real key

    var parts = [];
    if (e.ctrlKey) parts.push("Control");
    if (e.altKey) parts.push("Alt");
    if (e.shiftKey) parts.push("Shift");
    if (e.metaKey) parts.push("Super");
    parts.push(token);

    shortcutInput.value = parts.join("+");
    shortcutInput.blur();
  });

  // ----- debug console -----

  // Shift+L reveals the (otherwise hidden) debug section. Doesn't fire
  // while the shortcut recorder or another text field has focus, so it
  // can't clash with actually typing "L".
  document.addEventListener("keydown", function (e) {
    var tag = document.activeElement && document.activeElement.tagName;
    var typing = tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT";
    if (!typing && e.shiftKey && (e.key === "L" || e.key === "l" || e.code === "KeyL")) {
      if (debugSection.hidden) {
        debugSection.hidden = false;
        debugSection.scrollIntoView({ behavior: "smooth", block: "center" });
        dlog("info", "debug section revealed via Shift+L");
      }
    }
  });

  function formatEntry(entry) {
    var d = new Date(entry.ts_ms);
    var time = d.toLocaleTimeString(undefined, { hour12: false }) + "." + String(d.getMilliseconds()).padStart(3, "0");
    return { time: time, level: entry.level, source: entry.source, message: entry.message };
  }

  function renderLog(entries) {
    var atBottom = debugConsoleLog.scrollTop + debugConsoleLog.clientHeight >= debugConsoleLog.scrollHeight - 20;

    var sorted = entries.slice().sort(function (a, b) { return a.ts_ms - b.ts_ms; });
    debugConsoleLog.innerHTML = sorted.map(function (raw) {
      var e = formatEntry(raw);
      var esc = function (s) { return String(s).replace(/[&<>]/g, function (c) { return { "&": "&amp;", "<": "&lt;", ">": "&gt;" }[c]; }); };
      return '<div class="debug-log-line level-' + esc(e.level) + '">' +
        e.time + ' <span class="debug-log-source">[' + esc(e.source) + ']</span> ' + esc(e.message) +
        '</div>';
    }).join("");

    if (atBottom) debugConsoleLog.scrollTop = debugConsoleLog.scrollHeight;
  }

  // Every JS-side dlog() call writes to localStorage immediately *and*
  // separately asks the Rust side to log the same message (so it shows up
  // even from a different window, and to prove the IPC bridge is actually
  // alive) - so the same message legitimately shows up in both
  // `serverEntries` and the client's own localStorage copy, a few ms
  // apart. Dedupe by source+message within a loose time window rather
  // than requiring an exact ts_ms match, so that pair collapses to one
  // line instead of showing every event twice.
  function mergedLogEntries(serverEntries) {
    var clientEntries = window.LumaDebugLog ? window.LumaDebugLog.all() : [];
    var all = (serverEntries || []).concat(clientEntries).sort(function (a, b) { return a.ts_ms - b.ts_ms; });

    var lastSeenAt = {}; // "source|message" -> ts_ms of the last kept copy
    var out = [];
    all.forEach(function (e) {
      var key = e.source + "|" + e.message;
      var last = lastSeenAt[key];
      if (last !== undefined && e.ts_ms - last < 3000) return; // same event, other pipe
      lastSeenAt[key] = e.ts_ms;
      out.push(e);
    });
    return out;
  }

  function formatBytes(n) {
    if (!n && n !== 0) return "?";
    var units = ["B", "KB", "MB", "GB"];
    var i = 0;
    while (n >= 1024 && i < units.length - 1) { n /= 1024; i++; }
    return n.toFixed(i === 0 ? 0 : 1) + " " + units[i];
  }

  function formatUptime(secs) {
    if (!secs && secs !== 0) return "?";
    var h = Math.floor(secs / 3600), m = Math.floor((secs % 3600) / 60), s = Math.floor(secs % 60);
    return (h ? h + "h " : "") + (m ? m + "m " : "") + s + "s";
  }

  var sysinfoTimer = null;

  function refreshSysinfo() {
    if (!invoke) return;
    invoke("get_system_info")
      .then(function (info) {
        debugConsoleMeta.textContent = "LUMA " + info.app_version + " · build " + info.build_sha + " · pid " + info.pid;
        var windowsList = (info.windows || []).map(function (w) {
          return w.label + (w.visible ? " (visible)" : " (hidden)");
        }).join(", ") || "none";
        debugConsoleSysinfo.innerHTML =
          "<div><b>Memory:</b> " + formatBytes(info.process_rss_bytes) + " used by LUMA / " +
            formatBytes(info.system_used_mem_bytes) + " of " + formatBytes(info.system_total_mem_bytes) + " system</div>" +
          "<div><b>Uptime:</b> " + formatUptime(info.process_uptime_secs) + "</div>" +
          "<div><b>OS:</b> " + info.os + " " + info.os_version + "</div>" +
          "<div><b>Config dir:</b> " + info.config_dir + "</div>" +
          "<div><b>Shortcut:</b> " + info.shortcut + " &nbsp; <b>Browser mode:</b> " + info.browser_mode + " &nbsp; <b>Theme:</b> " + info.theme + "</div>" +
          "<div><b>Windows open:</b> " + windowsList + "</div>";
      })
      .catch(function (err) {
        debugConsoleSysinfo.textContent = "get_system_info failed: " + err;
      });
  }

  function refreshLog() {
    if (invoke) {
      invoke("get_debug_log")
        .then(function (serverEntries) { renderLog(mergedLogEntries(serverEntries)); })
        .catch(function () { renderLog(mergedLogEntries([])); });
    } else {
      renderLog(mergedLogEntries([]));
    }
  }

  function openDebugConsole() {
    debugConsole.hidden = false;
    refreshSysinfo();
    refreshLog();
    if (sysinfoTimer) clearInterval(sysinfoTimer);
    sysinfoTimer = setInterval(function () { refreshSysinfo(); refreshLog(); }, 1000);
    dlog("info", "debug console opened");
  }

  function closeDebugConsole() {
    debugConsole.hidden = true;
    if (sysinfoTimer) { clearInterval(sysinfoTimer); sysinfoTimer = null; }
  }

  openDebugConsoleBtn.addEventListener("click", openDebugConsole);
  debugCloseBtn.addEventListener("click", closeDebugConsole);

  debugClearBtn.addEventListener("click", function () {
    if (window.LumaDebugLog) window.LumaDebugLog.clear();
    if (invoke) invoke("clear_debug_log").catch(function () {});
    renderLog([]);
  });

  debugCopyBtn.addEventListener("click", function () {
    var text = debugConsoleLog.innerText;
    var done = function () {
      var original = debugCopyBtn.textContent;
      debugCopyBtn.textContent = "Copied!";
      setTimeout(function () { debugCopyBtn.textContent = original; }, 1200);
    };
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(text).then(done).catch(function () { fallbackCopy(text, done); });
    } else {
      fallbackCopy(text, done);
    }
  });

  function fallbackCopy(text, done) {
    try {
      var ta = document.createElement("textarea");
      ta.value = text;
      ta.style.position = "fixed";
      ta.style.opacity = "0";
      document.body.appendChild(ta);
      ta.select();
      document.execCommand("copy");
      document.body.removeChild(ta);
      done();
    } catch (e) {
      dlog("warn", "copy-to-clipboard fallback failed: " + e);
    }
  }

  debugLoggingEnabled.addEventListener("change", function () {
    if (currentConfig) {
      currentConfig.general.debug_logging = debugLoggingEnabled.checked;
      if (invoke) {
        invoke("save_config", { newConfig: currentConfig }).catch(function (err) {
          dlog("error", "failed to persist debug_logging toggle: " + err);
        });
      }
    }
    if (debugLoggingEnabled.checked) {
      openDebugConsole();
    }
  });

  // ----- updates -----
  // See check_for_update/apply_update in src-tauri/src/updater.rs - same
  // commands the main window's "Update available" banner uses on launch,
  // this is just the manual/visible half of it.
  var pendingUpdate = null;

  function checkForUpdates() {
    if (!invoke) return;
    checkUpdatesBtn.disabled = true;
    updateStatus.textContent = "Checking…";
    installUpdateBtn.hidden = true;
    pendingUpdate = null;

    invoke("check_for_update")
      .then(function (status) {
        checkUpdatesBtn.disabled = false;
        if (!status.checked_ok) {
          updateStatus.textContent = "Couldn't check for updates: " + status.error;
          dlog("warn", "check_for_update failed: " + status.error);
          return;
        }
        if (status.available) {
          pendingUpdate = status;
          updateStatus.textContent = "Update available (build " + status.latest_commit + ").";
          installUpdateBtn.hidden = false;
          dlog("info", "settings: update available " + status.current_commit + " -> " + status.latest_commit);
        } else {
          updateStatus.textContent = "You're up to date (build " + status.current_commit + ").";
        }
      })
      .catch(function (err) {
        checkUpdatesBtn.disabled = false;
        updateStatus.textContent = "Couldn't check for updates: " + err;
        dlog("error", "check_for_update invoke failed: " + err);
      });
  }

  function installUpdate() {
    if (!invoke || !pendingUpdate) return;
    installUpdateBtn.disabled = true;
    checkUpdatesBtn.disabled = true;
    updateStatus.textContent = "Starting the update…";

    invoke("apply_update").catch(function (err) {
      // A real failure resolves here; success instead ends with this
      // window going away as the app restarts, so there's no "it worked"
      // branch to handle on this side.
      dlog("error", "apply_update invoke failed: " + err);
      updateStatus.textContent = "Update failed: " + err;
      installUpdateBtn.disabled = false;
      checkUpdatesBtn.disabled = false;
    });
  }

  checkUpdatesBtn.addEventListener("click", checkForUpdates);
  installUpdateBtn.addEventListener("click", installUpdate);

  checkUpdatesEnabled.addEventListener("change", function () {
    if (currentConfig) {
      currentConfig.general.check_for_updates = checkUpdatesEnabled.checked;
      if (invoke) {
        invoke("save_config", { newConfig: currentConfig }).catch(function (err) {
          dlog("error", "failed to persist check_for_updates toggle: " + err);
        });
      }
    }
  });

  // ----- spotlight position (Pick position / Reset) -----
  // Unlike the rest of this form, picking or resetting the spotlight's
  // position takes effect immediately rather than waiting for Save - see
  // commands::report_spotlight_position/reset_spotlight_position, which is
  // what Julian asked for ("after that the clicked position is saved").
  function describePosition(cfg) {
    if (!cfg) return "";
    if (cfg.window.spotlight_position === "custom" &&
        cfg.window.spotlight_custom_x != null && cfg.window.spotlight_custom_y != null) {
      return "Currently: a custom picked spot (" +
        Math.round(cfg.window.spotlight_custom_x * 100) + "%, " +
        Math.round(cfg.window.spotlight_custom_y * 100) + "% of your screen).";
    }
    return "Currently: screen center (default).";
  }

  pickPositionBtn.addEventListener("click", function () {
    if (!invoke) return;
    pickPositionBtn.disabled = true;
    spotlightPositionStatus.textContent = "Click anywhere on your screen…";
    invoke("open_position_picker").catch(function (err) {
      dlog("error", "open_position_picker invoke failed: " + err);
      spotlightPositionStatus.textContent = "Couldn't open the position picker: " + err;
      pickPositionBtn.disabled = false;
    });
  });

  resetPositionBtn.addEventListener("click", function () {
    if (!invoke) return;
    invoke("reset_spotlight_position")
      .then(function () {
        if (currentConfig) {
          currentConfig.window.spotlight_position = "center";
          currentConfig.window.spotlight_custom_x = null;
          currentConfig.window.spotlight_custom_y = null;
        }
        spotlightPositionStatus.textContent = describePosition(currentConfig);
        dlog("info", "settings: spotlight position reset to center");
      })
      .catch(function (err) {
        dlog("error", "reset_spotlight_position invoke failed: " + err);
        spotlightPositionStatus.textContent = "Couldn't reset the position: " + err;
      });
  });

  // ----- search engines (default-engine dropdown + custom engines) -----
  // `local` engines (currently just !mypc) hand off to the OS instead of
  // resolving to a URL - see vendor/engine/bangdeck.js - so they don't
  // belong in a "pick your default engine" list. `user_added` marks the
  // ones from config::CustomEngine (see commands::get_engines), so this
  // can render just those with a "remove" button.
  async function loadEngines() {
    var engineConfig = await invoke("get_engines");
    var engines = engineConfig.engines || [];

    defaultEngineSelect.innerHTML = "";
    engines.filter(function (e) { return !e.local; }).forEach(function (engine) {
      var opt = document.createElement("option");
      opt.value = engine.name;
      opt.textContent = engine.name + " (!" + engine.bang + ")";
      defaultEngineSelect.appendChild(opt);
    });
    defaultEngineSelect.value = currentConfig.general.default_engine;

    renderCustomEngines(engines.filter(function (e) { return e.user_added; }));
  }

  function renderCustomEngines(customEngines) {
    customEnginesList.innerHTML = "";
    if (!customEngines.length) {
      var empty = document.createElement("span");
      empty.className = "update-status";
      empty.textContent = "No custom engines yet - add one below.";
      customEnginesList.appendChild(empty);
      return;
    }
    customEngines.forEach(function (engine) {
      var row = document.createElement("div");
      row.className = "engine-row";

      var label = document.createElement("span");
      label.textContent = engine.name + " ";
      var bang = document.createElement("span");
      bang.className = "bang";
      bang.textContent = "!" + engine.bang;
      label.appendChild(bang);
      row.appendChild(label);

      var removeBtn = document.createElement("button");
      removeBtn.type = "button";
      removeBtn.className = "engine-row-remove";
      removeBtn.textContent = "✕";
      removeBtn.title = "Remove " + engine.name;
      removeBtn.addEventListener("click", function () {
        invoke("remove_custom_engine", { name: engine.name })
          .then(function () {
            dlog("info", "settings: removed custom engine " + engine.name);
            return loadEngines();
          })
          .catch(function (err) {
            dlog("error", "remove_custom_engine invoke failed: " + err);
            engineFormStatus.textContent = "Couldn't remove it: " + err;
          });
      });
      row.appendChild(removeBtn);

      customEnginesList.appendChild(row);
    });
  }

  addEngineBtn.addEventListener("click", function () {
    if (!invoke) return;
    var engine = {
      name: newEngineName.value.trim(),
      action: newEngineUrl.value.trim(),
      bang: newEngineBang.value.trim(),
      placeholder: "",
    };
    if (!engine.name || !engine.action || !engine.bang) {
      engineFormStatus.textContent = "Name, URL, and bang are all required.";
      return;
    }
    addEngineBtn.disabled = true;
    engineFormStatus.textContent = "Adding…";
    invoke("add_custom_engine", { engine: engine })
      .then(function () {
        newEngineName.value = "";
        newEngineUrl.value = "";
        newEngineBang.value = "";
        engineFormStatus.textContent = "Added " + engine.name + ".";
        dlog("info", "settings: added custom engine " + engine.name);
        return loadEngines();
      })
      .catch(function (err) {
        dlog("error", "add_custom_engine invoke failed: " + err);
        engineFormStatus.textContent = "Couldn't add it: " + err;
      })
      .finally(function () {
        addEngineBtn.disabled = false;
      });
  });

  // ----- load current config + supporting data -----
  async function boot(tauri) {
    invoke = tauri.core.invoke;
    dlog("info", "settings window: boot() starting");

    var themeCss;
    try {
      currentConfig = await invoke("get_config");
      dlog("info", "settings: config loaded");
    } catch (err) {
      dlog("error", "settings: get_config invoke failed: " + err);
      showFatalBanner("couldn't load your settings (" + err + ") - nothing below will be accurate. Hold Shift+L for the debug log.");
      return;
    }

    try {
      themeCss = await invoke("get_theme_css", { themeId: currentConfig.appearance.theme });
      var style = document.createElement("style");
      style.textContent = themeCss;
      document.head.appendChild(style);
    } catch (err) {
      dlog("error", "settings: get_theme_css invoke failed: " + err);
    }

    shortcutInput.value = currentConfig.general.shortcut;
    document.querySelector('input[name="browser_mode"][value="' + currentConfig.general.browser_mode + '"]').checked = true;
    closeOnBlurCheckbox.checked = !!currentConfig.general.close_spotlight_on_blur;
    spotlightWidthInput.value = currentConfig.window.spotlight_width;
    spotlightPositionStatus.textContent = describePosition(currentConfig);
    disableAnimationsCheckbox.checked = !!currentConfig.general.disable_animations;
    showSpotlightBrandingCheckbox.checked = !!currentConfig.general.show_spotlight_branding;
    var mainSizeInput = document.querySelector(
      'input[name="main_window_size"][value="' + (currentConfig.window.main_window_size || "default") + '"]'
    );
    if (mainSizeInput) mainSizeInput.checked = true;
    startAtLoginCheckbox.checked = !!currentConfig.general.start_at_login;
    checkUpdatesEnabled.checked = currentConfig.general.check_for_updates !== false;
    customCssTextarea.value = currentConfig.appearance.custom_css || "";
    selectedThemeId = currentConfig.appearance.theme;

    tauri.event.listen("luma://update-progress", function (event) {
      var stage = event.payload && event.payload.stage;
      if (stage === "checking") updateStatus.textContent = "Checking the release…";
      else if (stage === "downloading") updateStatus.textContent = "Downloading the update…";
      else if (stage === "installing") updateStatus.textContent = "Installing - LUMA will restart itself…";
    });

    // The position-picker overlay saves straight to config.toml and emits
    // this (see commands::report_spotlight_position) - re-fetch so this
    // window's status line reflects the freshly-picked spot, and
    // re-enable the button either way (picked, or cancelled with Escape).
    tauri.event.listen("luma://config-changed", async function () {
      pickPositionBtn.disabled = false;
      try {
        currentConfig = await invoke("get_config");
        spotlightPositionStatus.textContent = describePosition(currentConfig);
      } catch (err) {
        dlog("warn", "settings: re-fetching config after config-changed failed: " + err);
      }
    });

    if (currentConfig.general.debug_logging) {
      debugSection.hidden = false;
      debugLoggingEnabled.checked = true;
    }

    try {
      await loadEngines();
    } catch (err) {
      dlog("error", "settings: get_engines invoke failed: " + err);
    }

    try {
      var themeList = await invoke("list_themes");
      renderThemeOptions(themeList);
    } catch (err) {
      dlog("error", "settings: list_themes invoke failed: " + err);
    }

    try {
      var info = await invoke("get_system_info");
      buildStamp.textContent = "LUMA " + info.app_version + " · build " + info.build_sha + " · pid " + info.pid;
    } catch (err) {
      buildStamp.textContent = "LUMA - build info unavailable (" + err + ")";
    }

    dlog("info", "settings: boot() complete");
  }

  function renderThemeOptions(themes) {
    themeOptionsContainer.innerHTML = "";
    themes.forEach(function (theme) {
      var el = document.createElement("div");
      el.className = "theme-option" + (theme.id === selectedThemeId ? " selected" : "");
      el.textContent = theme.name + " - " + theme.author;
      el.addEventListener("click", function () {
        selectedThemeId = theme.id;
        document.querySelectorAll(".theme-option").forEach(function (o) { o.classList.remove("selected"); });
        el.classList.add("selected");
      });
      themeOptionsContainer.appendChild(el);
    });
  }

  openThemesFolderBtn.addEventListener("click", function () {
    if (!invoke) return;
    invoke("reveal_themes_folder").catch(function (err) {
      dlog("error", "reveal_themes_folder invoke failed: " + err);
    });
  });

  form.addEventListener("submit", async function (e) {
    e.preventDefault();

    if (!invoke) {
      dlog("error", "Save clicked but the Tauri bridge never initialized - nothing to send this to.");
      saveStatus.textContent = "Failed to save - internal bridge not ready. See debug log (Shift+L).";
      saveStatus.classList.add("visible");
      return;
    }

    var browserModeInput = document.querySelector('input[name="browser_mode"]:checked');

    var updated = JSON.parse(JSON.stringify(currentConfig));
    updated.general.shortcut = shortcutInput.value || "Alt+Space";
    updated.general.browser_mode = browserModeInput ? browserModeInput.value : "system";
    updated.general.default_engine = defaultEngineSelect.value || updated.general.default_engine;
    updated.general.close_spotlight_on_blur = closeOnBlurCheckbox.checked;
    updated.general.start_at_login = startAtLoginCheckbox.checked;
    updated.general.debug_logging = debugLoggingEnabled.checked;
    updated.general.check_for_updates = checkUpdatesEnabled.checked;
    updated.general.disable_animations = disableAnimationsCheckbox.checked;
    updated.general.show_spotlight_branding = showSpotlightBrandingCheckbox.checked;
    updated.window.spotlight_width = parseInt(spotlightWidthInput.value, 10) || 640;
    // spotlight_position/custom_x/custom_y are NOT set here - Pick
    // position/Reset (above) write those straight to config.toml the
    // moment they happen, and `updated` is cloned from the freshly
    // re-fetched currentConfig, so a normal Save just carries them through
    // unchanged instead of stomping on whatever the picker last set.
    var mainSizeChecked = document.querySelector('input[name="main_window_size"]:checked');
    updated.window.main_window_size = mainSizeChecked ? mainSizeChecked.value : "default";
    updated.appearance.theme = selectedThemeId || updated.appearance.theme;
    updated.appearance.custom_css = customCssTextarea.value || "";

    dlog("info", "settings: submitting save with browser_mode=" + updated.general.browser_mode);

    try {
      await invoke("save_config", { newConfig: updated });
      currentConfig = updated;
      dlog("info", "settings: save_config resolved OK");
      saveStatus.textContent = "Saved.";
      saveStatus.classList.add("visible");
      setTimeout(function () { saveStatus.classList.remove("visible"); }, 2000);
    } catch (err) {
      dlog("error", "settings: save_config invoke failed: " + err);
      saveStatus.textContent = "Failed to save: " + err;
      saveStatus.classList.add("visible");
    }
  });

  function init(attempt) {
    attempt = attempt || 1;
    var tauri = getTauriBridge();

    if (!tauri) {
      dlog("warn", "settings: window.__TAURI__ not ready yet (attempt " + attempt + "/20)");
      if (attempt < 20) {
        setTimeout(function () { init(attempt + 1); }, 100);
        return;
      }
      dlog("error", "settings: window.__TAURI__ never became available after 20 attempts (2s)");
      buildStamp.textContent = "LUMA - internal bridge did not start";
      showFatalBanner(
        "internal bridge didn't start in this window - nothing here will load or save. " +
          "Try restarting LUMA. The client-side log above still recorded this, even " +
          "though it couldn't reach the backend log."
      );
      return;
    }

    if (attempt > 1) dlog("info", "settings: window.__TAURI__ became available on attempt " + attempt);

    boot(tauri).catch(function (err) {
      dlog("error", "settings: boot() threw: " + (err && err.message ? err.message : err));
      showFatalBanner("failed to start up (" + (err && err.message ? err.message : err) + ")");
    });
  }

  init();
})();
