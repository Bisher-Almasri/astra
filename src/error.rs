#[derive(Debug, Clone, PartialEq)]
pub enum BrowserError {
    HtmlParseError(String),
    CssParseError(String),
    NetworkError(String),
    IoError(String),
    RenderError(String),
}

impl std::fmt::Display for BrowserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrowserError::HtmlParseError(msg) => write!(f, "HTML parse error: {}", msg),
            BrowserError::CssParseError(msg) => write!(f, "CSS parse error: {}", msg),
            BrowserError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            BrowserError::IoError(msg) => write!(f, "I/O error: {}", msg),
            BrowserError::RenderError(msg) => write!(f, "Render error: {}", msg),
        }
    }
}

impl std::error::Error for BrowserError {}

pub fn error_page_html(error: &BrowserError) -> String {
    let (title, detail) = match error {
        BrowserError::HtmlParseError(msg) => ("Page could not be displayed", msg.as_str()),
        BrowserError::CssParseError(msg) => ("Stylesheet error", msg.as_str()),
        BrowserError::NetworkError(msg) => ("Failed to load page", msg.as_str()),
        BrowserError::IoError(msg) => ("File not found", msg.as_str()),
        BrowserError::RenderError(msg) => ("Rendering error", msg.as_str()),
    };
    format!(
        "<html><body><h1>Error: {}</h1><p>{}</p></body></html>",
        title, detail
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_html_parse_error() {
        let e = BrowserError::HtmlParseError("unexpected EOF".to_string());
        assert!(e.to_string().contains("HTML parse error"));
        assert!(e.to_string().contains("unexpected EOF"));
    }

    #[test]
    fn test_display_network_error() {
        let e = BrowserError::NetworkError("connection refused".to_string());
        assert!(e.to_string().contains("Network error"));
    }

    #[test]
    fn test_display_io_error() {
        let e = BrowserError::IoError("no such file".to_string());
        assert!(e.to_string().contains("I/O error"));
    }

    #[test]
    fn test_display_css_parse_error() {
        let e = BrowserError::CssParseError("missing semicolon".to_string());
        assert!(e.to_string().contains("CSS parse error"));
    }

    #[test]
    fn test_display_render_error() {
        let e = BrowserError::RenderError("buffer overflow".to_string());
        assert!(e.to_string().contains("Render error"));
    }

    #[test]
    fn test_error_page_html_network() {
        let e = BrowserError::NetworkError("timeout".to_string());
        let page = error_page_html(&e);
        assert!(page.contains("<html>"));
        assert!(page.contains("Failed to load page"));
        assert!(page.contains("timeout"));
    }

    #[test]
    fn test_error_page_html_io() {
        let e = BrowserError::IoError("not found".to_string());
        let page = error_page_html(&e);
        assert!(page.contains("File not found"));
    }

    #[test]
    fn test_error_page_html_parse() {
        let e = BrowserError::HtmlParseError("bad tag".to_string());
        let page = error_page_html(&e);
        assert!(page.contains("Page could not be displayed"));
        assert!(page.contains("bad tag"));
    }
}
