// /Users/xsm/Documents/workspace/XSM/crates/adminx/src/helpers/custom_helper.rs

use actix_web::{web, HttpRequest, HttpResponse};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

// For routes like "/{id}/<action>"
pub fn adapt_action_with_id(
    handler: crate::actions::ActionHandler,
) -> impl Fn(HttpRequest, web::Path<String>, web::Json<Value>) -> Pin<Box<dyn Future<Output = HttpResponse>>> + Clone + 'static
{
    move |req, id, body| {
        let segments = vec![id.into_inner()];
        let payload = body.into_inner();
        handler(req, segments, payload)
    }
}

pub fn adapt_action_get_with_id(
    handler: crate::actions::ActionHandler,
) -> impl Fn(HttpRequest, web::Path<String>) -> Pin<Box<dyn Future<Output = HttpResponse>>> + Clone + 'static
{
    move |req, id| {
        let segments = vec![id.into_inner()];
        handler(req, segments, Value::Null)
    }
}
