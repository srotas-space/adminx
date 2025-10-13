// src/actions.rs
use actix_web::HttpRequest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

// Flexible action handler:
// - `path`: arbitrary path segments (e.g., ["events", "{id}", "actions", "{name}"] )
// - `body`: JSON payload (can be `{}` if none provided)
pub type ActionHandler =
    fn(HttpRequest, Vec<String>, Value) -> Pin<Box<dyn Future<Output = actix_web::HttpResponse>>>;

// Describe a custom action exposed by a Resource.
// `ui` is optional; if None, the frontend can fall back to a generic JSON payload textarea.
pub struct CustomAction {
    pub name: &'static str,      // e.g., "set_status"
    pub method: &'static str,    // e.g., "POST" | "GET"
    pub handler: ActionHandler,  // server-side executor
    pub ui: Option<ActionUi>,    // optional metadata for dynamic forms
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionUi {
    pub label: Option<String>,
    pub confirm: Option<String>,
    pub fields: Option<Vec<ActionField>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionField {
    pub name: String,                 // was &'static str
    pub field_type: String,           // was &'static str
    pub label: Option<String>,        // was Option<&'static str>
    pub required: Option<bool>,
    pub options: Option<Vec<Value>>,
}


/* --------------------------------------------------------------------------
   Backwards-compat (optional):
   If you still have legacy handlers with `(HttpRequest, web::Path<String>, web::Json<Value>)`,
   you can write tiny adapters in the resources/controllers to convert them to `ActionHandler`.
   Example adapter (put near usage, not necessarily here):

   fn adapt_legacy<F>(
       f: F
   ) -> ActionHandler
   where
       F: Fn(HttpRequest, String, Value) -> Pin<Box<dyn Future<Output = HttpResponse>>> + Copy + 'static
   {
       move |req, path, body| {
           // Example: join segments with "/" if needed by legacy code
           let joined = path.join("/");
           f(req, joined, body)
       }
   }
-------------------------------------------------------------------------- */
