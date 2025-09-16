// src/actions.rs
use actix_web::{HttpRequest, web, HttpResponse};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

// Type for boxed handler functions with dynamic input
pub type DynHandler = 
    fn(HttpRequest, web::Path<String>, web::Json<Value>) -> Pin<Box<dyn Future<Output = HttpResponse> + Send>>;

pub struct CustomAction {
    pub name: &'static str,
    pub method: &'static str, // "GET", "POST"
    pub handler: DynHandler,
}
