# Community Archiver

This repo's primary purpose is to archive community (including member-only) post from Youtube channel ["Yozora Mel Ch. 夜空メルチャンネル"](https://www.youtube.com/channel/UCD8HOxPs4Xvsm8H0ZxXGiBw).

## Motivation

After the sudden Yozora Mel's termination announcement on 2024-01-16, there is a need to archive as much content as possible to preserve the memory of the channel and its members.
While there are many community and tools to archive videos and shorts such as yt-dlp, there seem to be no good tools to archive community posts, this project is custom-made to do that.

Note that while the code is not that specific to Yozora Mel's channel, it is designed with that channel in mind so some of the code might not be general enough to be used for other channels.

## How to use

Unfortunately, the process is not as automated as I wish due to Google's blocking automation tools from loggin in, which is required to access the member-only community posts. The current process is as follows:

### Collect post ids

1. Login to your Google account in your browser
2. Navigate to the channel's community tab
3. Open the developer console, then copy and paste to run the script `scripts/browser/download_post_ids.js`
4. File `post_ids.json` which contain all the post ids should get downloaded, put it in `data` folder

### Download posts html

5. Navigate back to channel's community tab, then copy and paste to run the script `scripts/browser/download_posts.js`,
   then run functiion `posts` with post ids (from `data/post_ids.json`) as argument, this will slowly download all the posts.
6. Put the downloaded posts into `archive` folder
7. Run the sanity check scripts

   - `scripts/ids_check.py` to check if all post ids has been downloaded
   - `scripts/sanity_check.py` and `scripts/sanity_check_2.py` to do basic validation on the downloaded posts
     (If there are any errors, you should identify the broken/missing posts and re-download asap)

### Process posts

8. Run main program (`cargo run --release`), this process the downloaded posts (in `archive` folder) and output the processed posts as `data/posts.json`
9. Run `scripts/download_imgs.py` to download all images in the posts into `archive/imgs` folder

### Extra: Download emojis

10. Navigate to any post (ie. `/community?lb=XXXXX`, not channel's community tab)
11. Open the developer console, then copy and paste to run the script `scripts/browser/download_emojis.js`
12. Put the downloaded `emote_mapping.json` into `data` folder
13. Run the sanity check scripts `scripts/sanity_check_3.py` to check if all emojis has been mapped
14. Run `scripts/download_emote.py` to download all channel's emojis, this will output all emoji images into `emote` folder based on the mapping.
