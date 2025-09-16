// adminx/src/nested.rs
use actix_web::Scope;

pub trait AdmixNestedResource: Send + Sync {
    fn base_path(&self) -> &'static str;
    fn parent_param(&self) -> &'static str;
    fn as_scope(&self) -> Scope;
}
