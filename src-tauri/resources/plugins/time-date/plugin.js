LumaPlugin.register({
  handle: function (bangWord) {
    var now = new Date();
    if (bangWord === "time") {
      return { text: now.toLocaleTimeString() };
    }
    if (bangWord === "date") {
      return {
        text: now.toLocaleDateString(undefined, {
          weekday: "long",
          year: "numeric",
          month: "long",
          day: "numeric",
        }),
      };
    }
    return null;
  },
});
