use std::collections::HashMap;
use std::fs;

use crate::css::CssTokenizer;
use crate::error::{error_page_html, BrowserError};
use crate::html::HtmlParser;
use crate::layout::LayoutEngine;
use crate::network::NetworkStack;
use crate::render::RenderEngine;
use crate::resources::ResourceManager;
use crate::style::StyleComputer;

const DEFAULT_VIEWPORT_WIDTH: f32 = 800.0;
const DEFAULT_VIEWPORT_HEIGHT: f32 = 600.0;


#[derive(Debug, Clone, PartialEq)]
pub enum LoadingState {
    Idle,
    Loading(String),
    Complete,
    Error(String),
}

pub struct ResourceCache {
    entries: HashMap<String, String>,
}

impl ResourceCache {
    pub fn new() -> Self {
        ResourceCache {
            entries: HashMap::new(),
        }
    }

    pub fn get_cached(&self, url: &str) -> Option<&str> {
        self.entries.get(url).map(String::as_str)
    }

    pub fn cache(&mut self, url: &str, content: String) {
        self.entries.insert(url.to_string(), content);
    }

    pub fn invalidate(&mut self, url: &str) {
        self.entries.remove(url);
    }

    pub fn clear_cache(&mut self) {
        self.entries.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for ResourceCache {
    fn default() -> Self {
        Self::new()
    }
}


pub struct Browser {
    pub history: Vec<String>,
    pub history_index: usize,
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub resource_manager: ResourceManager,
    pub resource_cache: ResourceCache,
    pub loading_state: LoadingState,
}

impl Browser {
    pub fn new() -> Self {
        Browser {
            history: Vec::new(),
            history_index: 0,
            viewport_width: DEFAULT_VIEWPORT_WIDTH,
            viewport_height: DEFAULT_VIEWPORT_HEIGHT,
            resource_manager: ResourceManager::new(),
            resource_cache: ResourceCache::new(),
            loading_state: LoadingState::Idle,
        }
    }

    pub fn with_viewport(width: f32, height: f32) -> Self {
        Browser {
            history: Vec::new(),
            history_index: 0,
            viewport_width: width,
            viewport_height: height,
            resource_manager: ResourceManager::new(),
            resource_cache: ResourceCache::new(),
            loading_state: LoadingState::Idle,
        }
    }

    pub fn load(&mut self, url: &str) -> Result<Vec<u8>, String> {
        self.load_typed(url).map_err(|e| e.to_string())
    }

    pub fn load_typed(&mut self, url: &str) -> Result<Vec<u8>, BrowserError> {
        self.loading_state = LoadingState::Loading(url.to_string());

        let result = self.run_pipeline(url);

        match &result {
            Ok(_) => self.loading_state = LoadingState::Complete,
            Err(e) => self.loading_state = LoadingState::Error(e.to_string()),
        }

        result
    }

    fn run_pipeline(&mut self, url: &str) -> Result<Vec<u8>, BrowserError> {
        let html = self.fetch_content_cached(url)?;

        let dom = match HtmlParser::new(html.clone()).parse() {
            Ok(node) => node,
            Err(e) => {
                let error_html = error_page_html(&BrowserError::HtmlParseError(e.to_string()));
                HtmlParser::new(error_html)
                    .parse()
                    .map_err(|e2| BrowserError::RenderError(
                        format!("Failed to render error page: {}", e2)
                    ))?
            }
        };

        let css_source = extract_style_content(&html);
        let stylesheet = CssTokenizer::new(css_source)
            .parse()
            .unwrap_or_else(|_| crate::css::Stylesheet { rules: vec![] });

        let computer = StyleComputer::new(stylesheet);
        let styled = computer.compute_styles(&dom);

        let layout_engine = LayoutEngine::new(self.viewport_width, self.viewport_height);
        let layout_root = layout_engine.layout(&styled);

        let mut renderer =
            RenderEngine::new(self.viewport_width as u32, self.viewport_height as u32);
        renderer.render(&layout_root);
        let pixels = renderer.get_pixels().to_vec();

        if !self.history.is_empty() {
            self.history.truncate(self.history_index + 1);
        }
        self.history.push(url.to_string());
        self.history_index = self.history.len() - 1;

        self.resource_manager.allocate(url, pixels.clone());

        Ok(pixels)
    }

    pub fn navigate_back(&mut self) -> Option<&str> {
        if self.can_go_back() {
            self.history_index -= 1;
            Some(&self.history[self.history_index])
        } else {
            None
        }
    }

    pub fn navigate_forward(&mut self) -> Option<&str> {
        if self.can_go_forward() {
            self.history_index += 1;
            Some(&self.history[self.history_index])
        } else {
            None
        }
    }

    pub fn current_url(&self) -> Option<&str> {
        self.history.get(self.history_index).map(String::as_str)
    }

    pub fn get_html(&self, url: &str) -> Option<&str> {
        self.resource_cache.get_cached(url)
    }

    pub fn can_go_back(&self) -> bool {
        !self.history.is_empty() && self.history_index > 0
    }

    pub fn can_go_forward(&self) -> bool {
        !self.history.is_empty() && self.history_index + 1 < self.history.len()
    }

    pub fn resize_viewport(&mut self, width: f32, height: f32) {
        self.viewport_width = width;
        self.viewport_height = height;
    }
    pub fn close_page(&mut self, page_id: &str) {
        self.resource_manager.release(page_id);
    }

    pub fn cleanup(&mut self) {
        self.resource_manager.release_all();
        self.resource_cache.clear_cache();
    }

    fn fetch_content_cached(&mut self, url: &str) -> Result<String, BrowserError> {
        if let Some(cached) = self.resource_cache.get_cached(url) {
            return Ok(cached.to_string());
        }
        let content = self.fetch_content(url)?;
        self.resource_cache.cache(url, content.clone());
        Ok(content)
    }

    fn fetch_content(&self, url: &str) -> Result<String, BrowserError> {
        if let Some(path) = url.strip_prefix("file://") {
            fs::read_to_string(path)
                .map_err(|e| BrowserError::IoError(format!("'{}': {}", path, e)))
        } else if url.starts_with("http://") || url.starts_with("https://") {
            let response = NetworkStack::fetch(url)
                .map_err(|e| BrowserError::NetworkError(e.to_string()))?;
            String::from_utf8(response.body)
                .map_err(|e| BrowserError::NetworkError(format!("UTF-8 decode error: {}", e)))
        } else {
            fs::read_to_string(url)
                .map_err(|e| BrowserError::IoError(format!("'{}': {}", url, e)))
        }
    }
}

impl Default for Browser {
    fn default() -> Self {
        Self::new()
    }
}


fn extract_style_content(html: &str) -> String {
    let lower = html.to_lowercase();
    if let Some(start) = lower.find("<style") {
        if let Some(close_bracket) = html[start..].find('>') {
            let content_start = start + close_bracket + 1;
            if let Some(end_tag) = lower[content_start..].find("</style") {
                return html[content_start..content_start + end_tag].to_string();
            }
        }
    }
    String::new()
}


#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_new_browser_has_empty_history() {
        let b = Browser::new();
        assert!(b.history.is_empty());
        assert_eq!(b.current_url(), None);
        assert!(!b.can_go_back());
        assert!(!b.can_go_forward());
    }

