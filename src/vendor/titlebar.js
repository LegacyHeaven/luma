(function () {
  var params = new URLSearchParams(window.location.search);
  if (params.get("mode") === "spotlight") return;

  function svg(pathD, viewBox) {
    return (
      '<svg viewBox="' + (viewBox || "0 0 10 10") + '" width="10" height="10" ' +
      'fill="none" stroke="currentColor" stroke-width="1" aria-hidden="true">' +
      '<path d="' + pathD + '"/></svg>'
    );
  }

  var ICONS = {
    minimize: svg("M0 5 H10"),
    maximize: svg('M0.5 0.5 H9.5 V9.5 H0.5 Z'),
    restore: svg("M2.5 0.5 H9.5 V7.5 M0.5 2.5 H7.5 V9.5 H0.5 Z"),
    close: svg("M0 0 L10 10 M10 0 L0 10"),
  };

  function button(name, title, onClick) {
    var b = document.createElement("button");
    b.type = "button";
    b.className = "luma-titlebar-btn luma-titlebar-btn-" + name;
    b.title = title;
    b.setAttribute("aria-label", title);
    b.innerHTML = ICONS[name];
    b.addEventListener("click", onClick);
    return b;
  }

  function mount() {
    if (document.getElementById("luma-titlebar")) return;

    var style = document.createElement("style");
    style.textContent = [
      "body.has-titlebar { padding-top: 44px; }",
      "#luma-titlebar-drag {",
      "  position: fixed; top: 0; left: 0; right: 0; height: 44px;",
      "  z-index: 5;",
      "}",
      "#luma-titlebar-controls {",
      "  position: fixed; top: 0; right: 0; height: 44px; z-index: 7;",
      "  display: flex; align-items: stretch;",
      "}",
      ".luma-titlebar-btn {",
      "  appearance: none; border: none; background: transparent; cursor: pointer;",
      "  width: 46px; height: 44px; display: flex; align-items: center; justify-content: center;",
      "  color: var(--color-gray, #c4c4c4); transition: background-color .12s ease, color .12s ease;",
      "}",
      ".luma-titlebar-btn:hover { background: rgba(255, 255, 255, .08); color: var(--color-white, #fff); }",
      ".luma-titlebar-btn-close:hover { background: #e5484d; color: #fff; }",
      ".luma-titlebar-btn:active { background: rgba(255, 255, 255, .14); }",
      ".luma-titlebar-btn-close:active { background: #c53a3e; }",
    ].join("\n");
    document.head.appendChild(style);

    var dragRegion = document.createElement("div");
    dragRegion.id = "luma-titlebar-drag";
    dragRegion.setAttribute("data-tauri-drag-region", "");
    document.body.appendChild(dragRegion);

    var controls = document.createElement("div");
    controls.id = "luma-titlebar-controls";

    var tauriWindow = window.__TAURI__ && window.__TAURI__.window;
    var current = tauriWindow ? tauriWindow.getCurrentWindow() : null;

    var maximizeBtn = button("maximize", "Maximize", function () {
      if (current) current.toggleMaximize();
    });

    function syncMaximizeIcon() {
      if (!current) return;
      current.isMaximized().then(function (isMax) {
        maximizeBtn.className = "luma-titlebar-btn luma-titlebar-btn-maximize";
        maximizeBtn.innerHTML = isMax ? ICONS.restore : ICONS.maximize;
        maximizeBtn.title = isMax ? "Restore" : "Maximize";
        maximizeBtn.setAttribute("aria-label", maximizeBtn.title);
      });
    }

    controls.appendChild(
      button("minimize", "Minimize", function () {
        if (current) current.minimize();
      })
    );
    controls.appendChild(maximizeBtn);
    controls.appendChild(
      button("close", "Close", function () {
        if (current) current.close();
      })
    );

    document.body.appendChild(controls);
    document.body.classList.add("has-titlebar");

    if (current) {
      syncMaximizeIcon();

      current.onResized(syncMaximizeIcon);
    }
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", mount);
  } else {
    mount();
  }
})();
