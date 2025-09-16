// src/utils/rbac.rs
use crate::menu::MenuAction;
use crate::resource::AdmixResource;

pub fn has_permission(resource: &dyn AdmixResource, roles: &[String], action: MenuAction) -> bool {
    let permissions = resource.allowed_roles_with_permissions();

    for role in roles {
        if let Some(perms) = permissions.get(role) {
            if let Some(perms_array) = perms.as_array() {
                for p in perms_array {
                    if p == action.as_str() {
                        return true;
                    }
                }
            }
        }
    }

    false
}

