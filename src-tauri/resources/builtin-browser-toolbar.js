(function () {
  if (window.__lumaToolbarInjected) return;
  window.__lumaToolbarInjected = true;

  var THEME_VARS = __THEME_VARS_JSON__;

  function invoke(cmd, args) {
    if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke) {
      return window.__TAURI_INTERNALS__.invoke(cmd, args || {});
    }
    return Promise.reject(new Error("Tauri bridge unavailable"));
  }

  function setStyles(el, props) {
    for (var name in props) {
      if (Object.prototype.hasOwnProperty.call(props, name)) {
        el.style.setProperty(name, props[name]);
      }
    }
  }

  function button(label, title, onClick) {
    var b = document.createElement("button");
    b.type = "button";
    b.textContent = label;
    b.title = title;
    setStyles(b, {
      background: "transparent",
      border: "1px solid " + THEME_VARS["luma-border"],
      color: THEME_VARS["luma-text"],
      "border-radius": "4px",
      padding: "3px 9px",
      cursor: "pointer",
      font: "12px " + THEME_VARS["luma-font"],
      "line-height": "1.4",
      transition: "border-color .15s ease",
    });
    b.addEventListener("mouseenter", function () {
      b.style.setProperty("border-color", THEME_VARS["luma-accent"]);
    });
    b.addEventListener("mouseleave", function () {
      b.style.setProperty("border-color", THEME_VARS["luma-border"]);
    });
    b.addEventListener("click", onClick);
    return b;
  }

  function windowButton(label, title, onClick, closeStyle) {
    var b = document.createElement("button");
    b.type = "button";
    b.textContent = label;
    b.title = title;
    setStyles(b, {
      background: "transparent",
      border: "none",
      color: THEME_VARS["luma-muted"],
      width: "34px",
      height: "34px",
      cursor: "pointer",
      font: "13px " + THEME_VARS["luma-font"],
      transition: "background-color .12s ease,color .12s ease",
      "flex-shrink": "0",
    });
    b.addEventListener("mouseenter", function () {
      b.style.setProperty("background-color", closeStyle ? "#e5484d" : "rgba(255,255,255,.08)");
      b.style.setProperty("color", "#fff");
    });
    b.addEventListener("mouseleave", function () {
      b.style.setProperty("background-color", "transparent");
      b.style.setProperty("color", THEME_VARS["luma-muted"]);
    });
    b.addEventListener("click", onClick);
    return b;
  }

  function mount() {
    if (document.getElementById("__luma_toolbar__")) return;

    var bar = document.createElement("div");
    bar.id = "__luma_toolbar__";

    bar.setAttribute("data-tauri-drag-region", "");
    setStyles(bar, {
      position: "fixed",
      top: "0",
      left: "0",
      right: "0",
      height: "34px",
      "z-index": "2147483647",
      display: "flex",
      "align-items": "center",
      gap: "6px",
      padding: "0 0 0 8px",
      background: THEME_VARS["luma-bg"],
      "backdrop-filter": "blur(6px)",
      "-webkit-backdrop-filter": "blur(6px)",
      "font-family": THEME_VARS["luma-font"],
      color: THEME_VARS["luma-text"],
      "border-bottom": "1px solid " + THEME_VARS["luma-border"],
      "box-sizing": "border-box",
    });

    bar.appendChild(button("←", "Back", function () { history.back(); }));
    bar.appendChild(button("→", "Forward", function () { history.forward(); }));
    bar.appendChild(button("↻", "Reload", function () { location.reload(); }));

    var urlLabel = document.createElement("span");
    urlLabel.textContent = location.href;
    setStyles(urlLabel, {
      flex: "1",
      overflow: "hidden",
      "text-overflow": "ellipsis",
      "white-space": "nowrap",
      color: THEME_VARS["luma-muted"],
      opacity: ".85",
      padding: "0 8px",
      "font-size": "12px",
    });
    bar.appendChild(urlLabel);

    bar.appendChild(button("Open in system browser", "Open this page in your default browser instead", function () {
      invoke("open_in_system_browser", { url: location.href });
    }));

    var tauriWindow = window.__TAURI__ && window.__TAURI__.window;
    var current = tauriWindow ? tauriWindow.getCurrentWindow() : null;

    bar.appendChild(windowButton("—", "Minimize", function () {
      if (current) current.minimize();
    }));
    bar.appendChild(windowButton("▢", "Maximize / restore", function () {
      if (current) current.toggleMaximize();
    }));

    bar.appendChild(windowButton("✕", "Close this window", function () {
      invoke("close_builtin_browser", {});
    }, true));

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

  setTimeout(mount, 400);
  setTimeout(mount, 1200);
})();
