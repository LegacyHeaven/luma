(function (root, factory) {
  if (typeof module === "object" && module.exports) {
    module.exports = factory();
  } else {
    root.BangDeckModule = factory();
  }
})(typeof self !== "undefined" ? self : this, function () {
  "use strict";

  class BangDeck {

    constructor(config) {
      if (!config || !Array.isArray(config.engines)) {
        throw new Error("BangDeck: config.engines must be an array");
      }
      this.engines = {};
      this.bangMap = {};
      config.engines.forEach((e) => this._register(e));
      this.defaultEngine = config.defaultEngine && this.engines[config.defaultEngine]
        ? config.defaultEngine
        : config.engines[0].name;
    }

    _register(engine) {
      if (!engine || !engine.name || !engine.bang) return;

      if (!engine.local && !engine.action) return;
      this.engines[engine.name] = engine;
      this.bangMap[engine.bang.toLowerCase()] = engine.name;
    }

    addEngines(list) {
      (list || []).forEach((e) => this._register(e));
    }

    reset(config) {
      if (!config || !Array.isArray(config.engines) || !config.engines.length) return;
      this.engines = {};
      this.bangMap = {};
      config.engines.forEach((e) => this._register(e));
      this.defaultEngine = config.defaultEngine && this.engines[config.defaultEngine]
        ? config.defaultEngine
        : (this.engines[this.defaultEngine] ? this.defaultEngine : config.engines[0].name);
    }

    removeEngine(name) {
      const e = this.engines[name];
      if (!e) return;
      delete this.bangMap[e.bang.toLowerCase()];
      delete this.engines[name];
    }

    listEngines() {
      return Object.values(this.engines).filter((e) => !e.local);
    }

    placeholderFor(name) {
      const e = this.engines[name];
      if (e && e.placeholder) return e.placeholder;
      return window.LumaI18n ? window.LumaI18n.t("search.placeholder_fallback", "search") : "search";
    }

    _prefixMatches(prefix, cfg) {
      return cfg.plugin_id ? prefix === "@" : prefix === "!";
    }

    parseQuery(raw) {
      const trimmed = (raw || "").trim();
      const match = trimmed.match(/^([!@])(\S+)\s+([\s\S]+)$/);
      if (match) {
        const bangWord = match[2].toLowerCase();
        const engineKey = this.bangMap[bangWord];
        const cfg = engineKey && this.engines[engineKey];
        if (cfg && this._prefixMatches(match[1], cfg)) {
          return { engine: engineKey, query: match[3].trim() };
        }
      }
      const bareMatch = trimmed.match(/^([!@])(\S+)$/);
      if (bareMatch) {
        const engineKey = this.bangMap[bareMatch[2].toLowerCase()];
        const cfg = engineKey && this.engines[engineKey];
        if (cfg && cfg.plugin_id && bareMatch[1] === "@") {
          return { engine: engineKey, query: "" };
        }
      }
      return { engine: null, query: trimmed };
    }

    peekBangEngine(raw) {
      const match = (raw || "").match(/^([!@])(\S+)/);
      if (!match) return null;
      const engineKey = this.bangMap[match[2].toLowerCase()];
      const cfg = engineKey && this.engines[engineKey];
      return cfg && this._prefixMatches(match[1], cfg) ? engineKey : null;
    }

    buildSearchUrl(engineKey, query) {
      const cfg = this.engines[engineKey];
      if (!cfg || !query || cfg.local) return null;

      if (cfg.template) {
        return cfg.action.replace(/%s/g, encodeURIComponent(query));
      }
      if (cfg.custom) {
        return cfg.action + encodeURIComponent(query);
      }
      const url = new URL(cfg.action);
      if (cfg.extra) {
        Object.keys(cfg.extra).forEach((k) => url.searchParams.set(k, cfg.extra[k]));
      }
      url.searchParams.set(cfg.param, query);
      return url.toString();
    }

    resolve(rawInput, fallbackEngine) {
      const parsed = this.parseQuery(rawInput);
      const engineKey = parsed.engine || fallbackEngine || this.defaultEngine;
      const cfg = this.engines[engineKey];

      const bang = parsed.engine && cfg ? cfg.bang : null;

      if (cfg && cfg.local) {
        if (!parsed.query && !cfg.plugin_id) return null;
        return { engine: engineKey, query: parsed.query, url: null, local: true, bang };
      }

      const url = this.buildSearchUrl(engineKey, parsed.query);
      if (!url) return null;
      return { engine: engineKey, query: parsed.query, url, bang };
    }
  }

  return { BangDeck };
});
