// crates/adminx/src/registry.rs
use crate::resource::AdmixResource;
use std::sync::RwLock;
use lazy_static::lazy_static;
use crate::menu::{MenuItem};
use std::collections::HashMap;

lazy_static! {
    static ref RESOURCE_REGISTRY: RwLock<Vec<Box<dyn AdmixResource>>> = RwLock::new(vec![]);
}

/// Register a resource globally
pub fn register_resource(resource: Box<dyn AdmixResource>) {
    RESOURCE_REGISTRY.write().unwrap().push(resource);
}

pub fn all_resources() -> Vec<Box<dyn AdmixResource>> {
    RESOURCE_REGISTRY
        .read()
        .unwrap()
        .iter()
        .map(|r| r.clone_box())
        .collect()
}

/// Collect all the menus from registered resources and group them properly
pub fn get_registered_menus() -> Vec<MenuItem> {
    let resources = RESOURCE_REGISTRY.read().unwrap();
    let mut grouped_menus: HashMap<String, Vec<MenuItem>> = HashMap::new();
    let mut ungrouped_menus: Vec<MenuItem> = Vec::new();

    // Group resources by their menu_group
    for resource in resources.iter() {
        if let Some(menu_item) = resource.generate_menu() {
            if let Some(group_name) = resource.menu_group() {
                // Add to grouped menus
                grouped_menus
                    .entry(group_name.to_string())
                    .or_insert_with(Vec::new)
                    .push(menu_item);
            } else {
                // Add to ungrouped menus (top-level)
                ungrouped_menus.push(menu_item);
            }
        }
    }

    let mut final_menus = Vec::new();

    // Create parent menus for groups with multiple resources
    for (group_name, mut children) in grouped_menus {
        // Sort children by title
        children.sort_by(|a, b| a.title.cmp(&b.title));
        
        let parent_menu = MenuItem {
            title: group_name,
            path: String::new(), // Non-clickable parent
            icon: Some("folder".to_string()),
            order: Some(5), // Groups appear before ungrouped items
            children: Some(children),
        };
        final_menus.push(parent_menu);
    }

    // Add ungrouped menus (resources without menu_group)
    final_menus.extend(ungrouped_menus);

    // Sort final menus by order and then by title
    final_menus.sort_by(|a, b| {
        match (a.order, b.order) {
            (Some(a_order), Some(b_order)) => a_order.cmp(&b_order),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.title.cmp(&b.title),
        }
    });

    final_menus
}

/// Clear all registered resources (useful for testing)
pub fn clear_registry() {
    RESOURCE_REGISTRY.write().unwrap().clear();
}

/// Get count of registered resources
pub fn resource_count() -> usize {
    RESOURCE_REGISTRY.read().unwrap().len()
}