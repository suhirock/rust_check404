use reqwest;
use scraper::{Html, Selector};
use std::collections::{HashSet, VecDeque};
use std::env;
use std::time::{Instant};
use url::Url;

async fn crawl(url: &str, visited: &mut HashSet<String>, depth: u32, current_depth: u32, base_url: &Url) -> Result<(), Box<dyn std::error::Error>> {
    if visited.contains(url) {
        return Ok(());
    }
    visited.insert(url.to_string());
    println!("Crawling: {}", url);

    let response = reqwest::get(url).await?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        println!("404 Error: {}", url);
        return Ok(());
    }

    let html = response.text().await?;
    let document = Html::parse_document(&html);
    let selector = Selector::parse("a").unwrap();

    let mut queue: VecDeque<(String, u32)> = VecDeque::new();

    for element in document.select(&selector) {
        if let Some(href) = element.value().attr("href") {
            if !href.starts_with("tel:") && !href.starts_with("mailto:") {
                if let Ok(absolute_url) = Url::parse(url)?.join(href) {
                    if absolute_url.domain() == base_url.domain() {
                        let url_str = absolute_url.to_string();
                        if !visited.contains(&url_str) {
                            queue.push_back((url_str, current_depth + 1));
                        }
                    }
                }
            }
        }
    }

    while let Some((next_url, next_depth)) = queue.pop_front() {
        if next_depth <= depth {
            let next_future = Box::pin(crawl(&next_url, visited, depth, next_depth, base_url));
            next_future.await?;
        }
    }

    Ok(())
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
    let has_404 = false;

    let start_time = Instant::now();
    let base_url = Url::parse(start_url).unwrap();
    crawl(start_url, &mut visited, depth, 1, &base_url).await?;
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
