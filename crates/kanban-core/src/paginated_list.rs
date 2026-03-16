#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaginatedList<T> {
    pub items: Vec<T>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

impl<T> PaginatedList<T> {
    pub fn paginate(items: Vec<T>, page: usize, page_size: usize) -> Self {
        let total = items.len();
        let page = page.max(1);
        let total_pages = if total == 0 || page_size == 0 {
            1
        } else {
            total.div_ceil(page_size)
        };
        let offset = (page - 1).saturating_mul(page_size);
        let items = if page_size == 0 {
            vec![]
        } else {
            items.into_iter().skip(offset).take(page_size).collect()
        };
        Self {
            items,
            total,
            page,
            page_size,
            total_pages,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paginate_normal() {
        let items: Vec<i32> = (1..=10).collect();
        let result = PaginatedList::paginate(items, 2, 3);
        assert_eq!(result.items, vec![4, 5, 6]);
        assert_eq!(result.total, 10);
        assert_eq!(result.total_pages, 4);
        assert_eq!(result.page, 2);
    }

    #[test]
    fn test_paginate_empty() {
        let result = PaginatedList::<i32>::paginate(vec![], 1, 10);
        assert_eq!(result.items, vec![]);
        assert_eq!(result.total, 0);
        assert_eq!(result.total_pages, 1);
        assert_eq!(result.page, 1);
    }

    #[test]
    fn test_paginate_page_zero_normalizes() {
        let items: Vec<i32> = (1..=5).collect();
        let result = PaginatedList::paginate(items, 0, 3);
        assert_eq!(result.page, 1);
        assert_eq!(result.items, vec![1, 2, 3]);
    }

    #[test]
    fn test_paginate_out_of_bounds() {
        let items: Vec<i32> = (1..=5).collect();
        let result = PaginatedList::paginate(items, 3, 5);
        assert_eq!(result.items, vec![]);
        assert_eq!(result.total, 5);
        assert_eq!(result.total_pages, 1);
    }

    #[test]
    fn test_paginate_fits_on_one_page() {
        let items: Vec<i32> = (1..=3).collect();
        let result = PaginatedList::paginate(items, 1, 10);
        assert_eq!(result.items, vec![1, 2, 3]);
        assert_eq!(result.total_pages, 1);
    }

    #[test]
    fn test_paginate_exact_boundary() {
        let items: Vec<i32> = (1..=9).collect();
        let result = PaginatedList::paginate(items, 3, 3);
        assert_eq!(result.items, vec![7, 8, 9]);
        assert_eq!(result.total_pages, 3);
    }
}