    #[test]
    fn test_new_browser_loading_state_is_idle() {
        let b = Browser::new();
        assert_eq!(b.loading_state, LoadingState::Idle);
    }

    #[test]
    fn test_load_file_pushes_history() {
        let mut b = Browser::new();
        let result = b.load("test/index.html");
        assert!(result.is_ok(), "load failed: {:?}", result.err());
        assert_eq!(b.history.len(), 1);
        assert_eq!(b.current_url(), Some("test/index.html"));
    }

    #[test]
    fn test_load_sets_complete_state() {
        let mut b = Browser::new();
        b.load("test/index.html").unwrap();
        assert_eq!(b.loading_state, LoadingState::Complete);
    }

    #[test]
    fn test_load_returns_pixel_buffer() {
        let mut b = Browser::new();
        let pixels = b.load("test/index.html").unwrap();
        let expected_len = (b.viewport_width as usize) * (b.viewport_height as usize) * 4;
        assert_eq!(pixels.len(), expected_len);
    }

    #[test]
    fn test_navigate_back_and_forward() {
        let mut b = Browser::new();
        b.load("test/index.html").unwrap();
        b.load("test/index.html").unwrap();

        assert_eq!(b.history.len(), 2);
        assert!(b.can_go_back());
        assert!(!b.can_go_forward());

        let back = b.navigate_back();
        assert_eq!(back, Some("test/index.html"));
        assert!(!b.can_go_back());
        assert!(b.can_go_forward());

        let fwd = b.navigate_forward();
        assert_eq!(fwd, Some("test/index.html"));
        assert!(b.can_go_back());
        assert!(!b.can_go_forward());
    }

