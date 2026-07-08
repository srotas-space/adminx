use serde::Serialize;

/// Default page size when the client doesn't specify one.
pub const DEFAULT_PER_PAGE: u64 = 25;

/// Hard upper bound on client-requested page size, to cap memory/DB load.
pub const MAX_PER_PAGE: u64 = 200;

/// Clamp client-supplied pagination to safe bounds: `page >= 1` and
/// `1 <= per_page <= MAX_PER_PAGE`.
///
/// This prevents the `(page - 1) * per_page` u64 underflow (which panics under
/// the dev profile's overflow checks and wraps to a huge skip in release) and
/// the `skip / limit` divide-by-zero panic when a client sends `page=0` or
/// `per_page=0`, as well as memory exhaustion from an unbounded `per_page`.
pub fn clamp_pagination(page: u64, per_page: u64) -> (u64, u64) {
    (page.max(1), per_page.clamp(1, MAX_PER_PAGE))
}

#[derive(Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub per_page: u64,
}
