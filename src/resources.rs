use std::collections::HashMap;
use std::time::Instant;

pub struct PageResources {
    pub url: String,
    pub pixel_buffer: Vec<u8>,
    pub allocated_at: Instant,
}

pub struct ResourceManager {
    pages: HashMap<String, PageResources>,
    next_id: u64,
}

impl ResourceManager {
    pub fn new() -> Self {
        ResourceManager {
            pages: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn allocate(&mut self, url: &str, pixels: Vec<u8>) -> String {
        let page_id = format!("{}-{}", url, self.next_id);
        self.next_id += 1;
        self.pages.insert(
            page_id.clone(),
            PageResources {
                url: url.to_string(),
                pixel_buffer: pixels,
                allocated_at: Instant::now(),
            },
        );
        page_id
    }

    pub fn release(&mut self, page_id: &str) {
        self.pages.remove(page_id);
    }

    pub fn release_all(&mut self) {
        self.pages.clear();
    }

    pub fn get(&self, page_id: &str) -> Option<&PageResources> {
        self.pages.get(page_id)
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    pub fn total_memory_bytes(&self) -> usize {
        self.pages.values().map(|r| r.pixel_buffer.len()).sum()
    }
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocate_returns_unique_ids() {
        let mut rm = ResourceManager::new();
        let id1 = rm.allocate("http://example.com", vec![0u8; 100]);
        let id2 = rm.allocate("http://example.com", vec![0u8; 100]);
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_page_count_tracks_allocations() {
        let mut rm = ResourceManager::new();
        assert_eq!(rm.page_count(), 0);
        let id = rm.allocate("http://a.com", vec![1, 2, 3]);
        assert_eq!(rm.page_count(), 1);
        rm.release(&id);
        assert_eq!(rm.page_count(), 0);
    }

    #[test]
    fn test_total_memory_bytes() {
        let mut rm = ResourceManager::new();
        rm.allocate("http://a.com", vec![0u8; 200]);
        rm.allocate("http://b.com", vec![0u8; 300]);
        assert_eq!(rm.total_memory_bytes(), 500);
    }

    #[test]
    fn test_release_all_clears_resources() {
        let mut rm = ResourceManager::new();
        rm.allocate("http://a.com", vec![0u8; 100]);
        rm.allocate("http://b.com", vec![0u8; 100]);
        rm.release_all();
        assert_eq!(rm.page_count(), 0);
        assert_eq!(rm.total_memory_bytes(), 0);
    }

    #[test]
    fn test_get_returns_correct_resources() {
        let mut rm = ResourceManager::new();
        let id = rm.allocate("http://example.com", vec![1, 2, 3]);
        let res = rm.get(&id).unwrap();
        assert_eq!(res.url, "http://example.com");
        assert_eq!(res.pixel_buffer, vec![1, 2, 3]);
    }

    #[test]
    fn test_get_unknown_id_returns_none() {
        let rm = ResourceManager::new();
        assert!(rm.get("nonexistent").is_none());
    }

    #[test]
    fn test_resource_isolation() {
        let mut rm = ResourceManager::new();
        let id1 = rm.allocate("http://a.com", vec![1, 2, 3]);
        let id2 = rm.allocate("http://b.com", vec![4, 5, 6]);
        assert_eq!(rm.get(&id1).unwrap().pixel_buffer, vec![1, 2, 3]);
        assert_eq!(rm.get(&id2).unwrap().pixel_buffer, vec![4, 5, 6]);
        rm.release(&id1);
        assert!(rm.get(&id1).is_none());
        assert!(rm.get(&id2).is_some());
    }
}
