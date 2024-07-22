#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn get_depth_from_args(args: Vec<String>) -> u32 {
        if let Some(arg) = args.iter().find(|arg| arg.starts_with("-d=")) {
            arg.strip_prefix("-d=").unwrap().parse().unwrap_or(3)
        } else {
            3
        }
    }

    fn get_pattern_file_from_args(args: Vec<String>) -> Option<String> {
        args.iter()
            .find(|arg| arg.starts_with("-x="))
            .map(|arg| arg.strip_prefix("-x=").unwrap().to_string())
    }

    #[test]
    fn test_depth_parsing() {
        // デフォルト値のテスト
        assert_eq!(get_depth_from_args(vec!["program".to_string()]), 5);

        // 正常な値のテスト
        assert_eq!(get_depth_from_args(vec!["program".to_string(), "-d=5".to_string()]), 5);
        assert_eq!(get_depth_from_args(vec!["program".to_string(), "http://example.com".to_string(), "-d=2".to_string()]), 2);

        // 不正な値のテスト（デフォルト値に戻るべき）
        assert_eq!(get_depth_from_args(vec!["program".to_string(), "-d=invalid".to_string()]), 3);

        // 複数の-d引数がある場合、最初の有効な値を使用
        assert_eq!(get_depth_from_args(vec!["program".to_string(), "-d=4".to_string(), "-d=6".to_string()]), 4);

        // 負の値のテスト（実装によっては別の処理が必要かもしれません）
        assert_eq!(get_depth_from_args(vec!["program".to_string(), "-d=-1".to_string()]), 3);
    }

    #[test]
    fn test_pattern_file_parsing() {
        // パターンファイルが指定されていない場合
        assert_eq!(get_pattern_file_from_args(vec!["program".to_string()]), None);

        // パターンファイルが指定されている場合
        assert_eq!(
            get_pattern_file_from_args(vec!["program".to_string(), "-x=patterns.txt".to_string()]),
            Some("patterns.txt".to_string())
        );

        // 複数の-x引数がある場合、最初の値を使用
        assert_eq!(
            get_pattern_file_from_args(vec!["program".to_string(), "-x=patterns1.txt".to_string(), "-x=patterns2.txt".to_string()]),
            Some("patterns1.txt".to_string())
        );
    }

    #[test]
    fn test_url_depth_calculation() {
        let base_url = Url::parse("http://example.com").unwrap();
        
        // ルートURLのテスト
        assert_eq!(calculate_url_depth("http://example.com", &base_url), 0);
        
        // 1階層のURLのテスト
        assert_eq!(calculate_url_depth("http://example.com/page", &base_url), 1);
        
        // 2階層のURLのテスト
        assert_eq!(calculate_url_depth("http://example.com/category/page", &base_url), 2);
        
        // 3階層のURLのテスト
        assert_eq!(calculate_url_depth("http://example.com/category/subcategory/page", &base_url), 3);
        
        // クエリパラメータを含むURLのテスト
        assert_eq!(calculate_url_depth("http://example.com/page?param=value", &base_url), 1);
        
        // フラグメントを含むURLのテスト
        assert_eq!(calculate_url_depth("http://example.com/page#section", &base_url), 1);
        
        // 末尾のスラッシュを含むURLのテスト
        assert_eq!(calculate_url_depth("http://example.com/category/", &base_url), 1);
    }

    #[test]
    fn test_load_unique_patterns() -> Result<(), Box<dyn std::error::Error>> {
        // テスト用の一時ファイルを作成
        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file, r"/news/\d+")?;
        writeln!(temp_file, r"/article/\d+")?;
        writeln!(temp_file, r"/blog/\d{4}/\d{2}/\d{2}/.*")?;
        writeln!(temp_file, r"/news/\d+")?;  // 重複パターン

        let patterns = load_unique_patterns(temp_file.path().to_str().unwrap())?;

        assert_eq!(patterns.len(), 3);  // 重複が除去されているか確認
        assert!(patterns.iter().any(|p| p.as_str() == r"/news/\d+"));
        assert!(patterns.iter().any(|p| p.as_str() == r"/article/\d+"));
        assert!(patterns.iter().any(|p| p.as_str() == r"/blog/\d{4}/\d{2}/\d{2}/.*"));

        Ok(())
    }

    #[test]
    fn test_normalize_url() {
        assert_eq!(normalize_url("http://example.com"), "http://example.com");
        assert_eq!(normalize_url("http://example.com/"), "http://example.com");
        assert_eq!(normalize_url("http://example.com/page/"), "http://example.com/page");
        assert_eq!(normalize_url("http://example.com/PAGE/"), "http://example.com/page");
        assert_eq!(normalize_url("HTTP://EXAMPLE.COM/PAGE"), "http://example.com/page");
    }

    fn calculate_url_depth(url: &str, base_url: &Url) -> usize {
        let url = Url::parse(url).unwrap();
        if url.domain() == base_url.domain() {
            url.path().trim_end_matches('/').split('/').filter(|s| !s.is_empty()).count()
        } else {
            0
        }
    }
}
