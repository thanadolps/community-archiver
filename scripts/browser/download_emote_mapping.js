// Run this script in the browser console
// run on any post, not on community page (ie. `/community?lb=UgkxAXc-DoH5NBBH2snr-ivZiKP9mwAljdda` NOT `/community`)

async function emote_mapping(include_custom = true, include_default = false) {
  // activate emoji picker
  document.querySelector("#placeholder-area").click();
  await sleep(500);
  document.querySelector("#emoji-button").click();
  await sleep(3000);

  /** @type {HTMLImageElement[]} */
  const emojis = [];
  if (include_custom) {
    emojis.push(
      ...document.querySelectorAll(
        "#emoji-picker #emoji[role=listbox].CATEGORY_TYPE_CUSTOM img[role=option]"
      )
    );
  }
  if (include_default) {
    emojis.push(
      ...document.querySelectorAll(
        "#emoji-picker #emoji[role=listbox].CATEGORY_TYPE_GLOBAL img[role=option]"
      )
    );
  }

  const mapping = Object.fromEntries(
    emojis.map((img) => {
      const name = img.alt;
      const code = img.src.split("/").pop().split("=")[0] + "=";
      return [code, name];
    })
  );
  console.log(mapping);
  await download(JSON.stringify(mapping), "emote_mapping.json");
}

await emote_mapping();

// ===================================

async function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function download(obj, name) {
  const blob = new Blob([obj], { type: "text/plain" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  a.click();
}
