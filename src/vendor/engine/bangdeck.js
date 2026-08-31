/**
 * BangDeck - the search/bang engine that powers Luma's search box.
 * Framework-agnostic, dependency-free, loaded directly as a script tag.
 *
 *   const deck = new BangDeckModule.BangDeck(engineConfig);
 *   deck.resolve("!yt lofi beats"); // -> { engine: "YouTube", url: "..." }
 */
(function (root, factory) {
  if (typeof module === "object" && module.exports) {
    module.exports = factory();
  } else {
    root.BangDeckModule = factory();
  }
})(typeof self !== "undefined" ? self : this, function () {
  "use strict";

  /**
   * @typedef {Object} EngineDef
   * @property {string} name
   * @property {string} action     base URL for the search request
   * @property {string} [param]    query-string key (omit when `custom` is true)
   * @property {boolean} [custom]  when true, the query is appended directly to `action`
   * @property {Object<string,string>} [extra] extra fixed query params
   * @property {string} bang       bang word, without the leading "!"
   * @property {string} [placeholder]
   */

  /**
   * BangDeck resolves "!bang query" / plain "query" text into a concrete
   * search URL for a configured set of engines. It owns no DOM and no
   * network calls - callers decide how to open the resulting URL
   * (new tab, redirect, in-app browser window, system browser, ...).
   */
  class BangDeck {
    /**
     * @param {{defaultEngine?: string, engines: EngineDef[]}} config
     */
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
      if (!engine || !engine.name || !engine.action || !engine.bang) return;
      this.engines[engine.name] = engine;
      this.bangMap[engine.bang.toLowerCase()] = engine.name;
    }

    /** Merge in (or overwrite) engines at runtime - used for user custom bangs. */
    addEngines(list) {
      (list || []).forEach((e) => this._register(e));
    }

    /** Remove an engine by name (e.g. a user disabling a default one). */
    removeEngine(name) {
      const e = this.engines[name];
      if (!e) return;
      delete this.bangMap[e.bang.toLowerCase()];
      delete this.engines[name];
    }

    listEngines() {
      return Object.values(this.engines);
    }

    placeholderFor(name) {
      const e = this.engines[name];
      return (e && e.placeholder) || "search";
    }

    /**
     * "!yt lofi beats" -> { engine: "YouTube", query: "lofi beats" }
     * "plain text"     -> { engine: null, query: "plain text" }
     * Unknown bang words are treated as literal query text (so "!" typos
     * don't silently vanish).
     */
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

    /** Live-preview helper: which engine would a partially-typed "!bang" resolve to right now? */
    peekBangEngine(raw) {
      const match = (raw || "").match(/^!(\S+)/);
      if (!match) return null;
      return this.bangMap[match[1].toLowerCase()] || null;
    }

    buildSearchUrl(engineKey, query) {
      const cfg = this.engines[engineKey];
      if (!cfg || !query) return null;
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

    /**
     * Resolve raw input text (which may contain a "!bang") plus a fallback
     * engine into a final { engine, query, url } result. Returns null when
     * there's nothing searchable (empty query).
     */
    resolve(rawInput, fallbackEngine) {
      const parsed = this.parseQuery(rawInput);
      const engineKey = parsed.engine || fallbackEngine || this.defaultEngine;
      const url = this.buildSearchUrl(engineKey, parsed.query);
      if (!url) return null;
      return { engine: engineKey, query: parsed.query, url };
    }
  }

  return { BangDeck };
});
