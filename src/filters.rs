use mongodb::bson::{doc, Document};

#[derive(Debug)]
pub struct FilterOptions {
    pub filter: Document,
    pub sort: Option<Document>,
    pub skip: u64,
    pub limit: u64,
}

pub fn parse_query(query: &str) -> FilterOptions {
    let params: Vec<(&str, &str)> = querystring::querify(query);

    let mut filter_doc = Document::new();
    let mut sort_doc = None;
    let mut page = 1u64;
    let mut per_page = 25u64;

    for (key, value) in params {
        match key {
            "page" => page = value.parse().unwrap_or(1),
            "per_page" => per_page = value.parse().unwrap_or(25),
            "sort" => {
                let direction = if value.starts_with('-') { -1 } else { 1 };
                let field = value.trim_start_matches('-').to_string();
                sort_doc = Some(doc! { field: direction });
            }
            _ => {
                if !value.is_empty() {
                    filter_doc.insert(key, value);
                }
            }
        }
    }

    let skip = (page - 1) * per_page;

    FilterOptions {
        filter: filter_doc,
        sort: sort_doc,
        skip,
        limit: per_page,
    }
}
