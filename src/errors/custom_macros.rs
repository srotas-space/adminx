// crates/adminx/src/errors/custom_macros.rs

#[macro_export]
macro_rules! handle_custom_error {
    (bad_request, $code:expr, $msg:expr) => {
        return Err($crate::errors::custom_error::CustomError::BadRequest($code, $msg.to_string()).into());
    };
    (invalid_request, $code:expr, $msg:expr) => {
        return Err($crate::errors::custom_error::CustomError::InvalidRequest($code, $msg.to_string()).into());
    };
    (internal_error, $code:expr, $msg:expr) => {
        return Err($crate::errors::custom_error::CustomError::InternalError($code, $msg.to_string()).into());
    };
    (unauthorized, $code:expr, $msg:expr) => {
        return Err($crate::errors::custom_error::CustomError::Unauthorized($code, $msg.to_string()).into());
    };
    (not_found, $code:expr, $msg:expr) => {
        return Err($crate::errors::custom_error::CustomError::NotFound($code, $msg.to_string()).into());
    };
    (conflict, $code:expr, $msg:expr) => {
        return Err($crate::errors::custom_error::CustomError::Conflict($code, $msg.to_string()).into());
    };
}


#[macro_export]
macro_rules! custom_error_expression {
    (bad_request, $code:expr, $msg:expr) => {
        $crate::errors::custom_error::CustomError::BadRequest($code, $msg.to_string())
    };
    (invalid_request, $code:expr, $msg:expr) => {
        $crate::errors::custom_error::CustomError::InvalidRequest($code, $msg.to_string())
    };
    (internal_error, $code:expr, $msg:expr) => {
        $crate::errors::custom_error::CustomError::InternalError($code, $msg.to_string())
    };
    (unauthorized, $code:expr, $msg:expr) => {
        $crate::errors::custom_error::CustomError::Unauthorized($code, $msg.to_string())
    };
    (not_found, $code:expr, $msg:expr) => {
        $crate::errors::custom_error::CustomError::NotFound($code, $msg.to_string())
    };
    (conflict, $code:expr, $msg:expr) => {
        $crate::errors::custom_error::CustomError::Conflict($code, $msg.to_string())
    };
}
