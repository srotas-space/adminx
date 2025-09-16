// inside adminx/src/health.rs
use actix_web::{HttpResponse, Responder};

pub async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("AdminX is healthy!")
}