    #[test]
    fn test_navigate_back_at_start_returns_none() {
        let mut b = Browser::new();
        b.load("test/index.html").unwrap();
        assert_eq!(b.navigate_back(), None);
    }

    #[test]
    fn test_navigate_forward_at_end_returns_none() {
        let mut b = Browser::new();
        b.load("test/index.html").unwrap();
        assert_eq!(b.navigate_forward(), None);
    }

    #[test]
    fn test_new_load_truncates_forward_history() {
        let mut b = Browser::new();
        b.load("test/index.html").unwrap();
        b.load("test/index.html").unwrap();
        b.navigate_back();
        b.load("test/index.html").unwrap();
        assert!(!b.can_go_forward());
        assert_eq!(b.history.len(), 2);
    }

    #[test]
    fn test_resize_viewport() {
        let mut b = Browser::new();
        b.resize_viewport(1024.0, 768.0);
        assert_eq!(b.viewport_width, 1024.0);
        assert_eq!(b.viewport_height, 768.0);
    }

    #[test]
    fn test_load_nonexistent_file_returns_error() {
        let mut b = Browser::new();
        let result = b.load("test/nonexistent.html");
        assert!(result.is_err());
        assert!(b.history.is_empty());
    }

    #[test]
    fn test_load_nonexistent_sets_error_state() {
        let mut b = Browser::new();
        let _ = b.load("test/nonexistent.html");
        assert!(matches!(b.loading_state, LoadingState::Error(_)));
    }

    #[test]
    fn test_load_typed_returns_browser_error() {
        let mut b = Browser::new();
        let result = b.load_typed("test/nonexistent.html");
        assert!(matches!(result, Err(BrowserError::IoError(_))));
    }


    #[test]
    fn test_cache_stores_and_retrieves() {
        let mut cache = ResourceCache::new();
        cache.cache("http://example.com", "<html></html>".to_string());
        assert_eq!(cache.get_cached("http://example.com"), Some("<html></html>"));
    }

    #[test]
    fn test_cache_miss_returns_none() {
        let cache = ResourceCache::new();
        assert_eq!(cache.get_cached("http://missing.com"), None);
    }

    #[test]
    fn test_cache_invalidate() {
        let mut cache = ResourceCache::new();
        cache.cache("http://example.com", "content".to_string());
        cache.invalidate("http://example.com");
        assert_eq!(cache.get_cached("http://example.com"), None);
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = ResourceCache::new();
        cache.cache("http://a.com", "a".to_string());
        cache.cache("http://b.com", "b".to_string());
        cache.clear_cache();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_browser_caches_loaded_page() {
        let mut b = Browser::new();
        b.load("test/index.html").unwrap();
        assert!(b.resource_cache.get_cached("test/index.html").is_some());
    }

    #[test]
    fn test_browser_uses_cache_on_second_load() {
        let mut b = Browser::new();
        b.load("test/index.html").unwrap();
        b.resource_cache.cache("test/index.html", "<html><body></body></html>".to_string());
        let result = b.load("test/index.html");
        assert!(result.is_ok());
    }


    #[test]
    fn test_css_parse_error_falls_back_to_empty_stylesheet() {
        let mut tokenizer = crate::css::CssTokenizer::new("THIS IS NOT CSS!!!".to_string());
        let stylesheet = tokenizer
            .parse()
            .unwrap_or_else(|_| crate::css::Stylesheet { rules: vec![] });
        assert!(stylesheet.rules.is_empty());
    }


    #[test]
    fn test_extract_style_content_found() {
        let html = "<html><head><style>body { color: red; }</style></head></html>";
        let css = extract_style_content(html);
        assert!(css.contains("color: red"));
    }

    #[test]
    fn test_extract_style_content_not_found() {
        let html = "<html><head></head></html>";
        let css = extract_style_content(html);
        assert!(css.is_empty());
    }
}
