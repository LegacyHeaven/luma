(function () {
  "use strict";

  function dlog(level, message) {
    if (window.LumaDebugLog) window.LumaDebugLog.record(level, message);
  }

  var NAV_TRANSITION_MS = 280;

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
      navigateTo("index.html");
    });
  }

  var shortcutInput = document.getElementById("shortcut-input");
  var defaultEngineSelect = document.getElementById("default-engine");
  var languageSelect = document.getElementById("language-select");
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
  var builtinEnginesList = document.getElementById("builtin-engines-list");
  var addBuiltinEngineSelect = document.getElementById("add-builtin-engine");
  var customAppsList = document.getElementById("custom-apps-list");
  var pluginsList = document.getElementById("plugins-list");
  var openPluginsFolderBtn = document.getElementById("open-plugins-folder");
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
  var showDebugTierCheckbox = document.getElementById("show-debug-tier");
  var revealLogFileBtn = document.getElementById("reveal-log-file");

  var currentConfig = null;
  var selectedThemeId = null;
  var invoke = null;
  var saveQueue = Promise.resolve();

  function showSaveStatus(text, isError) {
    saveStatus.textContent = text;
    saveStatus.classList.toggle("error", !!isError);
    saveStatus.classList.add("visible");
    clearTimeout(showSaveStatus._t);
    showSaveStatus._t = setTimeout(function () { saveStatus.classList.remove("visible"); }, 1500);
  }

  function persistPatch(mutator, label) {
    saveQueue = saveQueue.then(function () {
      if (!invoke || !currentConfig) return;
      var updated = JSON.parse(JSON.stringify(currentConfig));
      mutator(updated);
      return invoke("save_config", { newConfig: updated })
        .then(function () {
          currentConfig = updated;
          dlog("info", "settings: auto-saved " + label);
          showSaveStatus(tt("settings.status.saved", "Saved."));
        })
        .catch(function (err) {
          dlog("error", "settings: auto-save failed (" + label + "): " + err);
          showSaveStatus(ff("settings.status.save_failed", [err], "Couldn't save: {0}"), true);
        });
    });
    return saveQueue;
  }

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

  Array.prototype.slice.call(document.querySelectorAll('input[name="browser_mode"]')).forEach(function (radio) {
    radio.addEventListener("change", function () {
      if (!radio.checked) return;
      persistPatch(function (cfg) { cfg.general.browser_mode = radio.value; }, "browser mode");
    });
  });

  defaultEngineSelect.addEventListener("change", function () {
    persistPatch(function (cfg) { cfg.general.default_engine = defaultEngineSelect.value; }, "default engine");
  });

  async function populateLanguageSelect() {
    if (!languageSelect || !window.LumaI18n) return;
    var locales;
    try {
      locales = await window.LumaI18n.loadLocaleList(invoke);
    } catch (err) {
      dlog("error", "settings: loading locale list failed: " + err);
      return;
    }
    languageSelect.textContent = "";
    var current = (currentConfig.general && currentConfig.general.locale) || "en";
    (locales || []).forEach(function (locale) {
      var option = document.createElement("option");
      option.value = locale.id;
      option.textContent = locale.name;
      if (locale.id === current) option.selected = true;
      languageSelect.appendChild(option);
    });
  }

  if (languageSelect) {
    languageSelect.addEventListener("change", function () {
      var localeId = languageSelect.value;
      persistPatch(function (cfg) { cfg.general.locale = localeId; }, "language");
      if (window.LumaI18n) {
        window.LumaI18n.init(invoke, localeId).catch(function (err) {
          dlog("error", "settings: applying new locale failed: " + err);
        });
      }
    });
  }

  Array.prototype.slice.call(document.querySelectorAll('input[name="main_window_size"]')).forEach(function (radio) {
    radio.addEventListener("change", function () {
      if (!radio.checked) return;
      persistPatch(function (cfg) { cfg.window.main_window_size = radio.value; }, "main window size");
    });
  });

  startAtLoginCheckbox.addEventListener("change", function () {
    persistPatch(function (cfg) { cfg.general.start_at_login = startAtLoginCheckbox.checked; }, "start at login");
  });

  closeOnBlurCheckbox.addEventListener("change", function () {
    persistPatch(function (cfg) { cfg.general.close_spotlight_on_blur = closeOnBlurCheckbox.checked; }, "close on blur");
  });

  spotlightWidthInput.addEventListener("change", function () {
    persistPatch(function (cfg) {
      cfg.window.spotlight_width = parseInt(spotlightWidthInput.value, 10) || 640;
    }, "spotlight width");
  });

  disableAnimationsCheckbox.addEventListener("change", function () {
    document.body.classList.toggle("no-animations", disableAnimationsCheckbox.checked);
    persistPatch(function (cfg) { cfg.general.disable_animations = disableAnimationsCheckbox.checked; }, "disable animations");
  });

  showSpotlightBrandingCheckbox.addEventListener("change", function () {
    persistPatch(function (cfg) { cfg.general.show_spotlight_branding = showSpotlightBrandingCheckbox.checked; }, "show spotlight branding");
  });

  var customCssSaveTimer = null;
  customCssTextarea.addEventListener("input", function () {
    clearTimeout(customCssSaveTimer);
    customCssSaveTimer = setTimeout(function () {
      persistPatch(function (cfg) { cfg.appearance.custom_css = customCssTextarea.value || ""; }, "custom css");
    }, 600);
  });
  customCssTextarea.addEventListener("blur", function () {
    clearTimeout(customCssSaveTimer);
    persistPatch(function (cfg) { cfg.appearance.custom_css = customCssTextarea.value || ""; }, "custom css");
  });

  shortcutInput.addEventListener("focus", function () {
    shortcutInput.value = tt("settings.general.shortcut.pressing", "Press keys…");
  });

  shortcutInput.addEventListener("keydown", function (e) {
    e.preventDefault();
    var token = codeToToken(e);
    if (!token) return;

    var parts = [];
    if (e.ctrlKey) parts.push("Control");
    if (e.altKey) parts.push("Alt");
    if (e.shiftKey) parts.push("Shift");
    if (e.metaKey) parts.push("Super");
    parts.push(token);

    shortcutInput.value = parts.join("+");
    shortcutInput.blur();
    persistPatch(function (cfg) { cfg.general.shortcut = shortcutInput.value; }, "shortcut");
  });

  document.addEventListener("keydown", function (e) {
    var tag = document.activeElement && document.activeElement.tagName;
    var typing = tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT";
    if (!typing && e.shiftKey && (e.key === "L" || e.key === "l" || e.code === "KeyL")) {
      if (debugSection.hidden) {
        debugSection.hidden = false;
        if (advancedNavBtn) advancedNavBtn.hidden = false;
        setActiveCategory("advanced");
        debugSection.scrollIntoView({ behavior: "smooth", block: "center" });
        dlog("info", "debug section revealed via Shift+L");
      }
    }
  });

  var settingsNav = document.getElementById("settings-nav");
  var advancedNavBtn = document.getElementById("advanced-nav-btn");
  var navButtons = settingsNav ? Array.prototype.slice.call(settingsNav.querySelectorAll(".settings-nav-btn")) : [];
  var categorySections = Array.prototype.slice.call(document.querySelectorAll(".settings-content > .settings-section[data-category]"));

  var customizeSubnav = document.getElementById("customize-subnav");
  var subnavButtons = customizeSubnav ? Array.prototype.slice.call(customizeSubnav.querySelectorAll(".settings-subnav-btn")) : [];
  var subcategorySections = Array.prototype.slice.call(document.querySelectorAll(".settings-content > .settings-section[data-subcategory]"));

  function loadForSubcategory(subcategory) {
    if (subcategory === "themes") {
      loadMarketplace({ preservePage: true });
    } else if (subcategory === "plugins") {
      loadPluginMarketplace({ preservePage: true });
    }
  }

  function setActiveSubcategory(subcategory) {
    subnavButtons.forEach(function (btn) {
      btn.classList.toggle("active", btn.dataset.subcategory === subcategory);
    });
    subcategorySections.forEach(function (section) {
      section.classList.toggle("sub-active", section.dataset.subcategory === subcategory);
    });
  }

  subnavButtons.forEach(function (btn) {
    btn.addEventListener("click", function () {
      setActiveSubcategory(btn.dataset.subcategory);
      loadForSubcategory(btn.dataset.subcategory);
    });
  });

  setActiveSubcategory("themes");

  function setActiveCategory(category) {
    navButtons.forEach(function (btn) {
      btn.classList.toggle("active", btn.dataset.category === category);
    });
    categorySections.forEach(function (section) {
      section.classList.toggle("cat-active", section.dataset.category === category);
    });
  }

  navButtons.forEach(function (btn) {
    btn.addEventListener("click", function () {
      setActiveCategory(btn.dataset.category);
      if (btn.dataset.category === "customize") {
        var activeSubBtn = subnavButtons.filter(function (b) { return b.classList.contains("active"); })[0];
        loadForSubcategory(activeSubBtn ? activeSubBtn.dataset.subcategory : "themes");
      }
    });
  });

  setActiveCategory("general");

  function formatEntry(entry) {
    var d = new Date(entry.ts_ms);
    var time = d.toLocaleTimeString(undefined, { hour12: false }) + "." + String(d.getMilliseconds()).padStart(3, "0");
    return { time: time, level: entry.level, source: entry.source, message: entry.message };
  }

  function renderLog(entries) {
    var atBottom = debugConsoleLog.scrollTop + debugConsoleLog.clientHeight >= debugConsoleLog.scrollHeight - 20;

    var sorted = entries.slice().sort(function (a, b) { return a.ts_ms - b.ts_ms; });
    if (!showDebugTierCheckbox || !showDebugTierCheckbox.checked) {
      sorted = sorted.filter(function (raw) { return raw.level !== "debug"; });
    }

    debugConsoleLog.textContent = "";
    var frag = document.createDocumentFragment();
    sorted.forEach(function (raw) {
      var e = formatEntry(raw);

      var line = document.createElement("div");
      line.className = "debug-log-line level-" + String(e.level).replace(/[^a-z0-9-]/gi, "");

      line.appendChild(document.createTextNode(e.time + " "));

      var source = document.createElement("span");
      source.className = "debug-log-source";
      source.textContent = "[" + e.source + "]";
      line.appendChild(source);

      line.appendChild(document.createTextNode(" " + e.message));

      frag.appendChild(line);
    });
    debugConsoleLog.appendChild(frag);

    if (atBottom) debugConsoleLog.scrollTop = debugConsoleLog.scrollHeight;
  }

  function mergedLogEntries(serverEntries) {
    var clientEntries = window.LumaDebugLog ? window.LumaDebugLog.all() : [];
    var all = (serverEntries || []).concat(clientEntries).sort(function (a, b) { return a.ts_ms - b.ts_ms; });

    var lastSeenAt = {};
    var out = [];
    all.forEach(function (e) {
      var key = e.source + "|" + e.message;
      var last = lastSeenAt[key];
      if (last !== undefined && e.ts_ms - last < 3000) return;
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

  var sysinfoOpenSections = { memory: true, logging: false, system: false, windows: false };

  function sysinfoRow(label, value) {
    var row = document.createElement("div");
    var b = document.createElement("b");
    b.textContent = label + ":";
    row.appendChild(b);
    row.appendChild(document.createTextNode(" " + value));
    return row;
  }

  function sysinfoSection(id, title, rows) {
    var details = document.createElement("details");
    details.className = "debug-sysinfo-section";
    details.open = !!sysinfoOpenSections[id];
    details.addEventListener("toggle", function () {
      sysinfoOpenSections[id] = details.open;
    });

    var summary = document.createElement("summary");
    summary.textContent = title;
    details.appendChild(summary);

    var body = document.createElement("div");
    body.className = "debug-sysinfo-section-body";
    rows.forEach(function (row) { body.appendChild(row); });
    details.appendChild(body);

    return details;
  }

  function tt(key, fallback) {
    return window.LumaI18n ? window.LumaI18n.t(key, fallback) : fallback;
  }
  function ff(key, params, fallback) {
    return window.LumaI18n ? window.LumaI18n.format(key, params, fallback) : fallback;
  }

  function refreshSysinfo() {
    if (!invoke) return;
    invoke("get_system_info")
      .then(function (info) {
        debugConsoleMeta.textContent = "LUMA " + info.app_version + " · build " + info.build_sha + " · pid " + info.pid;
        var noneLabel = tt("debug_console.sysinfo.none", "none");
        var windowsList = (info.windows || []).map(function (w) {
          return w.label + " (" + (w.visible
            ? tt("debug_console.sysinfo.window_visible", "visible")
            : tt("debug_console.sysinfo.window_hidden", "hidden")) + ")";
        }).join(", ") || noneLabel;
        var processTree = (info.process_tree || []).map(function (p) {
          return p.name + " (pid " + p.pid + ", " + formatBytes(p.rss_bytes) + ")";
        }).join(", ") || noneLabel;

        var loggingRows = [sysinfoRow(
          tt("debug_console.sysinfo.advanced_logging", "Advanced logging"),
          info.advanced_logging ? tt("debug_console.sysinfo.on", "on") : tt("debug_console.sysinfo.off", "off")
        )];
        if (info.advanced_logging) {
          loggingRows.push(sysinfoRow(
            tt("debug_console.sysinfo.log_file", "Log file"),
            ff("debug_console.sysinfo.log_file_value", [info.log_file_path, formatBytes(info.log_file_bytes)], "{0} ({1})")
          ));
        }

        debugConsoleSysinfo.textContent = "";
        var frag = document.createDocumentFragment();
        frag.appendChild(sysinfoSection("memory", tt("debug_console.sysinfo.section_memory", "Memory"), [
          sysinfoRow(tt("debug_console.sysinfo.this_process", "This process"), formatBytes(info.process_rss_bytes)),
          sysinfoRow(
            tt("debug_console.sysinfo.whole_app", "Whole app (all processes)"),
            ff("debug_console.sysinfo.whole_app_value", [formatBytes(info.process_tree_rss_bytes), (info.process_tree || []).length], "{0} across {1} process(es)")
          ),
          sysinfoRow(
            tt("debug_console.sysinfo.system_memory", "System memory"),
            ff("debug_console.sysinfo.of_value", [formatBytes(info.system_used_mem_bytes), formatBytes(info.system_total_mem_bytes)], "{0} of {1}")
          ),
          sysinfoRow(tt("debug_console.sysinfo.process_tree", "Process tree"), processTree),
        ]));
        frag.appendChild(sysinfoSection("logging", tt("debug_console.sysinfo.section_logging", "Logging"), loggingRows));
        frag.appendChild(sysinfoSection("system", tt("debug_console.sysinfo.section_system", "System"), [
          sysinfoRow(tt("debug_console.sysinfo.uptime", "Uptime"), formatUptime(info.process_uptime_secs)),
          sysinfoRow(tt("debug_console.sysinfo.os", "OS"), info.os + " " + info.os_version),
          sysinfoRow(tt("debug_console.sysinfo.config_dir", "Config dir"), info.config_dir),
        ]));
        frag.appendChild(sysinfoSection("windows", tt("debug_console.sysinfo.section_windows", "Windows & config"), [
          sysinfoRow(tt("debug_console.sysinfo.shortcut", "Shortcut"), info.shortcut),
          sysinfoRow(tt("debug_console.sysinfo.browser_mode", "Browser mode"), info.browser_mode),
          sysinfoRow(tt("debug_console.sysinfo.theme", "Theme"), info.theme),
          sysinfoRow(tt("debug_console.sysinfo.windows_open", "Windows open"), windowsList),
        ]));
        debugConsoleSysinfo.appendChild(frag);
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

  if (showDebugTierCheckbox) {
    showDebugTierCheckbox.addEventListener("change", function () { refreshLog(); });
  }

  if (revealLogFileBtn) {
    revealLogFileBtn.addEventListener("click", function () {
      if (!invoke) return;
      invoke("reveal_log_file").catch(function (err) {
        dlog("error", "reveal_log_file invoke failed: " + err);
      });
    });
  }

  var maintenanceStatus = document.getElementById("maintenance-status");
  function showMaintenanceStatus(text, isError) {
    if (!maintenanceStatus) return;
    maintenanceStatus.textContent = text;
    maintenanceStatus.classList.toggle("error", !!isError);
    maintenanceStatus.classList.add("visible");
    clearTimeout(showMaintenanceStatus._t);
    showMaintenanceStatus._t = setTimeout(function () { maintenanceStatus.classList.remove("visible"); }, 4000);
  }

  var clearBrowsingDataBtn = document.getElementById("clear-browsing-data-btn");
  if (clearBrowsingDataBtn) {
    clearBrowsingDataBtn.addEventListener("click", function () {
      if (!invoke) return;
      if (!window.confirm(tt("settings.advanced.maintenance.clear_browsing_data_confirm", "Clear cookies, cache, and history for LUMA and the built-in browser?"))) return;
      invoke("clear_browsing_data")
        .then(function () {
          dlog("info", "settings: cleared browsing data");
          showMaintenanceStatus(tt("settings.advanced.maintenance.clear_browsing_data_done", "Browsing data cleared."));
        })
        .catch(function (err) {
          dlog("error", "clear_browsing_data invoke failed: " + err);
          showMaintenanceStatus(ff("settings.advanced.maintenance.action_failed", [err], "Couldn't do that: {0}"), true);
        });
    });
  }

  var resetDefaultsBtn = document.getElementById("reset-defaults-btn");
  if (resetDefaultsBtn) {
    resetDefaultsBtn.addEventListener("click", function () {
      if (!invoke) return;
      if (!window.confirm(tt("settings.advanced.maintenance.reset_defaults_confirm", "Reset every setting to its default? Themes and translations you added are kept."))) return;
      invoke("reset_to_defaults")
        .then(function () {
          dlog("info", "settings: reset config to defaults, reloading");
          window.location.reload();
        })
        .catch(function (err) {
          dlog("error", "reset_to_defaults invoke failed: " + err);
          showMaintenanceStatus(ff("settings.advanced.maintenance.action_failed", [err], "Couldn't do that: {0}"), true);
        });
    });
  }

  var uninstallBtn = document.getElementById("uninstall-btn");
  if (uninstallBtn) {
    uninstallBtn.addEventListener("click", function () {
      if (!invoke) return;
      if (!window.confirm(tt("settings.advanced.maintenance.uninstall_confirm", "Uninstall LUMA? This closes the app and removes it from your computer. Your settings are kept in case you reinstall."))) return;
      uninstallBtn.disabled = true;
      invoke("uninstall_app").catch(function (err) {
        dlog("error", "uninstall_app invoke failed: " + err);
        uninstallBtn.disabled = false;
        showMaintenanceStatus(ff("settings.advanced.maintenance.action_failed", [err], "Couldn't do that: {0}"), true);
      });
    });
  }

  debugClearBtn.addEventListener("click", function () {
    if (window.LumaDebugLog) window.LumaDebugLog.clear();
    if (invoke) invoke("clear_debug_log").catch(function () {});
    renderLog([]);
  });

  debugCopyBtn.addEventListener("click", function () {
    var text = debugConsoleLog.innerText;
    var done = function () {
      var original = debugCopyBtn.textContent;
      debugCopyBtn.textContent = tt("debug_console.copied", "Copied!");
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
    if (window.LumaDebugLog) window.LumaDebugLog.setEnabled(debugLoggingEnabled.checked);
    persistPatch(function (cfg) { cfg.general.debug_logging = debugLoggingEnabled.checked; }, "debug logging");
    if (debugLoggingEnabled.checked) {
      openDebugConsole();
    }
  });

  var pendingUpdate = null;

  function checkForUpdates() {
    if (!invoke) return;
    if (window.LumaOffline && !window.LumaOffline.isOnline()) {
      updateStatus.textContent = tt("offline.feature_unavailable", "You're offline - can't reach that right now.");
      return;
    }
    checkUpdatesBtn.disabled = true;
    updateStatus.textContent = tt("settings.updates.status_checking_short", "Checking…");
    installUpdateBtn.hidden = true;
    pendingUpdate = null;

    invoke("check_for_update")
      .then(function (status) {
        checkUpdatesBtn.disabled = false;
        if (!status.checked_ok) {
          updateStatus.textContent = ff("settings.updates.check_failed", [status.error], "Couldn't check for updates: {0}");
          dlog("warn", "check_for_update failed: " + status.error);
          return;
        }
        if (status.available) {
          pendingUpdate = status;
          updateStatus.textContent = ff("settings.updates.available", [status.latest_commit], "Update available (build {0}).");
          installUpdateBtn.hidden = false;
          dlog("info", "settings: update available " + status.current_commit + " -> " + status.latest_commit);
        } else {
          updateStatus.textContent = ff("settings.updates.up_to_date", [status.current_commit], "You're up to date (build {0}).");
        }
      })
      .catch(function (err) {
        checkUpdatesBtn.disabled = false;
        updateStatus.textContent = ff("settings.updates.check_failed", [err], "Couldn't check for updates: {0}");
        dlog("error", "check_for_update invoke failed: " + err);
      });
  }

  function installUpdate() {
    if (!invoke || !pendingUpdate) return;
    installUpdateBtn.disabled = true;
    checkUpdatesBtn.disabled = true;
    updateStatus.textContent = tt("settings.updates.status_installing", "Starting the update…");

    invoke("apply_update").catch(function (err) {

      dlog("error", "apply_update invoke failed: " + err);
      updateStatus.textContent = ff("update_banner.failed", [err], "Update failed: {0}");
      installUpdateBtn.disabled = false;
      checkUpdatesBtn.disabled = false;
    });
  }

  checkUpdatesBtn.addEventListener("click", checkForUpdates);
  installUpdateBtn.addEventListener("click", installUpdate);

  checkUpdatesEnabled.addEventListener("change", function () {
    persistPatch(function (cfg) { cfg.general.check_for_updates = checkUpdatesEnabled.checked; }, "check for updates");
  });

  function describePosition(cfg) {
    if (!cfg) return "";
    if (cfg.window.spotlight_position === "custom" &&
        cfg.window.spotlight_custom_x != null && cfg.window.spotlight_custom_y != null) {
      return ff(
        "settings.spotlight.window.position_current_custom",
        [Math.round(cfg.window.spotlight_custom_x * 100), Math.round(cfg.window.spotlight_custom_y * 100)],
        "Currently: a custom picked spot ({0}%, {1}% of your screen)."
      );
    }
    return tt("settings.spotlight.window.position_current_default", "Currently: screen center (default).");
  }

  pickPositionBtn.addEventListener("click", function () {
    if (!invoke) return;
    pickPositionBtn.disabled = true;
    spotlightPositionStatus.textContent = tt("settings.spotlight.window.position_picking", "Click anywhere on your screen…");
    invoke("open_position_picker").catch(function (err) {
      dlog("error", "open_position_picker invoke failed: " + err);
      spotlightPositionStatus.textContent = ff("settings.spotlight.window.position_picker_failed", [err], "Couldn't open the position picker: {0}");
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
        spotlightPositionStatus.textContent = ff("settings.spotlight.window.position_reset_failed", [err], "Couldn't reset the position: {0}");
      });
  });

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

  function renderRemovableRows(container, items, emptyText, describe, onRemove) {
    container.innerHTML = "";
    if (!items.length) {
      var empty = document.createElement("span");
      empty.className = "update-status";
      empty.textContent = emptyText;
      container.appendChild(empty);
      return;
    }
    items.forEach(function (item) {
      var row = document.createElement("div");
      row.className = "engine-row";

      var info = describe(item);
      var label = document.createElement("span");
      label.textContent = info.name + (info.bang ? " " : "");
      if (info.bang) {
        var bang = document.createElement("span");
        bang.className = "bang";
        bang.textContent = "!" + info.bang;
        label.appendChild(bang);
      }
      row.appendChild(label);

      var removeBtn = document.createElement("button");
      removeBtn.type = "button";
      removeBtn.className = "engine-row-remove";
      removeBtn.textContent = "✕";
      removeBtn.title = ff("settings.search.engines.remove_title", [info.name], "Remove {0}");
      removeBtn.addEventListener("click", function () {
        onRemove(item, removeBtn);
      });
      row.appendChild(removeBtn);

      container.appendChild(row);
    });
  }

  function renderCustomEngines(customEngines) {
    renderRemovableRows(
      customEnginesList,
      customEngines,
      tt("settings.search.engines.empty", "No custom engines yet - add one below."),
      function (engine) { return { name: engine.name, bang: engine.bang }; },
      function (engine) {
        invoke("remove_custom_engine", { name: engine.name })
          .then(function () {
            dlog("info", "settings: removed custom engine " + engine.name);
            return loadEngines();
          })
          .catch(function (err) {
            dlog("error", "remove_custom_engine invoke failed: " + err);
            engineFormStatus.textContent = ff("settings.search.engines.remove_failed", [err], "Couldn't remove it: {0}");
          });
      }
    );
  }

  async function loadBuiltinEngines() {
    var allEngines = (await invoke("list_all_builtin_engines")).filter(function (e) {
      return !e.local && e.name !== "Google";
    });
    var enabled = allEngines.filter(function (e) {
      return currentConfig.search.enabled_builtin_engines.indexOf(e.name) !== -1;
    });
    var available = allEngines.filter(function (e) {
      return currentConfig.search.enabled_builtin_engines.indexOf(e.name) === -1;
    });
    renderBuiltinEngines(enabled);
    renderAddBuiltinEngineOptions(available);
  }

  function setBuiltinEngineEnabled(name, enabled) {
    return invoke("set_builtin_engine_enabled", { name: name, enabled: enabled }).then(function () {
      currentConfig.search.enabled_builtin_engines = currentConfig.search.enabled_builtin_engines.filter(
        function (n) { return n !== name; }
      );
      if (enabled) currentConfig.search.enabled_builtin_engines.push(name);
      dlog("info", "settings: " + name + (enabled ? " enabled" : " disabled"));
      return Promise.all([loadEngines(), loadBuiltinEngines()]);
    });
  }

  function renderBuiltinEngines(enabled) {
    renderRemovableRows(
      builtinEnginesList,
      enabled,
      tt("settings.search.more_engines.empty", "None turned on yet - pick one from the dropdown below."),
      function (engine) { return { name: engine.name, bang: engine.bang }; },
      function (engine) {
        setBuiltinEngineEnabled(engine.name, false).catch(function (err) {
          dlog("error", "set_builtin_engine_enabled invoke failed: " + err);
        });
      }
    );
  }

  function renderAddBuiltinEngineOptions(available) {
    addBuiltinEngineSelect.innerHTML = "";
    var placeholder = document.createElement("option");
    placeholder.value = "";
    placeholder.textContent = tt("settings.search.more_engines.add_option", "+ Add a search engine…");
    addBuiltinEngineSelect.appendChild(placeholder);
    available.forEach(function (engine) {
      var opt = document.createElement("option");
      opt.value = engine.name;
      opt.textContent = engine.name + " (!" + engine.bang + ")";
      addBuiltinEngineSelect.appendChild(opt);
    });
    addBuiltinEngineSelect.value = "";
  }

  addBuiltinEngineSelect.addEventListener("change", function () {
    var name = addBuiltinEngineSelect.value;
    if (!name) return;
    setBuiltinEngineEnabled(name, true).catch(function (err) {
      dlog("error", "set_builtin_engine_enabled invoke failed: " + err);
      addBuiltinEngineSelect.value = "";
    });
  });

  function renderCustomApps(customApps) {
    renderRemovableRows(
      customAppsList,
      customApps,
      tt("settings.search.custom_apps.empty", "None yet - !open saves one here the first time it can't find an app on its own."),
      function (appEntry) { return { name: appEntry.name }; },
      function (appEntry) {
        invoke("remove_custom_app", { name: appEntry.name })
          .then(function () {
            dlog("info", "settings: removed custom app " + appEntry.name);
            currentConfig.search.custom_apps = currentConfig.search.custom_apps.filter(function (a) {
              return a.name !== appEntry.name;
            });
            renderCustomApps(currentConfig.search.custom_apps);
          })
          .catch(function (err) {
            dlog("error", "remove_custom_app invoke failed: " + err);
          });
      }
    );
  }

  async function loadPlugins() {
    var plugins = await invoke("list_plugins");
    renderPlugins(plugins);
  }

  function renderPlugins(plugins) {
    pluginsList.innerHTML = "";
    if (!plugins.length) {
      var empty = document.createElement("span");
      empty.className = "update-status";
      empty.textContent = tt("settings.search.plugins.empty", "No plugins installed - grab one from the marketplace below.");
      pluginsList.appendChild(empty);
      return;
    }
    plugins.forEach(function (plugin) {
      var el = document.createElement("label");
      el.className = "theme-option checkbox-option";

      var checkbox = document.createElement("input");
      checkbox.type = "checkbox";
      checkbox.checked = currentConfig.plugins.enabled_plugins.indexOf(plugin.id) !== -1;
      checkbox.addEventListener("change", function () {
        checkbox.disabled = true;
        invoke("set_plugin_enabled", { pluginId: plugin.id, enabled: checkbox.checked })
          .then(function () {
            if (checkbox.checked) {
              if (currentConfig.plugins.enabled_plugins.indexOf(plugin.id) === -1) {
                currentConfig.plugins.enabled_plugins.push(plugin.id);
              }
            } else {
              currentConfig.plugins.enabled_plugins = currentConfig.plugins.enabled_plugins.filter(function (id) { return id !== plugin.id; });
            }
            dlog("info", "settings: plugin " + plugin.id + (checkbox.checked ? " enabled" : " disabled"));
          })
          .catch(function (err) {
            checkbox.checked = !checkbox.checked;
            dlog("error", "set_plugin_enabled invoke failed: " + err);
          })
          .finally(function () {
            checkbox.disabled = false;
          });
      });
      el.appendChild(checkbox);

      var label = document.createElement("span");
      label.textContent = plugin.name + " - " + plugin.author;
      el.appendChild(label);

      if (!plugin.is_builtin) {
        var removeBtn = document.createElement("button");
        removeBtn.type = "button";
        removeBtn.className = "theme-option-remove";
        removeBtn.textContent = "✕";
        removeBtn.title = ff("settings.search.plugins.uninstall_title", [plugin.name], "Uninstall {0}");
        removeBtn.addEventListener("click", function (e) {
          e.preventDefault();
          e.stopPropagation();
          removeBtn.disabled = true;
          invoke("uninstall_plugin", { pluginId: plugin.id })
            .then(async function () {
              dlog("info", "settings: uninstalled plugin " + plugin.id);
              currentConfig = await invoke("get_config");
              await loadPlugins();
              refreshPluginMarketplaceGrid();
            })
            .catch(function (err) {
              removeBtn.disabled = false;
              dlog("error", "settings: uninstall_plugin failed for " + plugin.id + ": " + err);
            });
        });
        el.appendChild(removeBtn);
      }

      pluginsList.appendChild(el);
    });
  }

  var pluginMarketplaceStatus = document.getElementById("plugin-marketplace-status");
  var pluginMarketplaceGrid = document.getElementById("plugin-marketplace-grid");
  var pluginMarketplacePagination = document.getElementById("plugin-marketplace-pagination");
  var pluginMarketplacePrevBtn = document.getElementById("plugin-marketplace-prev-btn");
  var pluginMarketplaceNextBtn = document.getElementById("plugin-marketplace-next-btn");
  var pluginMarketplacePageCounter = document.getElementById("plugin-marketplace-page-counter");
  var pluginMarketplaceUrlInput = document.getElementById("plugin-marketplace-url-input");
  var pluginMarketplaceUrlInstallBtn = document.getElementById("plugin-marketplace-url-install-btn");
  var pluginMarketplaceUrlStatus = document.getElementById("plugin-marketplace-url-status");
  var installedPluginIds = [];
  var pluginMarketplacePage = 0;
  var pluginMarketplaceEntries = [];
  var pluginMarketplaceLoading = false;

  function renderPluginMarketplacePagination(total) {
    var totalPages = Math.max(1, Math.ceil(total / MARKETPLACE_PAGE_SIZE));
    if (pluginMarketplacePage >= totalPages) pluginMarketplacePage = totalPages - 1;
    if (pluginMarketplacePage < 0) pluginMarketplacePage = 0;

    if (total <= MARKETPLACE_PAGE_SIZE) {
      pluginMarketplacePagination.hidden = true;
      return;
    }
    pluginMarketplacePagination.hidden = false;
    pluginMarketplacePrevBtn.disabled = pluginMarketplacePage === 0;
    pluginMarketplaceNextBtn.disabled = pluginMarketplacePage >= totalPages - 1;
    pluginMarketplacePageCounter.textContent = ff(
      "settings.appearance.marketplace.page_counter",
      [pluginMarketplacePage + 1, totalPages],
      "Page {0} of {1}"
    );
  }

  function renderPluginMarketplaceGrid(entries) {
    pluginMarketplaceGrid.innerHTML = "";
    var pageStart = pluginMarketplacePage * MARKETPLACE_PAGE_SIZE;
    var pageEntries = entries.slice(pageStart, pageStart + MARKETPLACE_PAGE_SIZE);
    pageEntries.forEach(function (entry) {
      var card = document.createElement("div");
      card.className = "marketplace-card";

      var name = document.createElement("div");
      name.className = "marketplace-card-name";
      name.textContent = entry.name;
      card.appendChild(name);

      var author = document.createElement("div");
      author.className = "marketplace-card-author";
      author.textContent = ff("settings.appearance.marketplace.by_author", [entry.author], "by {0}");
      card.appendChild(author);

      if (entry.description) {
        var desc = document.createElement("div");
        desc.className = "marketplace-card-desc";
        desc.textContent = entry.description;
        card.appendChild(desc);
      }

      var installed = installedPluginIds.indexOf(entry.id) !== -1;
      var actions = document.createElement("div");
      actions.className = "marketplace-card-actions";

      var btn = document.createElement("button");
      btn.type = "button";
      btn.className = "btn";
      btn.textContent = installed ? tt("settings.appearance.marketplace.installed", "Installed") : tt("settings.appearance.marketplace.install_button", "Install");
      btn.disabled = installed;
      btn.addEventListener("click", function () {
        btn.disabled = true;
        btn.textContent = tt("settings.appearance.marketplace.installing", "Installing…");
        invoke("install_plugin_from_url", {
          jsUrl: entry.js_url,
          jsonUrl: entry.json_url || null,
          idHint: entry.id,
          nameHint: entry.name,
          authorHint: entry.author,
        })
          .then(async function (plugin) {
            await invoke("set_plugin_enabled", { pluginId: plugin.id, enabled: true }).catch(function (err) {
              dlog("error", "set_plugin_enabled invoke failed: " + err);
            });
            if (currentConfig.plugins.enabled_plugins.indexOf(plugin.id) === -1) {
              currentConfig.plugins.enabled_plugins.push(plugin.id);
            }
            await loadPlugins();
            await refreshPluginMarketplaceGrid();
            dlog("info", "settings: installed marketplace plugin " + entry.id);
          })
          .catch(function (err) {
            btn.disabled = false;
            btn.textContent = tt("settings.appearance.marketplace.install_button", "Install");
            dlog("error", "settings: install_plugin_from_url failed for " + entry.id + ": " + err);
            pluginMarketplaceStatus.textContent = ff("settings.search.plugin_marketplace.install_failed", [entry.name, err], "Couldn't install {0}: {1}");
          });
      });
      actions.appendChild(btn);
      card.appendChild(actions);

      pluginMarketplaceGrid.appendChild(card);
    });
    renderPluginMarketplacePagination(entries.length);
  }

  async function refreshPluginMarketplaceGrid() {
    installedPluginIds = (await invoke("list_plugins")).map(function (p) { return p.id; });
    renderPluginMarketplaceGrid(pluginMarketplaceEntries);
  }

  pluginMarketplacePrevBtn.addEventListener("click", function () {
    if (pluginMarketplacePage > 0) {
      pluginMarketplacePage -= 1;
    }
    loadPluginMarketplace({ preservePage: true });
  });

  pluginMarketplaceNextBtn.addEventListener("click", function () {
    var totalPages = Math.max(1, Math.ceil(pluginMarketplaceEntries.length / MARKETPLACE_PAGE_SIZE));
    if (pluginMarketplacePage < totalPages - 1) {
      pluginMarketplacePage += 1;
    }
    loadPluginMarketplace({ preservePage: true });
  });

  async function loadPluginMarketplace(options) {
    options = options || {};
    if (pluginMarketplaceLoading) return;
    if (window.LumaOffline && !window.LumaOffline.isOnline()) {
      pluginMarketplaceStatus.textContent = tt("offline.feature_unavailable", "You're offline - can't reach that right now.");
      return;
    }
    pluginMarketplaceLoading = true;
    pluginMarketplacePrevBtn.disabled = true;
    pluginMarketplaceNextBtn.disabled = true;
    pluginMarketplaceStatus.textContent = tt("settings.search.plugin_marketplace.loading", "Loading the plugin marketplace…");
    try {
      installedPluginIds = (await invoke("list_plugins")).map(function (p) { return p.id; });
      var entries = await invoke("fetch_plugin_marketplace_index");
      pluginMarketplaceEntries = entries || [];
      if (!options.preservePage) pluginMarketplacePage = 0;
      renderPluginMarketplaceGrid(pluginMarketplaceEntries);
      pluginMarketplaceStatus.textContent = pluginMarketplaceEntries.length
        ? ff(
            pluginMarketplaceEntries.length === 1 ? "settings.search.plugin_marketplace.count_singular" : "settings.search.plugin_marketplace.count_plural",
            [pluginMarketplaceEntries.length],
            pluginMarketplaceEntries.length === 1 ? "{0} plugin available" : "{0} plugins available"
          )
        : tt("settings.search.plugin_marketplace.empty", "Nothing here yet - check back soon.");
    } catch (err) {
      dlog("error", "settings: fetch_plugin_marketplace_index failed: " + err);
      pluginMarketplaceStatus.textContent = ff("settings.search.plugin_marketplace.unreachable", [err], "Couldn't reach the plugin marketplace ({0}). Check your connection and reopen Settings.");
      renderPluginMarketplacePagination(pluginMarketplaceEntries.length);
    } finally {
      pluginMarketplaceLoading = false;
    }
  }

  pluginMarketplaceUrlInstallBtn.addEventListener("click", function () {
    var url = pluginMarketplaceUrlInput.value.trim();
    if (!url) return;
    if (window.LumaOffline && !window.LumaOffline.isOnline()) {
      pluginMarketplaceUrlStatus.textContent = tt("offline.feature_unavailable", "You're offline - can't reach that right now.");
      return;
    }
    pluginMarketplaceUrlInstallBtn.disabled = true;
    pluginMarketplaceUrlStatus.textContent = tt("settings.appearance.marketplace.installing", "Installing…");
    invoke("install_plugin_from_url", { jsUrl: url, jsonUrl: null, idHint: null, nameHint: null, authorHint: null })
      .then(async function (plugin) {
        await invoke("set_plugin_enabled", { pluginId: plugin.id, enabled: true }).catch(function (err) {
          dlog("error", "set_plugin_enabled invoke failed: " + err);
        });
        if (currentConfig.plugins.enabled_plugins.indexOf(plugin.id) === -1) {
          currentConfig.plugins.enabled_plugins.push(plugin.id);
        }
        await loadPlugins();
        pluginMarketplaceUrlStatus.textContent = ff("settings.search.plugin_marketplace.url_installed", [plugin.name], "Installed \"{0}\" - it's on above.");
        pluginMarketplaceUrlInput.value = "";
      })
      .catch(function (err) {
        pluginMarketplaceUrlStatus.textContent = ff("settings.search.plugin_marketplace.url_install_failed", [err], "Couldn't install that: {0}");
      })
      .finally(function () {
        pluginMarketplaceUrlInstallBtn.disabled = false;
      });
  });

  addEngineBtn.addEventListener("click", function () {
    if (!invoke) return;
    var engine = {
      name: newEngineName.value.trim(),
      action: newEngineUrl.value.trim(),
      bang: newEngineBang.value.trim(),
      placeholder: "",
    };
    if (!engine.name || !engine.action || !engine.bang) {
      engineFormStatus.textContent = tt("settings.search.engines.validation", "Name, URL, and bang are all required.");
      return;
    }
    addEngineBtn.disabled = true;
    engineFormStatus.textContent = tt("settings.search.engines.adding", "Adding…");
    invoke("add_custom_engine", { engine: engine })
      .then(function () {
        newEngineName.value = "";
        newEngineUrl.value = "";
        newEngineBang.value = "";
        engineFormStatus.textContent = ff("settings.search.engines.added", [engine.name], "Added {0}.");
        dlog("info", "settings: added custom engine " + engine.name);
        return loadEngines();
      })
      .catch(function (err) {
        dlog("error", "add_custom_engine invoke failed: " + err);
        engineFormStatus.textContent = ff("settings.search.engines.add_failed", [err], "Couldn't add it: {0}");
      })
      .finally(function () {
        addEngineBtn.disabled = false;
      });
  });

  async function boot(tauri) {
    invoke = tauri.core.invoke;
    dlog("info", "settings window: boot() starting");

    var themeCss;
    try {
      currentConfig = await invoke("get_config");
      if (window.LumaDebugLog) window.LumaDebugLog.setEnabled(!!(currentConfig.general && currentConfig.general.debug_logging));
      dlog("info", "settings: config loaded");
    } catch (err) {
      dlog("error", "settings: get_config invoke failed: " + err);
      revealBody();
      showFatalBanner("couldn't load your settings (" + err + ") - nothing below will be accurate. Hold Shift+L for the debug log.");
      return;
    }

    if (window.LumaI18n) {
      try {
        await window.LumaI18n.init(invoke, currentConfig.general && currentConfig.general.locale);
        await populateLanguageSelect();
      } catch (err) {
        dlog("error", "settings: LumaI18n.init failed: " + err);
      }
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
    document.body.classList.toggle("no-animations", disableAnimationsCheckbox.checked);
    showSpotlightBrandingCheckbox.checked = !!currentConfig.general.show_spotlight_branding;
    var mainSizeInput = document.querySelector(
      'input[name="main_window_size"][value="' + (currentConfig.window.main_window_size || "roomy") + '"]'
    );
    if (mainSizeInput) mainSizeInput.checked = true;
    startAtLoginCheckbox.checked = !!currentConfig.general.start_at_login;
    checkUpdatesEnabled.checked = currentConfig.general.check_for_updates !== false;
    customCssTextarea.value = currentConfig.appearance.custom_css || "";
    selectedThemeId = currentConfig.appearance.theme;

    tauri.event.listen("luma://update-progress", function (event) {
      var stage = event.payload && event.payload.stage;
      if (stage === "checking") updateStatus.textContent = tt("settings.updates.status_checking", "Checking the release…");
      else if (stage === "downloading") updateStatus.textContent = tt("settings.updates.status_downloading", "Downloading the update…");
      else if (stage === "installing") updateStatus.textContent = tt("settings.updates.status_installing", "Installing - LUMA will restart itself…");
    });

    tauri.event.listen("luma://config-changed", async function () {
      pickPositionBtn.disabled = false;
      try {
        currentConfig = await invoke("get_config");
        if (window.LumaDebugLog) window.LumaDebugLog.setEnabled(!!(currentConfig.general && currentConfig.general.debug_logging));
        if (window.LumaI18n && currentConfig.general && currentConfig.general.locale !== window.LumaI18n.getCurrentLocale()) {
          await window.LumaI18n.init(invoke, currentConfig.general.locale);
        }
        spotlightPositionStatus.textContent = describePosition(currentConfig);
      } catch (err) {
        dlog("warn", "settings: re-fetching config after config-changed failed: " + err);
      }
    });

    if (currentConfig.general.debug_logging) {
      debugSection.hidden = false;
      if (advancedNavBtn) advancedNavBtn.hidden = false;
      debugLoggingEnabled.checked = true;
    }

    try {
      await loadEngines();
      await loadBuiltinEngines();
    } catch (err) {
      dlog("error", "settings: get_engines invoke failed: " + err);
    }
    renderCustomApps(currentConfig.search.custom_apps || []);

    try {
      await loadPlugins();
    } catch (err) {
      dlog("error", "settings: list_plugins invoke failed: " + err);
    }
    loadPluginMarketplace();

    try {
      await refreshThemeList();
    } catch (err) {
      dlog("error", "settings: list_themes invoke failed: " + err);
    }

    loadMarketplace();

    try {
      var info = await invoke("get_system_info");
      buildStamp.textContent = "LUMA " + info.app_version + " · build " + info.build_sha + " · pid " + info.pid;
    } catch (err) {
      buildStamp.textContent = ff("settings.build_stamp_unavailable", [err], "LUMA - build info unavailable ({0})");
    }

    dlog("info", "settings: boot() complete");
    revealBody();
  }

  function renderThemeOptions(themes) {
    themeOptionsContainer.innerHTML = "";
    themes.forEach(function (theme) {
      var el = document.createElement("div");
      el.className = "theme-option" + (theme.id === selectedThemeId ? " selected" : "");

      var label = document.createElement("span");
      label.textContent = theme.name + " - " + theme.author;
      el.appendChild(label);

      el.addEventListener("click", function () {
        selectedThemeId = theme.id;
        document.querySelectorAll(".theme-option").forEach(function (o) { o.classList.remove("selected"); });
        el.classList.add("selected");
        persistPatch(function (cfg) { cfg.appearance.theme = theme.id; }, "theme");
      });

      if (!theme.is_builtin) {
        var removeBtn = document.createElement("button");
        removeBtn.type = "button";
        removeBtn.className = "theme-option-remove";
        removeBtn.textContent = "✕";
        removeBtn.title = ff("settings.appearance.theme.uninstall_title", [theme.name], "Uninstall {0}");
        removeBtn.addEventListener("click", function (e) {
          e.stopPropagation();
          removeBtn.disabled = true;
          invoke("uninstall_theme", { themeId: theme.id })
            .then(async function () {
              dlog("info", "settings: uninstalled theme " + theme.id);
              currentConfig = await invoke("get_config");
              selectedThemeId = currentConfig.appearance.theme;
              await refreshThemeList();
              refreshMarketplaceGrid();
            })
            .catch(function (err) {
              removeBtn.disabled = false;
              dlog("error", "settings: uninstall_theme failed for " + theme.id + ": " + err);
            });
        });
        el.appendChild(removeBtn);
      }

      themeOptionsContainer.appendChild(el);
    });
  }

  openThemesFolderBtn.addEventListener("click", function () {
    if (!invoke) return;
    invoke("reveal_themes_folder").catch(function (err) {
      dlog("error", "reveal_themes_folder invoke failed: " + err);
    });
  });

  openPluginsFolderBtn.addEventListener("click", function () {
    if (!invoke) return;
    invoke("reveal_plugins_folder").catch(function (err) {
      dlog("error", "reveal_plugins_folder invoke failed: " + err);
    });
  });

  var marketplaceStatus = document.getElementById("marketplace-status");
  var marketplaceGrid = document.getElementById("marketplace-grid");
  var marketplacePagination = document.getElementById("marketplace-pagination");
  var marketplacePrevBtn = document.getElementById("marketplace-prev-btn");
  var marketplaceNextBtn = document.getElementById("marketplace-next-btn");
  var marketplacePageCounter = document.getElementById("marketplace-page-counter");
  var marketplaceUrlInput = document.getElementById("marketplace-url-input");
  var marketplaceUrlInstallBtn = document.getElementById("marketplace-url-install-btn");
  var marketplaceUrlStatus = document.getElementById("marketplace-url-status");
  var installedThemeIds = [];
  var MARKETPLACE_PAGE_SIZE = 3;
  var marketplacePage = 0;

  async function refreshThemeList() {
    var themeList = await invoke("list_themes");
    installedThemeIds = themeList.map(function (t) { return t.id; });
    renderThemeOptions(themeList);
    return themeList;
  }

  function renderMarketplacePagination(total) {
    var totalPages = Math.max(1, Math.ceil(total / MARKETPLACE_PAGE_SIZE));
    if (marketplacePage >= totalPages) marketplacePage = totalPages - 1;
    if (marketplacePage < 0) marketplacePage = 0;

    if (total <= MARKETPLACE_PAGE_SIZE) {
      marketplacePagination.hidden = true;
      return;
    }
    marketplacePagination.hidden = false;
    marketplacePrevBtn.disabled = marketplacePage === 0;
    marketplaceNextBtn.disabled = marketplacePage >= totalPages - 1;
    marketplacePageCounter.textContent = ff(
      "settings.appearance.marketplace.page_counter",
      [marketplacePage + 1, totalPages],
      "Page {0} of {1}"
    );
  }

  function renderMarketplaceGrid(entries) {
    marketplaceGrid.innerHTML = "";
    var pageStart = marketplacePage * MARKETPLACE_PAGE_SIZE;
    var pageEntries = entries.slice(pageStart, pageStart + MARKETPLACE_PAGE_SIZE);
    pageEntries.forEach(function (entry) {
      var card = document.createElement("div");
      card.className = "marketplace-card";

      if (entry.colors && entry.colors.length) {
        var swatches = document.createElement("div");
        swatches.className = "marketplace-card-swatches";
        entry.colors.slice(0, 5).forEach(function (hex) {
          var sw = document.createElement("span");
          sw.className = "marketplace-card-swatch";
          sw.style.background = hex;
          swatches.appendChild(sw);
        });
        card.appendChild(swatches);
      }

      var name = document.createElement("div");
      name.className = "marketplace-card-name";
      name.textContent = entry.name;
      card.appendChild(name);

      var author = document.createElement("div");
      author.className = "marketplace-card-author";
      author.textContent = ff("settings.appearance.marketplace.by_author", [entry.author], "by {0}");
      card.appendChild(author);

      if (entry.description) {
        var desc = document.createElement("div");
        desc.className = "marketplace-card-desc";
        desc.textContent = entry.description;
        card.appendChild(desc);
      }

      var installed = installedThemeIds.indexOf(entry.id) !== -1;
      var actions = document.createElement("div");
      actions.className = "marketplace-card-actions";

      var btn = document.createElement("button");
      btn.type = "button";
      btn.className = "btn";
      btn.textContent = installed ? tt("settings.appearance.marketplace.installed", "Installed") : tt("settings.appearance.marketplace.install_button", "Install");
      btn.disabled = installed;
      btn.addEventListener("click", function () {
        btn.disabled = true;
        btn.textContent = tt("settings.appearance.marketplace.installing", "Installing…");
        invoke("install_theme_from_url", {
          cssUrl: entry.css_url,
          jsonUrl: entry.json_url || null,
          idHint: entry.id,
          nameHint: entry.name,
          authorHint: entry.author,
        })
          .then(async function () {
            await refreshThemeList();
            refreshMarketplaceGrid();
            dlog("info", "settings: installed marketplace theme " + entry.id);
          })
          .catch(function (err) {
            btn.disabled = false;
            btn.textContent = tt("settings.appearance.marketplace.install_button", "Install");
            dlog("error", "settings: install_theme_from_url failed for " + entry.id + ": " + err);
            marketplaceStatus.textContent = ff("settings.appearance.marketplace.install_failed", [entry.name, err], "Couldn't install {0}: {1}");
          });
      });
      actions.appendChild(btn);

      if (installed) {
        var uninstallBtn = document.createElement("button");
        uninstallBtn.type = "button";
        uninstallBtn.className = "btn btn-danger";
        uninstallBtn.textContent = tt("settings.appearance.marketplace.uninstall_button", "Uninstall");
        uninstallBtn.addEventListener("click", function () {
          uninstallBtn.disabled = true;
          uninstallBtn.textContent = tt("settings.appearance.marketplace.uninstalling", "Uninstalling…");
          invoke("uninstall_theme", { themeId: entry.id })
            .then(async function () {
              dlog("info", "settings: uninstalled marketplace theme " + entry.id);
              currentConfig = await invoke("get_config");
              selectedThemeId = currentConfig.appearance.theme;
              await refreshThemeList();
              refreshMarketplaceGrid();
            })
            .catch(function (err) {
              uninstallBtn.disabled = false;
              uninstallBtn.textContent = tt("settings.appearance.marketplace.uninstall_button", "Uninstall");
              dlog("error", "settings: uninstall_theme failed for " + entry.id + ": " + err);
              marketplaceStatus.textContent = ff("settings.appearance.marketplace.uninstall_failed", [entry.name, err], "Couldn't uninstall {0}: {1}");
            });
        });
        actions.appendChild(uninstallBtn);
      }

      card.appendChild(actions);

      marketplaceGrid.appendChild(card);
    });
    renderMarketplacePagination(entries.length);
  }

  var marketplaceEntries = [];

  function refreshMarketplaceGrid() {
    renderMarketplaceGrid(marketplaceEntries);
  }

  var marketplaceLoading = false;

  marketplacePrevBtn.addEventListener("click", function () {
    if (marketplacePage > 0) {
      marketplacePage -= 1;
    }
    loadMarketplace({ preservePage: true });
  });

  marketplaceNextBtn.addEventListener("click", function () {
    var totalPages = Math.max(1, Math.ceil(marketplaceEntries.length / MARKETPLACE_PAGE_SIZE));
    if (marketplacePage < totalPages - 1) {
      marketplacePage += 1;
    }
    loadMarketplace({ preservePage: true });
  });

  async function loadMarketplace(options) {
    options = options || {};
    if (marketplaceLoading) return;
    if (window.LumaOffline && !window.LumaOffline.isOnline()) {
      marketplaceStatus.textContent = tt("offline.feature_unavailable", "You're offline - can't reach that right now.");
      return;
    }
    marketplaceLoading = true;
    marketplacePrevBtn.disabled = true;
    marketplaceNextBtn.disabled = true;
    marketplaceStatus.textContent = tt("settings.appearance.marketplace.loading", "Loading the marketplace…");
    try {
      var entries = await invoke("fetch_marketplace_index");
      marketplaceEntries = entries || [];
      if (!options.preservePage) marketplacePage = 0;
      renderMarketplaceGrid(marketplaceEntries);
      marketplaceStatus.textContent = marketplaceEntries.length
        ? ff(
            marketplaceEntries.length === 1 ? "settings.appearance.marketplace.count_singular" : "settings.appearance.marketplace.count_plural",
            [marketplaceEntries.length],
            marketplaceEntries.length === 1 ? "{0} theme available" : "{0} themes available"
          )
        : tt("settings.appearance.marketplace.empty", "Nothing here yet - check back soon.");
    } catch (err) {
      dlog("error", "settings: fetch_marketplace_index failed: " + err);
      marketplaceStatus.textContent = ff("settings.appearance.marketplace.unreachable", [err], "Couldn't reach the marketplace ({0}). Check your connection and reopen Settings.");
      // Keep whatever was already rendered (from a previous successful fetch)
      // rather than blanking the grid out on a transient network error.
      renderMarketplacePagination(marketplaceEntries.length);
    } finally {
      marketplaceLoading = false;
    }
  }

  marketplaceUrlInstallBtn.addEventListener("click", function () {
    var url = marketplaceUrlInput.value.trim();
    if (!url) return;
    if (window.LumaOffline && !window.LumaOffline.isOnline()) {
      marketplaceUrlStatus.textContent = tt("offline.feature_unavailable", "You're offline - can't reach that right now.");
      return;
    }
    marketplaceUrlInstallBtn.disabled = true;
    marketplaceUrlStatus.textContent = tt("settings.appearance.marketplace.installing", "Installing…");
    invoke("install_theme_from_url", { cssUrl: url, jsonUrl: null, idHint: null, nameHint: null, authorHint: null })
      .then(async function (theme) {
        await refreshThemeList();
        marketplaceUrlStatus.textContent = ff("settings.appearance.marketplace.url_installed", [theme.name], "Installed \"{0}\" - pick it above.");
        marketplaceUrlInput.value = "";
      })
      .catch(function (err) {
        marketplaceUrlStatus.textContent = ff("settings.appearance.marketplace.url_install_failed", [err], "Couldn't install that: {0}");
      })
      .finally(function () {
        marketplaceUrlInstallBtn.disabled = false;
      });
  });

  form.addEventListener("submit", function (e) {
    e.preventDefault();
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
      revealBody();
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
      revealBody();
      showFatalBanner("failed to start up (" + (err && err.message ? err.message : err) + ")");
    });
  }

  if (window.LumaOffline) {
    window.LumaOffline.onChange(function (online) {
      checkUpdatesBtn.disabled = !online;
      marketplaceUrlInstallBtn.disabled = !online;
      pluginMarketplaceUrlInstallBtn.disabled = !online;
    });
    checkUpdatesBtn.disabled = !window.LumaOffline.isOnline();
    marketplaceUrlInstallBtn.disabled = !window.LumaOffline.isOnline();
    pluginMarketplaceUrlInstallBtn.disabled = !window.LumaOffline.isOnline();
  }

  setTimeout(revealBody, 4000);
  init();
})();
