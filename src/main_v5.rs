use reqwest;
use scraper::{Html, Selector};
use std::collections::{HashMap, HashSet, VecDeque};
use std::env;
use std::time::Instant;
use url::Url;
use regex::Regex;

async fn crawl(url: &str, visited: &mut HashSet<String>, max_depth: u32, base_url: &Url, pattern_limit: &mut HashMap<String, usize>) -> Result<(), Box<dyn std::error::Error>> {
    let url_without_hash = url.split('#').next().unwrap_or(url);
    if visited.contains(url_without_hash) {
        return Ok(());
    }
    visited.insert(url_without_hash.to_string());
    println!("Crawling: {}", url_without_hash);

    let response = reqwest::get(url_without_hash).await?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        println!("404 Error: {}", url_without_hash);
        return Ok(());
    }

    let html = response.text().await?;
    let document = Html::parse_document(&html);
    let selector = Selector::parse("a").unwrap();

    let mut queue: VecDeque<String> = VecDeque::new();

    for element in document.select(&selector) {
        if let Some(href) = element.value().attr("href") {
            if !href.starts_with("tel:") && !href.starts_with("mailto:") {
                if let Ok(mut absolute_url) = Url::parse(url_without_hash)?.join(href) {
                    absolute_url.set_fragment(None);  // ハッシュフラグメントを除去
                    if absolute_url.domain() == base_url.domain() {
                        let url_str = absolute_url.to_string();
                        let depth = url_str.matches('/').count() - 2; // Subtract 2 for "http://"
                        if depth <= max_depth as usize && !visited.contains(&url_str) {
                            let pattern = get_url_pattern(&url_str);
                            let count = pattern_limit.entry(pattern).or_insert(0);
                            if *count < 3 {
                                *count += 1;
                                queue.push_back(url_str);
                            }
                        }
                    }
                }
            }
        }
    }

    while let Some(next_url) = queue.pop_front() {
        let next_future = Box::pin(crawl(&next_url, visited, max_depth, base_url, pattern_limit));
        next_future.await?;
    }

    Ok(())
}

fn get_url_pattern(url: &str) -> String {
    let re = Regex::new(r"/\d+").unwrap();
    re.replace_all(url, "/{number}").to_string()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() == 2 && (args[1] == "-h" || args[1] == "--help") {
        println!("Usage: your_program [OPTIONS] <URL> [-d=<DEPTH>]");
        println!();
        println!("Arguments:");
        println!("  <URL>    The starting URL to crawl");
        println!("  -d=<DEPTH>  The maximum depth to crawl (default: 3)");
        println!();
        println!("Options:");
        println!("  -h, --help  Print help information");
        return Ok(());
    }

    let start_url = if args.len() > 1 {
        &args[1]
    } else {
        "http://localhost/"
    };

    let depth = if let Some(arg) = args.iter().find(|arg| arg.starts_with("-d=")) {
        arg.strip_prefix("-d=").unwrap().parse().unwrap_or(3)
    } else {
        3
    };

    let mut visited = HashSet::new();
    let mut pattern_limit = HashMap::new();
    let has_404 = false;

    let start_time = Instant::now();
    let base_url = Url::parse(start_url).unwrap();
    crawl(start_url, &mut visited, depth, &base_url, &mut pattern_limit).await?;
    let elapsed_time = start_time.elapsed();

    if !has_404 {
        println!("No 404 pages found.");
    }

    println!("Total URLs crawled: {}", visited.len());

    let elapsed_seconds = elapsed_time.as_secs();
    let hours = elapsed_seconds / 3600;
    let minutes = (elapsed_seconds % 3600) / 60;
    let seconds = elapsed_seconds % 60;

    if hours > 0 {
        println!("Total elapsed time: {}時間{}分{}秒", hours, minutes, seconds);
    } else {
        println!("Total elapsed time: {}分{}秒", minutes, seconds);
    }

    Ok(())
}
