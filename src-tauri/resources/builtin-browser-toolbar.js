/**
 * Injected (via WebviewWindowBuilder::initialization_script) into Luma's
 * "built-in browser" window, on top of whatever external site the user
 * searched to. Adds a minimal back/forward/reload bar plus an escape
 * hatch to the system browser, since this window has no native chrome.
 *
 * __THEME_VARS__ below is replaced with a handful of real CSS custom
 * property declarations (see window.rs's theme_vars_css) before this
 * script is ever injected - this window shows *external* page content
 * that never loads any of Luma's own stylesheets, so a plain
 * `var(--color-purple)` wouldn't resolve to anything here on its own.
 * Templating the actual values in is what lets this toolbar match
 * whichever theme is currently selected, instead of one hardcoded look.
 */
(function () {
  if (window.__lumaToolbarInjected) return;
  window.__lumaToolbarInjected = true;

  var THEME_VARS = "__THEME_VARS__";

  function invoke(cmd, args) {
    if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke) {
      return window.__TAURI_INTERNALS__.invoke(cmd, args || {});
    }
    return Promise.reject(new Error("Tauri bridge unavailable"));
  }

  function injectThemeVars() {
    if (document.getElementById("__luma_toolbar_vars__")) return;
    var style = document.createElement("style");
    style.id = "__luma_toolbar_vars__";
    style.textContent = ":root {\n" + THEME_VARS + "}";
    document.documentElement.appendChild(style);
  }

  function button(label, title, onClick) {
    var b = document.createElement("button");
    b.type = "button";
    b.textContent = label;
    b.title = title;
    b.style.cssText =
      "background:transparent;border:1px solid var(--luma-border,#3a1f5c);" +
      "color:var(--luma-text,#fff);border-radius:4px;padding:3px 9px;cursor:pointer;" +
      "font:12px var(--luma-font,monospace);line-height:1.4;transition:border-color .15s ease;";
    b.addEventListener("mouseenter", function () {
      b.style.borderColor = "var(--luma-accent, #cf59e6)";
    });
    b.addEventListener("mouseleave", function () {
      b.style.borderColor = "var(--luma-border, #3a1f5c)";
    });
    b.addEventListener("click", onClick);
    return b;
  }

  function mount() {
    if (document.getElementById("__luma_toolbar__")) return;
    injectThemeVars();

    var bar = document.createElement("div");
    bar.id = "__luma_toolbar__";
    bar.style.cssText = [
      "position:fixed", "top:0", "left:0", "right:0", "height:34px",
      "z-index:2147483647", "display:flex", "align-items:center", "gap:6px",
      "padding:0 8px", "background:var(--luma-bg, #180d29)", "backdrop-filter:blur(6px)",
      "-webkit-backdrop-filter:blur(6px)",
      "font-family:var(--luma-font, monospace)", "color:var(--luma-text, #fff)",
      "border-bottom:1px solid var(--luma-border, #3a1f5c)", "box-sizing:border-box",
    ].join(";");

    bar.appendChild(button("←", "Back", function () { history.back(); }));
    bar.appendChild(button("→", "Forward", function () { history.forward(); }));
    bar.appendChild(button("↻", "Reload", function () { location.reload(); }));

    var urlLabel = document.createElement("span");
    urlLabel.textContent = location.href;
    urlLabel.style.cssText =
      "flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;" +
      "color:var(--luma-muted, #c4c4c4);opacity:.85;padding:0 8px;font-size:12px;";
    bar.appendChild(urlLabel);

    bar.appendChild(button("Open in system browser", "Open this page in your default browser instead", function () {
      invoke("open_in_system_browser", { url: location.href });
    }));

    bar.appendChild(button("✕", "Close this window", function () {
      invoke("close_builtin_browser", {});
    }));

    var spacer = document.createElement("div");
    spacer.style.height = "34px";

    document.documentElement.appendChild(bar);
    if (document.body) {
      document.body.style.marginTop = "34px";
    }
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", mount);
  } else {
    mount();
  }
  // Some sites replace <body> after their own JS runs - retry briefly.
  setTimeout(mount, 400);
  setTimeout(mount, 1200);
})();
