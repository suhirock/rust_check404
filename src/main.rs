use reqwest;
use scraper::{Html, Selector};
use std::collections::{HashMap, HashSet, VecDeque};
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::time::Instant;
use url::Url;
use regex::Regex;
use rusqlite::{Connection, params};
use chrono::Utc;

async fn crawl(url: &str, visited: &mut HashSet<String>, max_depth: u32, base_url: &Url, pattern_limit: &mut HashMap<String, usize>, unique_patterns: &[Regex], conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    let url_without_hash = url.split('#').next().unwrap_or(url);
    let normalized_url = normalize_url(url_without_hash);
    
    if visited.contains(&normalized_url) {
        return Ok(());
    }
    visited.insert(normalized_url.clone());
    println!("Crawling: {}", url_without_hash);

    let response = reqwest::get(url_without_hash).await?;
    let status = response.status().as_u16();

    // SQLiteにデータを保存
    let domain = base_url.domain().unwrap_or("").to_string();
    let current_time = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO pages (check_url, domain, status, updated_at) VALUES (?1, ?2, ?3, ?4)",
        params![url_without_hash, domain, status, current_time],
    )?;

    if status == 404 {
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
                    absolute_url.set_fragment(None);
                    if absolute_url.domain() == base_url.domain() {
                        let url_str = absolute_url.to_string();
                        let normalized_url_str = normalize_url(&url_str);
                        let depth = normalized_url_str.matches('/').count() - 2;
                        if depth <= max_depth as usize && !visited.contains(&normalized_url_str) {
                            let pattern = get_url_pattern(&normalized_url_str, unique_patterns);
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
        let next_future = Box::pin(crawl(&next_url, visited, max_depth, base_url, pattern_limit, unique_patterns, conn));
        next_future.await?;
    }

    Ok(())
}

// normalize_url, get_url_pattern, load_unique_patterns 関数は変更なし

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    // ヘルプメッセージの表示部分は変更なし

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

    let pattern_file = args.iter()
        .find(|arg| arg.starts_with("-x="))
        .map(|arg| arg.strip_prefix("-x=").unwrap());

    let unique_patterns = if let Some(file_path) = pattern_file {
        load_unique_patterns(file_path)?
    } else {
        vec![Regex::new(r"/\d+").unwrap()]
    };

    let mut visited = HashSet::new();
    let mut pattern_limit = HashMap::new();

    // SQLiteデータベースの初期化
    let conn = Connection::open("crawl_data.db")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS pages (
            id INTEGER PRIMARY KEY,
            check_url TEXT NOT NULL,
            domain TEXT NOT NULL,
            status INTEGER NOT NULL,
            updated_at TEXT NOT NULL
        )",
        [],
    )?;

    let start_time = Instant::now();
    let base_url = Url::parse(start_url).unwrap();
    crawl(start_url, &mut visited, depth, &base_url, &mut pattern_limit, &unique_patterns, &conn).await?;
    let elapsed_time = start_time.elapsed();

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