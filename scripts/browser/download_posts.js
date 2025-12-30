// Run this script in the browser console
// run on community page (ie. `/community` or `/posts`)

/**
 * @param {string[]} _ids
 */
async function posts(_ids) {
  const ids = [..._ids];

  for (let i = 0; i < ids.length; i++) {
    const id = ids[i];
    console.log(`Processing: ${id} (${i + 1}/${ids.length})`);

    let a;
    while (true) {
      a = [...document.querySelectorAll("a")].find((a) => a.href.includes(id));
      if (a) {
        break;
      }
      if ((await scroll_step()) === null) {
        throw new Error(`Post ${id} anchor not found`);
      }
    }
    a.scrollIntoView();
    await sleep(500);
    a.click();
    await sleep(2000);

    if (!(await download_post())) {
      console.log(`Failed ${id}, will retry later`);
      ids.push(id);
    }
    await sleep(2000);

    history.back();
    await sleep(2000);
  }

  console.log(`Finished processing ${ids.length} posts`);
}

async function download_post() {
  const result = await extract_post();
  if (!result) {
    return false;
  }

  const { id, html } = result;
  console.log(`Download post ${id}`);
  download(html, `${id}.html`);

  return true;
}

/**
 * Extract post content without downloading.
 * Returns {id, html} object or false on failure.
 */
async function extract_post() {
  const id =
    new URLSearchParams(window.location.search).get("lb") ??
    window.location.pathname.split("/")[2];
  console.log(`Prepare to extract post ${id}`);

  // get content view
  const content = document.querySelector("#primary>*>#contents");

  // expand the post
  const ex = content.querySelector("#post #more:not([hidden])");
  if (ex) {
    ex.scrollIntoView({ behavior: "smooth" });
    await sleep(500);
    ex.click();
    await sleep(500);
  }

  // Load poll results
  const pollOptions = content.querySelectorAll(
    "#poll-attachment:not([hidden]) a[role='option']",
  );
  if (pollOptions.length > 0) {
    const i = Math.floor(Math.random() * pollOptions.length);
    pollOptions[i].scrollIntoView({ behavior: "smooth" });
    await sleep(500);
    console.log(`Found poll options, clicking ${pollOptions[i].textContent}`);
    pollOptions[i].click();
    await sleep(500);
  }

  // load all the comments
  await scroll_to_end();
  await sleep(1000);

  // expand all the comments
  while (true) {
    let pass = true;

    const mrs = [
      ...content.querySelectorAll("#more-replies, #more-replies-sub-thread"),
    ].filter((e) => e.checkVisibility());
    console.log(`Found ${mrs.length} more-replies`);
    pass &= mrs.length == 0;

    for (let i = 0; i < mrs.length; i++) {
      const mr = mrs[i];
      console.log(`[mr] ${i + 1}/${mrs.length}`);
      mr.scrollIntoView({ behavior: "smooth" });
      await sleep(500);
      mr.click();
      await sleep(1000);
      await waitSpinner();
    }
    await sleep(1000);

    // expand more replies
    while (true) {
      // WARN: THAILAND ONLY
      const bs = [
        ...content.querySelectorAll(
          "button[aria-label='แสดงการตอบกลับเพิ่มเติม']",
        ),
      ].filter((e) => e.checkVisibility());
      console.log(`Activating ${bs.length} more replies`);
      pass &= bs.length == 0;
      if (bs.length === 0) {
        break;
      }

      for (b of bs) {
        b.scrollIntoView({ behavior: "smooth" });
        await sleep(500);
        b.click();
        await sleep(1000);
        await waitSpinner();
      }
    }

    // expand "read more"
    const rms = content.querySelectorAll("#comment #more:not([hidden])");
    console.log(`Found ${rms.length} "read more" in comments`);
    pass &= rms.length == 0;

    for (let i = 0; i < rms.length; i++) {
      const rm = rms[i];
      console.log(`[rm] ${i + 1}/${rms.length}`);
      rm.scrollIntoView({ behavior: "smooth" });
      await sleep(500);
      rm.click();
      await sleep(250);
      await waitSpinner();
    }

    // load all images
    // this may fail, so timeout is required
    if (!(await loadImgs(content, 10000))) {
      return false;
    }

    if (pass) {
      break;
    }
  }

  // extract the html
  console.log(`Extracted post ${id}`);
  const html = content.outerHTML;

  return { id, html };
}

/**
 * @param {HTMLElement} content
 * @param {number} timeout
 * @returns
 */
async function loadImgs(content, timeout) {
  let should_kill = false;
  const kill = () => (should_kill = true);

  let timer = setTimeout(kill, timeout);
  let bestLength = 999999;

  while (true) {
    const unload = [
      ...content.querySelectorAll("ytd-comment-thread-renderer img:not([src])"),
    ].filter((img) => img.checkVisibility());
    if (unload.length === 0) {
      break;
    }
    console.log(`Loading ${unload.length} images...`);

    if (unload.length < bestLength) {
      bestLength = unload.length;
      clearTimeout(timer);
      timer = setTimeout(kill, timeout);
    }

    const img = unload[Math.floor(Math.random() * unload.length)];
    const align = ["start", "center", "end", "nearest"][
      Math.floor(Math.random() * 4)
    ];

    img.scrollIntoView({ behavior: "smooth", block: align });

    if (should_kill) {
      console.log("Timeout");
      console.log("Unloaded:", unload);
      clearTimeout(timer);
      return false;
    }

    await sleep(500);
  }

  return true;
}

// ===================================

async function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function scrollHeight() {
  return document.scrollingElement.scrollHeight;
}

async function scroll_to_end() {
  while (true) {
    const height = await scroll_step();
    if (height === null) {
      console.log("Reached the end of the page");
      break;
    }

    console.log(`Height changed to ${height}`);
    await waitSpinner();
  }
}

async function scroll_step() {
  const lastHeight = scrollHeight();
  window.scrollTo({ left: 0, top: scrollHeight(), behavior: "smooth" });
  for (let i = 0; i < 6; i++) {
    await sleep(1000);
    const newHeight = scrollHeight();
    if (newHeight !== lastHeight) {
      return newHeight;
    }
  }
  return null;
}

async function waitSpinner(delay = 1000) {
  while (activeSpinners().length > 0) {
    console.log("waiting for spinner loading");
    await sleep(delay);
  }
}

function activeSpinners() {
  return [
    ...document.querySelectorAll("#spinner:not([aria-hidden=true])"),
  ].filter((x) => x.checkVisibility());
}

function download(obj, name) {
  const blob = new Blob([obj], { type: "text/plain" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  a.click();
}
