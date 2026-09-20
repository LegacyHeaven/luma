(function () {
  "use strict";

  function isOnline() {
    return typeof navigator === "undefined" || navigator.onLine !== false;
  }

  var bannerEl = null;

  function sync() {
    if (isOnline()) {
      if (bannerEl) {
        bannerEl.remove();
        bannerEl = null;
      }
      return;
    }
    if (bannerEl) return;
    bannerEl = document.createElement("div");
    bannerEl.id = "luma-offline-banner";
    bannerEl.className = "luma-update-banner";
    var text = document.createElement("span");
    text.textContent = window.LumaI18n
      ? window.LumaI18n.t("offline.banner", "You're offline - online features are unavailable until your connection is back.")
      : "You're offline - online features are unavailable until your connection is back.";
    bannerEl.appendChild(text);
    document.body.appendChild(bannerEl);
  }

  var listeners = [];
  function fire() {
    sync();
    var online = isOnline();
    listeners.forEach(function (fn) {
      try {
        fn(online);
      } catch (e) {}
    });
  }

  window.addEventListener("online", fire);
  window.addEventListener("offline", fire);
  if (document.body) sync();
  else document.addEventListener("DOMContentLoaded", sync);

  window.LumaOffline = {
    isOnline: isOnline,
    onChange: function (fn) {
      listeners.push(fn);
    },
  };
})();
