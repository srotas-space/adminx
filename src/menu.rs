// crates/adminx/src/menu.rs

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuItem {
    pub title: String,
    pub path: String,
    pub children: Option<Vec<MenuItem>>,
    pub icon: Option<String>,
    pub order: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuAction {
    List,
    View,
    Create,
    Edit,
    Delete,
}



impl MenuAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            MenuAction::List => "list",
            MenuAction::View => "view",
            MenuAction::Create => "create",
            MenuAction::Edit => "edit",
            MenuAction::Delete => "delete",
        }
    }

    pub fn to_path(&self, base_path: &str) -> String {
        match self {
            MenuAction::List => base_path.to_string(),
            MenuAction::Create => format!("{}/create", base_path),
            MenuAction::View => format!("{}/{{id}}", base_path),
            MenuAction::Edit => format!("{}/{{id}}/edit", base_path),
            MenuAction::Delete => base_path.to_string(),
        }
    }
}
