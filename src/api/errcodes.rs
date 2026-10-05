//! Published error codes (Go `pkg/api/errcodes` parity, port-spec §3).
//!
//! The canonical string values live in [`crate::platform::errcodes`]; this
//! facade re-exports them both under their `SCREAMING_SNAKE` wire names and
//! under the Go constant names (`NotFound`, `InvalidInput`, …), so external
//! consumers can use either spelling without importing server internals.

pub use crate::platform::errcodes::{
    CANCELED, CONFLICT, DEADLINE_EXCEEDED, FORBIDDEN, INTERNAL, INVALID_INPUT, METHOD_NOT_ALLOWED,
    NOT_FOUND, PAYLOAD_TOO_LARGE, UNAUTHORIZED,
};

pub use crate::platform::errcodes::{
    CANCELED as Canceled, CONFLICT as Conflict, DEADLINE_EXCEEDED as DeadlineExceeded,
    FORBIDDEN as Forbidden, INTERNAL as Internal, INVALID_INPUT as InvalidInput,
    METHOD_NOT_ALLOWED as MethodNotAllowed, NOT_FOUND as NotFound,
    PAYLOAD_TOO_LARGE as PayloadTooLarge, UNAUTHORIZED as Unauthorized,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn go_parity_names_and_wire_values() {
        assert_eq!(NotFound, "NOT_FOUND");
        assert_eq!(InvalidInput, "INVALID_INPUT");
        assert_eq!(Unauthorized, "UNAUTHORIZED");
        assert_eq!(Forbidden, "FORBIDDEN");
        assert_eq!(Conflict, "CONFLICT");
        assert_eq!(Canceled, "CANCELED");
        assert_eq!(DeadlineExceeded, "DEADLINE_EXCEEDED");
        assert_eq!(Internal, "INTERNAL");
        assert_eq!(MethodNotAllowed, "METHOD_NOT_ALLOWED");
        assert_eq!(PayloadTooLarge, "PAYLOAD_TOO_LARGE");
    }
}
