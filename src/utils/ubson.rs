use actix_web::error::ErrorInternalServerError;
use actix_web::Error;
use mongodb::bson::{to_bson, Bson};

pub fn convert_to_bson<T: serde::Serialize>(value: &T) -> Result<Bson, Error> {
    to_bson(value).map_err(ErrorInternalServerError)
}
