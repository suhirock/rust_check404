use reqwest;
use scraper::{Html, Selector};
use std::collections::{HashSet, VecDeque};
use std::env;
use std::time::Instant;
use url::Url;

async fn crawl(url: &str, visited: &mut HashSet<String>, depth: u32, current_depth: u32) -> Result<(), Box<dyn std::error::Error>> {
    if visited.contains(url) {
        return Ok(());
    }
    visited.insert(url.to_string());

    let start_time = Instant::now();
    let response = reqwest::get(url).await?;
    let elapsed_time = start_time.elapsed().as_millis();
    println!("Fetched {} in {} ms", url, elapsed_time);

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
                    queue.push_back((absolute_url.to_string(), current_depth + 1));
                }
            }
        }
    }

    while let Some((next_url, next_depth)) = queue.pop_front() {
        if next_depth <= depth {
            let next_future = Box::pin(crawl(&next_url, visited, depth, next_depth));
            next_future.await?;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let start_url = if args.len() > 1 {
        &args[1]
    } else {
        "http://localhost/"
    };
    let depth = if args.len() > 2 {
        args[2].parse().unwrap_or(3)
    } else {
        3
    };
    let mut visited = HashSet::new();
    crawl(start_url, &mut visited, depth, 1).await?;
    Ok(())
}
