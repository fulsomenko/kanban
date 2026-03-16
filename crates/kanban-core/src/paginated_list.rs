use crate::pagination::Page;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaginatedList<T> {
    pub items: Vec<T>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

impl<T: serde::Serialize> PaginatedList<T> {
    pub fn paginate(items: Vec<T>, page: usize, page_size: usize) -> Self {
        let total = items.len();
        let scroll_page = Page::new(total);
        let total_pages = scroll_page.get_page_info(page_size).total_pages;
        let offset = page.saturating_sub(1) * page_size;
        let items = items.into_iter().skip(offset).take(page_size).collect();
        Self { items, total, page, page_size, total_pages }
    }
}
