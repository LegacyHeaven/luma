/**
 * Luma desktop app - Settings page. Loads the current config.toml (via
 * get_config), lets you edit it, and writes it back with save_config.
 */
(function () {
  "use strict";

  var tauri = window.__TAURI__;
  var invoke = tauri.core.invoke;

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
  var spotlightPositionSelect = document.getElementById("spotlight-position");
  var startAtLoginCheckbox = document.getElementById("start-at-login");
  var themeOptionsContainer = document.getElementById("theme-options");
  var customCssTextarea = document.getElementById("custom-css");
  var openThemesFolderBtn = document.getElementById("open-themes-folder");
  var form = document.getElementById("settings-form");
  var saveStatus = document.getElementById("save-status");

  var currentConfig = null;
  var selectedThemeId = null;

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

  // ----- load current config + supporting data -----
  async function boot() {
    var themeCss;
    try {
      currentConfig = await invoke("get_config");
    } catch (err) {
      console.error("luma: failed to load config", err);
      return;
    }

    try {
      themeCss = await invoke("get_theme_css", { themeId: currentConfig.appearance.theme });
      var style = document.createElement("style");
      style.textContent = themeCss;
      document.head.appendChild(style);
    } catch (err) {
      console.error("luma: failed to load theme", err);
    }

    shortcutInput.value = currentConfig.general.shortcut;
    document.querySelector('input[name="browser_mode"][value="' + currentConfig.general.browser_mode + '"]').checked = true;
    closeOnBlurCheckbox.checked = !!currentConfig.general.close_spotlight_on_blur;
    spotlightWidthInput.value = currentConfig.window.spotlight_width;
    spotlightPositionSelect.value = currentConfig.window.spotlight_position;
    startAtLoginCheckbox.checked = !!currentConfig.general.start_at_login;
    customCssTextarea.value = currentConfig.appearance.custom_css || "";
    selectedThemeId = currentConfig.appearance.theme;

    try {
      var engineConfig = await invoke("get_engines");
      (engineConfig.engines || []).forEach(function (engine) {
        var opt = document.createElement("option");
        opt.value = engine.name;
        opt.textContent = engine.name + " (!" + engine.bang + ")";
        defaultEngineSelect.appendChild(opt);
      });
      defaultEngineSelect.value = currentConfig.general.default_engine;
    } catch (err) {
      console.error("luma: failed to load engines", err);
    }

    try {
      var themeList = await invoke("list_themes");
      renderThemeOptions(themeList);
    } catch (err) {
      console.error("luma: failed to list themes", err);
    }
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
    invoke("reveal_themes_folder").catch(function (err) {
      console.error("luma: could not open themes folder", err);
    });
  });

  form.addEventListener("submit", async function (e) {
    e.preventDefault();

    var browserModeInput = document.querySelector('input[name="browser_mode"]:checked');

    var updated = JSON.parse(JSON.stringify(currentConfig));
    updated.general.shortcut = shortcutInput.value || "Alt+Space";
    updated.general.browser_mode = browserModeInput ? browserModeInput.value : "system";
    updated.general.default_engine = defaultEngineSelect.value || updated.general.default_engine;
    updated.general.close_spotlight_on_blur = closeOnBlurCheckbox.checked;
    updated.general.start_at_login = startAtLoginCheckbox.checked;
    updated.window.spotlight_width = parseInt(spotlightWidthInput.value, 10) || 640;
    updated.window.spotlight_position = spotlightPositionSelect.value;
    updated.appearance.theme = selectedThemeId || updated.appearance.theme;
    updated.appearance.custom_css = customCssTextarea.value || "";

    try {
      await invoke("save_config", { newConfig: updated });
      currentConfig = updated;
      saveStatus.textContent = "Saved.";
      saveStatus.classList.add("visible");
      setTimeout(function () { saveStatus.classList.remove("visible"); }, 2000);
    } catch (err) {
      console.error("luma: failed to save config", err);
      saveStatus.textContent = "Failed to save - see console.";
      saveStatus.classList.add("visible");
    }
  });

  boot();
})();
