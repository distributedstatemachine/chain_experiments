use super::{RpcNode, RpcRequest, RpcResponse};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RpcPolicy {
    pub auth_token: Option<String>,
    pub max_body_bytes: usize,
    pub max_requests_per_client: u64,
}

impl Default for RpcPolicy {
    fn default() -> Self {
        Self {
            auth_token: None,
            max_body_bytes: 1024 * 1024,
            max_requests_per_client: 1_000,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RpcGateway {
    pub node: RpcNode,
    pub policy: RpcPolicy,
    request_counts: BTreeMap<String, u64>,
}

impl RpcGateway {
    pub fn new(node: RpcNode, policy: RpcPolicy) -> Self {
        Self {
            node,
            policy,
            request_counts: BTreeMap::new(),
        }
    }

    pub fn handle(
        &mut self,
        client_id: &str,
        auth_token: Option<&str>,
        request: &RpcRequest,
    ) -> RpcResponse {
        if request.body.len() > self.policy.max_body_bytes {
            return RpcNode::response(413, "request body too large");
        }
        if let Some(response) = self.authorize_request(client_id, auth_token) {
            return response;
        }
        self.node.handle_mut(request)
    }

    pub(super) fn authorize_request(
        &mut self,
        client_id: &str,
        auth_token: Option<&str>,
    ) -> Option<RpcResponse> {
        if let Some(required) = &self.policy.auth_token
            && auth_token != Some(required.as_str())
        {
            return Some(RpcNode::response(401, "unauthorized"));
        }
        let count = self.request_counts.entry(client_id.to_owned()).or_default();
        if *count >= self.policy.max_requests_per_client {
            return Some(RpcNode::response(429, "rate limit exceeded"));
        }
        *count += 1;
        None
    }

    pub fn request_count(&self, client_id: &str) -> u64 {
        self.request_counts.get(client_id).copied().unwrap_or(0)
    }
}
