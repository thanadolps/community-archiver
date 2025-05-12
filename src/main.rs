mod emote;

use clap::Parser;
use color_eyre::{
    Result,
    eyre::{Context, ContextCompat, ensure, eyre},
};
use emote::EmoteResolver;
use indicatif::{HumanBytes, ParallelProgressIterator, ProgressStyle};
use itertools::Itertools;
use rayon::prelude::*;
use scraper::{Element, ElementRef, Node, Selector};
use std::{
    fs::{self, File},
    io::{self, BufWriter, Write},
    ops::Not,
    path::PathBuf,
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{Instant, SystemTime},
};

#[derive(Parser)]
struct Args {
    #[arg(long, value_name = "DIR", default_value = "archive")]
    archive_dir: PathBuf,
    #[arg(long, value_name = "DIR", default_value = "data")]
    /// Directory containing emote data (emote_mapping.json and emotes_default_mapping.json).
    /// Use for fallback emote resolution.
    emote_data_dir: PathBuf,
    #[arg(long, value_name = "FILE", default_value = "data/post_ids.json")]
    /// Path to post ids.json file, use for ordering the output.
    post_ids_file: PathBuf,
    #[arg(long, value_name = "FILE", default_value = "data/posts.json")]
    output_file: PathBuf,
}

static EMOTE_RESOLVER: OnceLock<EmoteResolver> = OnceLock::new();

fn main() -> Result<()> {
    color_eyre::install()?;

    let Args {
        archive_dir,
        emote_data_dir,
        post_ids_file,
        output_file,
    } = Args::parse();
    println!("Processing posts in `{}`", archive_dir.display());

    let t0 = Instant::now();
    let dirs = fs::read_dir(archive_dir)?.collect::<io::Result<Vec<_>>>()?;
    if dirs.is_empty() {
        println!("No posts to process");
        return Ok(());
    }

    EMOTE_RESOLVER
        .set(EmoteResolver::with_emote_dir(&emote_data_dir))
        .unwrap();

    let total_bytes = AtomicU64::new(0);
    let mut posts = dirs
        .into_par_iter()
        .progress()
        .with_style(ProgressStyle::with_template(
            "{wide_bar} {pos}/{len} {per_sec} {eta}",
        )?)
        .map(|dir| {
            let t0 = Instant::now();
            let content = fs::read_to_string(dir.path())?;
            let name = dir
                .path()
                .file_stem()
                .context("no file name")?
                .to_string_lossy()
                .to_string();
            total_bytes.fetch_add(content.len() as u64, Ordering::Relaxed);

            let metadata = dir.metadata().ok();
            let created_at = metadata.as_ref().and_then(|m| m.created().ok());
            let modified_at = metadata.as_ref().and_then(|m| m.modified().ok());
            let processed_at = SystemTime::now();

            let post = parse(&content, name.clone())
                .with_context(|| format!("Fail to parse post from {name}"))?;
            let elapsed = t0.elapsed();

            let meta = Meta {
                source_created_at: created_at.map(|t| t.try_into().unwrap()),
                source_modified_at: modified_at.map(|t| t.try_into().unwrap()),
                processed_at: processed_at.try_into().unwrap(),
                process_time: elapsed.try_into().unwrap(),
            };
            Ok::<_, color_eyre::eyre::Error>(PostWithMeta { post, meta })
        })
        .collect::<Result<Vec<PostWithMeta>>>()?;

    let post_ids = 'a: {
        let Ok(post_ids) = fs::read_to_string(&post_ids_file) else {
            eprintln!("Failed to read post ids file: {}", post_ids_file.display());
            break 'a Vec::new();
        };
        let Ok(post_ids) = serde_json::from_str::<Vec<String>>(post_ids.as_str()) else {
            eprintln!("Failed to parse post ids file: {}", post_ids_file.display());
            break 'a Vec::new();
        };
        post_ids
    };
    posts.sort_unstable_by_key(|post| post_ids.iter().position(|id| id == &post.post.id));

    let elapsed = t0.elapsed();
    let total_bytes = total_bytes.into_inner();
    println!(
        "Processing {} posts done in : {:.2?} ({:.2} post/s), Total bytes: {} ({}/s)",
        posts.len(),
        elapsed,
        posts.len() as f64 / elapsed.as_secs_f64(),
        HumanBytes(total_bytes),
        HumanBytes(total_bytes / elapsed.as_secs())
    );

    // Write JSON
    let t0 = Instant::now();
    let mut posts_writer = BufWriter::new(File::create(output_file)?);
    serde_json::to_writer(&mut posts_writer, &posts)?;
    posts_writer.flush()?;
    println!("Writing JSON done in : {:.2?}", t0.elapsed());

    Ok(())
}

