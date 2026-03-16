#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaginatedList<T: serde::Serialize> {
    pub items: Vec<T>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

impl<T: serde::Serialize> PaginatedList<T> {
    pub fn paginate(items: Vec<T>, page: usize, page_size: usize) -> Self {
        let total = items.len();
        let total_pages = if page_size == 0 { 1 } else { total.div_ceil(page_size) };
        let offset = (page.saturating_sub(1)) * page_size;
        let items = items.into_iter().skip(offset).take(page_size).collect();
        Self { items, total, page, page_size, total_pages }
    }
}
