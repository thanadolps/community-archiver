// Run this script in the browser console
// run on community page (ie. `/community`)

async function post_ids() {
  await scroll_to_end();
  return [...document.querySelectorAll("#post")].map((post) => {
    const links = [...post.querySelectorAll("a")];
    const i = links.findIndex(
      (link) => link.href.includes("post") || link.href.includes("community")
    );
    if (i === -1) {
      console.error(post);
      throw new Error("Post link not found");
    }

    return links[i].href
      .replace("https://www.youtube.com", "")
      .replace("/post/", "")
      .replace(/\/channel\/[^/]+\/community\?lb=/, "");
  });
}

async function download_post_ids() {
  const postIds = await post_ids();
  console.log("Finished scrolling, downloading post ids");
  await download(JSON.stringify(postIds), "post_ids.json");
  console.log(`${postIds.length} post ids downloaded`);
}

await download_post_ids();

// ===================================

async function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function scrollHeight() {
  return document.scrollingElement.scrollHeight;
}

async function scroll_to_end() {
  let lastHeight = scrollHeight();

  scroll: while (true) {
    window.scrollTo(0, scrollHeight());

    for (let i = 0; i < 6; i++) {
      await sleep(1000);
      const newHeight = scrollHeight();
      if (newHeight !== lastHeight) {
        lastHeight = newHeight;
        console.log(`Height changed to ${newHeight}`);
        continue scroll;
      }
    }

    console.log("Reached the end of the page");
    break;
  }
}

async function download(obj, name) {
  const blob = new Blob([obj], { type: "text/plain" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  a.click();
}
