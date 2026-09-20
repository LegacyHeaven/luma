// Example Luma plugin - reference only. Not shipped with Luma, not listed in the marketplace.
//
// A plugin registers one handler. Luma calls it whenever the user types one of
// the @bangs listed in plugin.json. Return { text } to answer inline, or
// return null to fall through to a normal web search.
LumaPlugin.register({
  handle: function (bangWord, query) {
    if (bangWord === "example") {
      var q = (query || "").trim();
      return { text: q ? "You said: " + q : "Type something after !example." };
    }
    return null;
  },
});
