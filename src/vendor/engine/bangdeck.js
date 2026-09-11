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
      return (e && e.placeholder) || "search";
    }

    parseQuery(raw) {
      const trimmed = (raw || "").trim();
      const match = trimmed.match(/^!(\S+)\s+([\s\S]+)$/);
      if (match) {
        const bangWord = match[1].toLowerCase();
        const engineKey = this.bangMap[bangWord];
        if (engineKey) {
          return { engine: engineKey, query: match[2].trim() };
        }
      }
      return { engine: null, query: trimmed };
    }

    peekBangEngine(raw) {
      const match = (raw || "").match(/^!(\S+)/);
      if (!match) return null;
      return this.bangMap[match[1].toLowerCase()] || null;
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

      if (cfg && cfg.local) {
        if (!parsed.query) return null;
        return { engine: engineKey, query: parsed.query, url: null, local: true };
      }

      const url = this.buildSearchUrl(engineKey, parsed.query);
      if (!url) return null;
      return { engine: engineKey, query: parsed.query, url };
    }
  }

  return { BangDeck };
});
