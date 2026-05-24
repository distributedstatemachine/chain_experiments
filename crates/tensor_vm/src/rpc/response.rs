use super::{RpcNode, RpcResponse};

impl RpcNode {
    pub(super) fn ok(&self, body: String) -> RpcResponse {
        RpcResponse { status: 200, body }
    }

    pub(super) fn accepted(&self) -> RpcResponse {
        RpcResponse {
            status: 202,
            body: "{\"accepted\":true}".to_owned(),
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
            body: format!("{{\"error\":\"{message}\"}}"),
        }
    }
}
