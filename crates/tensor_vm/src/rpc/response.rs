use super::{RpcNode, RpcResponse};
use serde_json::json;

impl RpcNode {
    pub(super) fn ok(&self, body: String) -> RpcResponse {
        RpcResponse { status: 200, body }
    }

    pub(super) fn accepted(&self) -> RpcResponse {
        RpcResponse {
            status: 202,
            body: json!({ "accepted": true }).to_string(),
        }
    }

    pub(super) fn bad_request(&self, message: &str) -> RpcResponse {
        Self::response(400, message)
    }

    pub(super) fn not_found(&self, message: &str) -> RpcResponse {
        Self::response(404, message)
    }

    pub(super) fn conflict(&self, message: &str) -> RpcResponse {
        Self::response(409, message)
    }

    pub(super) fn response(status: u16, message: &str) -> RpcResponse {
        RpcResponse {
            status,
            body: json!({ "error": message }).to_string(),
        }
    }
}