#[derive(Debug, serde::Serialize)]
struct PostWithMeta {
    meta: Meta,
    #[serde(flatten)]
    post: Post,
}

#[derive(Debug, serde::Serialize)]
struct Meta {
    source_created_at: Option<jiff::Timestamp>,
    source_modified_at: Option<jiff::Timestamp>,
    processed_at: jiff::Timestamp,
    process_time: jiff::SignedDuration,
}

#[derive(Debug, serde::Serialize)]
struct Post {
    id: String,
    #[serde(flatten)]
    main: Main,
    comments: Option<Vec<MainComment>>,
    total_comment: Option<u32>,
}

#[derive(Debug, serde::Serialize)]
struct Main {
    author: String,
    publish_time: String,
    sponsor_only: Option<String>,

    content: String,
    content_attachment: Option<ContentAttachment>,
    poll_attachment: Option<PollAttachment>,
    like: u32,
}

#[derive(Debug, serde::Serialize)]
struct ContentAttachment {
    images: Vec<String>,
    videos: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    unknown: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
struct PollAttachment {
    total_votes: u32,
    items: Vec<PollItem>,
}

#[derive(Debug, serde::Serialize)]
struct PollItem {
    text: String,
    percentage: String,
}

#[derive(Debug, serde::Serialize)]
struct MainComment {
    #[serde(flatten)]
    comment: Comment,
    replies: Vec<Comment>,
}

#[derive(Debug, serde::Serialize)]
struct Comment {
    author: String,
    content: String,
    publish_time: String,
    sponsor_duration: Option<String>,
    sponsor_badge: Option<String>,
    like: u32,
}

fn parse(content: &str, id: String) -> Result<Post> {
    let html = scraper::Html::parse_document(content);

    let selector = Selector::parse("body>#contents>*").unwrap();
    let mut content_items = html.select(&selector);
    let post = content_items.next().unwrap();
    let comment = content_items.next();
    assert!(content_items.next().is_none());
    match comment {
        Some(comment) => assert_eq!(comment.value().name(), "ytd-comments"),
        None => assert!(
            // ensure missing comments are due to comments being turned off
            post.html()
                .contains("support.google.com/youtube/answer/9706180")
        ),
    }

    let main = post
        .select(&Selector::parse("#post>*>#body #main").unwrap())
        .exactly_one()
        .unwrap();
    let main = parse_main(main)?;

    let (total_comment, comments) = if let Some(comment) = comment {
        let (total_comment, comments) = parse_comments(comment)?;
        (Some(total_comment), Some(comments))
    } else {
        (None, None)
    };

    Ok(Post {
        id,
        main,
        comments,
        total_comment,
    })
}

fn parse_main(main: scraper::ElementRef<'_>) -> Result<Main> {
    let author_text = main
        .select(&Selector::parse("#author-text").unwrap())
        .exactly_one()
        .unwrap()
        .text()
        .map(|s| s.trim())
        .collect::<String>();
    let publish_time = main
        .select(&Selector::parse("#published-time-text").unwrap())
        .exactly_one()
        .unwrap()
        .text()
        .collect::<String>();
    let sponsor_only = main
        .select(&Selector::parse("#sponsors-only-badge").unwrap())
        .at_most_one()
        .unwrap()
        .map(|s| {
            s.text()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .join("\n")
        })
        .filter(|s| !s.is_empty());
    let content = main
        .select(&Selector::parse("#content").unwrap())
        .next()
        .unwrap()
        .text()
        .map(|s| s.trim())
        .collect::<String>();
    let content_attachment = main
        .select(&Selector::parse("#content-attachment:not([hidden])").unwrap())
        .next()
        .map(|c| {
            let images = c
                .select(&Selector::parse("img[src]").unwrap())
                .map(|img| img.attr("src").unwrap().to_owned())
                .collect::<Vec<_>>();
            let (videos, unknown) = c
                .select(&Selector::parse("a[href]").unwrap())
                .map(|link| link.attr("href").unwrap().to_owned())
                .filter(|link| {
                    // filter out self-link to current channel
                    link.contains("/@").not()
                })
                .dedup()
                .partition(|link| link.contains("/watch?v="));

            ContentAttachment {
                images,
                videos,
                unknown,
            }
        });
    let poll_attachment = main
        .select(&Selector::parse("#poll-attachment:not([hidden])").unwrap())
        .at_most_one()
        .unwrap()
        .map(|poll| -> Result<PollAttachment> {
            let total_votes = poll
                .select(&Selector::parse("#vote-info").unwrap())
                .exactly_one()
                .unwrap()
                .text()
                .collect::<String>();
            let total_votes =
                parse_vote(total_votes.trim_end_matches("คะแนน")).wrap_err_with(|| {
                    format!("total votes should be a parseable number: {total_votes}")
                })?;

            let items = poll
                .select(&Selector::parse("a[role='option'] .choice-info").unwrap())
                .map(|poll| {
                    let text = poll
                        .select(&Selector::parse(".choice-text").unwrap())
                        .exactly_one()
                        .unwrap()
                        .text()
                        .collect::<String>();
                    let percentage = poll
                        .select(&Selector::parse(".vote-percentage").unwrap())
                        .exactly_one()
                        .unwrap()
                        .text()
                        .collect::<String>();
                    PollItem { text, percentage }
                })
                .collect();

            Ok(PollAttachment { total_votes, items })
        })
        .transpose()?;

    let like = main
        .select(&Selector::parse("#vote-count-middle").unwrap())
        .next()
        .unwrap()
        .text()
        .collect::<String>();
    let like = parse_vote(&like)?;

    Ok(Main {
        author: author_text,
        publish_time,
        sponsor_only,
        content,
        content_attachment,
        poll_attachment,
        like,
    })
}

fn parse_comments(comment: scraper::ElementRef<'_>) -> Result<(u32, Vec<MainComment>)> {
    // TODO: used to validate
    let n: u32 = comment
        .select(&Selector::parse("#count").unwrap())
        .exactly_one()
        .map_err(|err| eyre!("{}", err))
        .wrap_err("comments should has exactly one count")?
        .text()
        .find_map(|t| parse_numerical_int(t).ok())
        .wrap_err("comments' count should be a number")?;

    let s = Selector::parse("#contents>ytd-comment-thread-renderer").unwrap();
    let threads = comment.select(&s);
    let comments = threads
        .map(parse_comment_thread)
        .collect::<Result<Vec<_>>>()?;

    // cannot compare exactly because youtube may hide some comments
    // ensure!(
    //     n >= comments.len() as u32,
    //     "provided comment count ({}) should be at least the number of visible comments ({})",
    //     n,
    //     comments.len()
    // );

    Ok((n, comments))
}

fn parse_comment_thread(thread: scraper::ElementRef<'_>) -> Result<MainComment> {
    let comment = thread
        .select(&Selector::parse("#comment").unwrap())
        .exactly_one()
        .unwrap();
    let comment = parse_comment(comment)?;

    let replies = thread
        .select(&Selector::parse("#replies:not([hidden]) #contents>*").unwrap())
        .map(parse_comment)
        .collect::<Result<_>>()?;

    Ok(MainComment { comment, replies })
}

fn parse_comment(comment: scraper::ElementRef<'_>) -> Result<Comment> {
    let author = comment
        .select(&Selector::parse("#author-text").unwrap())
        .exactly_one()
        .map_err(|err| {
            let msg = err.to_string();
            let authors = err.map(|x| x.text().collect::<String>()).collect_vec();
            eyre!("{} {:?}", msg, authors)
        })
        .wrap_err("comment should has exactly one author")?
        .text()
        .map(|s| s.trim())
        .collect::<String>();

    let publish_time = comment
        .select(&Selector::parse("#published-time-text").unwrap())
        .exactly_one()
        .map_err(|err| eyre!("{}", err))
        .wrap_err("comment should has exactly one publish time")?
        .text()
        .map(|s| s.trim())
        .collect::<String>();

    let sponsor = comment
        .select(
            &Selector::parse("#sponsor-comment-badge>ytd-sponsor-comment-badge-renderer").unwrap(),
        )
        .at_most_one()
        .map_err(|err| eyre!("{}", err))
        .wrap_err("comment should has at most one sponsor duration")?;
    let sponsor_duration = sponsor
        .map(|ele| -> Result<String> {
            Ok(ele
                .attr("aria-label")
                .wrap_err("comment's sponsor's aria-label should exists for sponsor duration")?
                .to_owned())
        })
        .transpose()?;
    let sponsor_badge = sponsor
        .map(|ele| -> Result<Option<String>> {
            Ok(ele
                .select(&Selector::parse("img[src]").unwrap())
                .at_most_one()
                .map_err(|err| eyre!("{}", err))
                .wrap_err("comment's sponsor's badge should not be more than one")?
                .map(|ele| ele.attr("src").unwrap().to_owned()))
        })
        .transpose()?
        .flatten();

    let like = comment
        .select(&Selector::parse("#vote-count-middle").unwrap())
        .next()
        .wrap_err("comment should has at least one like")?
        .text()
        .collect::<String>();
    let like = parse_vote(&like)?;

    let content = comment
        .select(&Selector::parse("#content-text>*").unwrap())
        .exactly_one()
        .map_err(|err| eyre!("{}", err))
        .wrap_err("comment should has exactly one content")?;
    let content = content
        .children()
        .map(stringify_content_item)
        .collect::<Result<String>>()?;

    Ok(Comment {
        author,
        publish_time,
        sponsor_duration,
        sponsor_badge,
        like,
        content,
    })
}

fn stringify_content_item(item: ego_tree::NodeRef<Node>) -> Result<String> {
    // Span
    if let Some(e) = ElementRef::wrap(item) {
        if e.value().name() != "span" {
            panic!("unexpected element: {:?}", e.html());
        }

        let text = e.text().collect::<String>();
        let text = text.trim();

        // Likely a styled text
        if !text.is_empty() {
            if e.attr("style")
                .is_some_and(|style| style.contains("font-weight: 500"))
            {
                return Ok(format!("<b>{}</b>", text));
            } else {
                return Ok(text.to_string());
            }
        }

        if let Some(child) = e.first_element_child() {
            match child.value().name() {
                // Likly a emoji
                "img" => {
                    let src = child.attr("src").unwrap();
                    let alt = child.attr("alt");
                    match EMOTE_RESOLVER.get().unwrap().resolve_emoji(src, alt) {
                        Some(res) => return Ok(res),
                        None => {
                            eprintln!("failed to resolve emoji: {}", src);
                            return Ok(format!("<img src=\"{}\">", src));
                        }
                    }
                }
                // Some kind of link
                "a" => {
                    let href = child.attr("href").unwrap();
                    return Ok(format!(
                        "<a href=\"{}\">{}</a>",
                        href,
                        child.text().collect::<String>()
                    ));
                }
                _ => {
                    panic!("unhandled span's child element: {}", child.html());
                }
            }
        }

        panic!("unhandled span: {:?}", e.html());
    };

    // Plain text
    if let Some(t) = item.value().as_text() {
        return Ok(t.trim().to_owned());
    }

    panic!("unexpected node: {:?}", item);
}

fn parse_vote(vote: &str) -> Result<u32> {
    let vote = vote.trim();
    if vote.is_empty() {
        return Ok(0);
    }
    parse_numerical_int(vote)
}

fn parse_numerical_int(s: &str) -> Result<u32> {
    let s = s.trim().replace([',', ' '], "");
    if let Some(s) = s.strip_suffix("พัน") {
        return Ok((s.parse::<f64>()? * 1_000.0) as u32);
    }
    if let Some(s) = s.strip_suffix("หมื่น") {
        return Ok((s.parse::<f64>()? * 10_000.0) as u32);
    }
    if let Some(s) = s.strip_suffix("แสน") {
        return Ok((s.parse::<f64>()? * 100_000.0) as u32);
    }
    Ok(s.parse::<u32>()?)
}
