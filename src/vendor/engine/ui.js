/**
 * Luma search UI controller - the DOM behavior for the search box, shared
 * by the main window and the spotlight window (both use src/index.html).
 *
 * It expects a fixed DOM shape (see src/index.html):
 *
 *   <div class="engine-selector" id="engine-selector">
 *     <button id="current-engine"></button>
 *     <div id="engine-list" role="listbox"></div>
 *   </div>
 *   <div class="search-box">
 *     <form id="search-form"><input id="search-input"></form>
 *   </div>
 *   <div id="particles" class="particles"></div>
 *
 * Opening the resolved URL is delegated to `onSearch`, since that differs
 * between the system browser and Luma's built-in browser window.
 */
(function (root, factory) {
  if (typeof module === "object" && module.exports) {
    module.exports = factory();
  } else {
    root.LumaUI = factory();
  }
})(typeof self !== "undefined" ? self : this, function () {
  "use strict";

  /**
   * @param {Object} opts
   * @param {import('./bangdeck.js').BangDeck} opts.deck   a constructed BangDeck instance
   * @param {Document} [opts.document]
   * @param {(result: {engine:string, query:string, url:string}, newTab: boolean) => void} opts.onSearch
   * @param {() => string|null} [opts.getPreferredEngine]  read last-chosen engine (persistence)
   * @param {(engine: string) => void} [opts.setPreferredEngine]
   * @param {boolean} [opts.particles=true]
   * @param {boolean} [opts.autofocus=true]
   */
  function mount(opts) {
    const doc = opts.document || document;
    const deck = opts.deck;
    const onSearch = opts.onSearch || function () {};
    const getPreferredEngine = opts.getPreferredEngine || function () { return null; };
    const setPreferredEngine = opts.setPreferredEngine || function () {};

    const currentEngineEl = doc.getElementById("current-engine");
    const engineListEl = doc.getElementById("engine-list");
    const engineSelectorEl = doc.getElementById("engine-selector");
    const formEl = doc.getElementById("search-form");
    const inputEl = doc.getElementById("search-input");

    if (!currentEngineEl || !engineListEl || !engineSelectorEl || !formEl || !inputEl) {
      throw new Error("LumaUI.mount: expected search-shell DOM elements were not found");
    }

    let currentEngine = getPreferredEngine() || deck.defaultEngine;

    function renderEngineList() {
      engineListEl.innerHTML = "";
      deck.listEngines().forEach((engine, i) => {
        const btn = doc.createElement("button");
        btn.type = "button";
        btn.dataset.engine = engine.name;
        btn.setAttribute("role", "option");
        btn.style.animationDelay = (i * 0.02) + "s";
        if (engine.name === currentEngine) btn.classList.add("selected");

        const label = doc.createElement("span");
        label.textContent = engine.name;
        btn.appendChild(label);

        const bang = doc.createElement("span");
        bang.className = "bang";
        bang.textContent = "!" + engine.bang;
        btn.appendChild(bang);

        btn.addEventListener("click", (e) => {
          e.stopPropagation();
          setEngine(engine.name);
          closeDropdown();
        });

        engineListEl.appendChild(btn);
      });
    }

    function setEngine(name) {
      if (!deck.engines[name]) return;
      currentEngine = name;
      setPreferredEngine(name);
      inputEl.placeholder = deck.placeholderFor(name);
      refreshEngineDisplay(true);
    }

    function setEngineText(text, animate) {
      if (!animate) {
        currentEngineEl.textContent = text;
        return;
      }
      currentEngineEl.style.transform = "scale(0.9)";
      currentEngineEl.style.opacity = "0.7";
      setTimeout(() => {
        currentEngineEl.textContent = text;
        currentEngineEl.style.transform = "scale(1)";
        currentEngineEl.style.opacity = "1";
      }, 150);
    }

    function refreshEngineDisplay(animate) {
      const bangEngine = deck.peekBangEngine(inputEl.value);
      const displayEngine = bangEngine || currentEngine;
      setEngineText(displayEngine, animate);
      currentEngineEl.classList.toggle("bang-override", !!bangEngine);
      engineListEl.querySelectorAll("button").forEach((b) => {
        b.classList.toggle("selected", b.dataset.engine === displayEngine);
      });
    }

    function openDropdown() {
      engineSelectorEl.classList.add("active");
      currentEngineEl.setAttribute("aria-expanded", "true");
    }

    function closeDropdown() {
      engineSelectorEl.classList.remove("active");
      currentEngineEl.setAttribute("aria-expanded", "false");
    }

    function toggleDropdown(e) {
      e.stopPropagation();
      engineSelectorEl.classList.contains("active") ? closeDropdown() : openDropdown();
    }

    currentEngineEl.addEventListener("click", toggleDropdown);
    currentEngineEl.addEventListener("keydown", (e) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        toggleDropdown(e);
      }
    });

    doc.addEventListener("click", (e) => {
      if (!engineSelectorEl.contains(e.target)) closeDropdown();
    });

    function doSearch(newTab) {
      const result = deck.resolve(inputEl.value, currentEngine);
      if (!result) return;
      onSearch(result, newTab);
    }

    formEl.addEventListener("submit", (e) => {
      e.preventDefault();
      doSearch(true);
    });

    inputEl.addEventListener("input", () => refreshEngineDisplay(false));

    doc.addEventListener("keydown", (e) => {
      if (!e.ctrlKey && !e.altKey && !e.metaKey && e.key.length === 1) {
        if (doc.activeElement !== inputEl) inputEl.focus();
      }
      if (e.key === "Escape") {
        if (engineSelectorEl.classList.contains("active")) {
          closeDropdown();
        } else {
          inputEl.blur();
          inputEl.value = "";
          refreshEngineDisplay(false);
        }
      }
    });

    function createParticles() {
      const container = doc.getElementById("particles");
      if (!container) return;
      container.innerHTML = "";
      const count = 25;
      for (let i = 0; i < count; i++) {
        const particle = doc.createElement("div");
        particle.className = "particle";
        particle.style.left = Math.random() * 100 + "%";
        particle.style.animationDelay = Math.random() * 20 + "s";
        particle.style.animationDuration = (15 + Math.random() * 10) + "s";
        container.appendChild(particle);
      }
    }

    function focusInput() {
      inputEl.focus();
      inputEl.select();
    }

    // ----- init -----
    renderEngineList();
    setEngine(currentEngine);
    if (opts.particles !== false) createParticles();

    if (opts.autofocus !== false) {
      setTimeout(focusInput, 300);
    }

    return {
      setEngine,
      focusInput,
      clear: () => { inputEl.value = ""; refreshEngineDisplay(false); },
      refresh: refreshEngineDisplay,
    };
  }

  return { mount };
});
