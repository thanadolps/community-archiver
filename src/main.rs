mod emoji;

use color_eyre::{
    Result,
    eyre::{Context, ContextCompat, eyre},
};
use indicatif::ParallelProgressIterator;
use itertools::Itertools;
use rayon::prelude::*;
use scraper::{Element, ElementRef, Node, Selector};
use std::{
    fs::{self, File},
    io,
    ops::Not,
    time::{Instant, SystemTime},
};

fn main() -> Result<()> {
    color_eyre::install()?;

    let t0 = Instant::now();

    let dirs = fs::read_dir("archive")?.collect::<io::Result<Vec<_>>>()?;

    let posts = dirs
        .into_par_iter()
        .progress()
        .map(|dir| {
            let content = fs::read_to_string(dir.path())?;
            let name = dir
                .path()
                .file_stem()
                .context("no file name")?
                .to_string_lossy()
                .to_string();

            let metadata = dir.metadata().ok();
            let created_at = metadata.as_ref().and_then(|m| m.created().ok());
            let modified_at = metadata.as_ref().and_then(|m| m.modified().ok());
            let processed_at = SystemTime::now();
            let meta = Meta {
                source_created_at: created_at.map(|t| t.into()),
                source_modified_at: modified_at.map(|t| t.into()),
                processed_at: processed_at.into(),
            };

            let post = parse(&content, name.clone())
                .with_context(|| format!("Fail to parse post from {name}"))?;
            Ok::<_, color_eyre::eyre::Error>(PostWithMeta { post, meta })
        })
        .collect::<Result<Vec<PostWithMeta>>>()?;

    println!("Processing done in : {:.2?}", t0.elapsed());

    let t0 = Instant::now();
    serde_json::to_writer_pretty(File::create("data/posts.json")?, &posts)?;
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
    #[serde(with = "time::serde::rfc3339::option")]
    source_created_at: Option<time::OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    source_modified_at: Option<time::OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    processed_at: time::OffsetDateTime,
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
    assert!(n >= comments.len() as u32);

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
                    match emoji::resolve_emoji(src, alt) {
                        Some(res) => return Ok(res),
                        None => return Ok(format!("<img src=\"{}\">", src)),
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
