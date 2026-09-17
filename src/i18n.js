window.LumaI18n = (function () {
  "use strict";

  var strings = {};
  var locales = [{ id: "en", name: "English" }];
  var currentLocale = "en";

  function dlog(level, message) {
    if (window.LumaDebugLog) window.LumaDebugLog.record(level, message);
  }

  function has(key) {
    return Object.prototype.hasOwnProperty.call(strings, key);
  }

  function t(key, fallback) {
    if (has(key)) return strings[key];
    return fallback !== undefined ? fallback : key;
  }

  function format(key, params, fallback) {
    var template = t(key, fallback);
    params = params || [];
    return template.replace(/\{(\d+)\}/g, function (whole, index) {
      var i = parseInt(index, 10);
      return i < params.length ? String(params[i]) : whole;
    });
  }

  function buildRichNodes(text) {
    var frag = document.createDocumentFragment();
    var re = /\{code\}([\s\S]*?)\{\/code\}/g;
    var lastIndex = 0;
    var match;
    while ((match = re.exec(text))) {
      if (match.index > lastIndex) frag.appendChild(document.createTextNode(text.slice(lastIndex, match.index)));
      var code = document.createElement("code");
      code.textContent = match[1];
      frag.appendChild(code);
      lastIndex = re.lastIndex;
    }
    if (lastIndex < text.length) frag.appendChild(document.createTextNode(text.slice(lastIndex)));
    return frag;
  }

  function applyOne(el) {
    var key = el.getAttribute("data-i18n");
    if (key) el.textContent = t(key, el.textContent);

    var richKey = el.getAttribute("data-i18n-rich");
    if (richKey && has(richKey)) {
      el.textContent = "";
      el.appendChild(buildRichNodes(strings[richKey]));
    }

    var phKey = el.getAttribute("data-i18n-placeholder");
    if (phKey) el.setAttribute("placeholder", t(phKey, el.getAttribute("placeholder") || ""));

    var titleKey = el.getAttribute("data-i18n-title");
    if (titleKey) el.setAttribute("title", t(titleKey, el.getAttribute("title") || ""));

    var ariaKey = el.getAttribute("data-i18n-aria-label");
    if (ariaKey) el.setAttribute("aria-label", t(ariaKey, el.getAttribute("aria-label") || ""));
  }

  function apply(root) {
    root = root || document;
    var selector = "[data-i18n], [data-i18n-rich], [data-i18n-placeholder], [data-i18n-title], [data-i18n-aria-label]";
    if (root.matches && root.matches(selector)) applyOne(root);
    var nodes = root.querySelectorAll ? root.querySelectorAll(selector) : [];
    for (var i = 0; i < nodes.length; i++) applyOne(nodes[i]);
  }

  async function load(invoke, localeId) {
    localeId = localeId || "en";
    try {
      strings = await invoke("get_locale_strings", { localeId: localeId });
      currentLocale = localeId;
      dlog("info", "i18n: loaded locale '" + localeId + "' (" + Object.keys(strings).length + " strings)");
    } catch (err) {
      strings = {};
      currentLocale = "en";
      dlog("error", "get_locale_strings invoke failed: " + err);
    }
    return strings;
  }

  async function loadLocaleList(invoke) {
    try {
      locales = await invoke("list_locales");
    } catch (err) {
      dlog("error", "list_locales invoke failed: " + err);
    }
    return locales;
  }

  async function init(invoke, localeId, root) {
    await load(invoke, localeId);
    apply(root || document);
    return strings;
  }

  return {
    init: init,
    load: load,
    loadLocaleList: loadLocaleList,
    apply: apply,
    t: t,
    format: format,
    has: has,
    richNodes: buildRichNodes,
    getLocales: function () { return locales; },
    getCurrentLocale: function () { return currentLocale; },
  };
})();
