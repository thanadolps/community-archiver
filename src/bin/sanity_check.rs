use color_eyre::eyre::{Result, ensure, eyre};
use indicatif::ParallelProgressIterator;
use itertools::Itertools;
use rayon::prelude::*;
use scraper::{Node, Selector};
use std::{
    collections::HashMap,
    fs::{File, read_dir},
};

fn check(content: &str) -> Vec<&'static str> {
    let mut errs = Vec::new();

    let comment_enabled = !content.contains("support.google.com/youtube/answer/9706180");
    if comment_enabled != content.contains("จัดเรียงความคิดเห็น")
    {
        errs.push("จัดเรียงความคิดเห็น")
    }
    if comment_enabled != content.contains("เพิ่มความคิดเห็น") {
        errs.push("เพิ่มความคิดเห็น")
    }
    if !content.contains("ชอบ") {
        errs.push("ชอบ")
    }
    if !content.contains("ไม่ชอบ") {
        errs.push("ไม่ชอบ")
    }

    let html = scraper::Html::parse_document(content);

    errs.extend(check_visible_texts(
        html.tree.root(),
        &["แสดงการตอบกลับเพิ่มเติม", "อ่านเพิ่มเติม", "การตอบกลับ"],
    ));

    match html
        .select(&Selector::parse("#poll-attachment:not([hidden])").unwrap())
        .at_most_one()
    {
        Ok(Some(poll)) => {
            if !poll
                .text()
                .collect::<String>()
                .as_bytes()
                .windows(3)
                .any(|w| matches!(w, [b'0'..=b'9', b'0'..=b'9', b'%']))
            {
                errs.push("poll-attachment")
            }
        }
        Ok(None) => {}
        Err(_) => errs.push("poll-attachment"),
    }

    errs
}

fn check_visible_texts<'a>(node: ego_tree::NodeRef<Node>, texts: &[&'a str]) -> Vec<&'a str> {
    let mut seen = vec![false; texts.len()];
    let mut ft = vec![node];

    while let Some(n) = ft.pop() {
        if let Some(text) = n.value().as_text()
            && let Some(pos) = texts.iter().position(|&s| text.contains(s))
        {
            seen[pos] = true;
            if seen.iter().all(|&s| s) {
                break;
            }
        }

        if let Some(element) = n.value().as_element()
            && (element.attr("hidden").is_some() || element.attr("style") == Some("display: none;"))
        {
            continue;
        }

        ft.extend(n.children());
    }

    texts
        .iter()
        .enumerate()
        .filter_map(|(i, &s)| seen[i].then_some(s))
        .collect()
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let dirs = read_dir("archive")?.collect::<std::io::Result<Vec<_>>>()?;

    let dirs_len = dirs.len();
    let invalid = dirs
        .into_par_iter()
        .progress()
        .filter(|fs| fs.file_type().is_ok_and(|ft| ft.is_file()))
        .map(|fs| {
            let path = fs.path();
            let content = std::fs::read_to_string(&path)?;
            let errs = check(&content);
            if errs.is_empty() {
                return Ok(None);
            }
            println!("{} is not valid: {:?}", fs.file_name().display(), errs);

            let id = path
                .file_stem()
                .unwrap()
                .to_owned()
                .into_string()
                .map_err(|ostr| eyre!("{:?} should be utf-8", ostr))?;
            Ok::<_, color_eyre::eyre::Error>(Some((id, errs)))
        })
        .filter_map(|r| r.transpose())
        .collect::<Result<HashMap<_, _>, _>>()?;

    println!(
        "{}/{} invalid ({:.2}%)",
        invalid.len(),
        dirs_len,
        100.0 * (invalid.len() as f64 / dirs_len as f64)
    );
    let ids: Vec<String> = serde_json::from_str(&std::fs::read_to_string("data/post_ids.json")?)?;
    serde_json::to_writer_pretty(File::create("scripts/err/invalid.json")?, &invalid)?;
    serde_json::to_writer_pretty(
        File::create("scripts/err/invalid_ids.json")?,
        &invalid
            .keys()
            .sorted_by_key(|iv| ids.iter().position(|x| x == *iv))
            .collect::<Vec<_>>(),
    )?;

    ensure!(invalid.is_empty());
    Ok(())
}
