#[cfg(test)]
use crate::hash::hex;
#[cfg(test)]
use crate::jobs::PrimitiveType;
#[cfg(test)]
use std::net::SocketAddr;

mod dispatch;
mod explorer;
mod explorer_routes;
mod gateway;
mod http;
mod mutations;
mod node;
mod parse;
mod read_routes;
mod render;
mod response;
mod tensor_routes;
mod types;
mod websocket;
#[cfg(test)]
use explorer::{hardware_class_label, primitive_label};
pub use gateway::{RpcGateway, RpcPolicy};
#[cfg(test)]
use http::{
    ParsedHttpRequest, read_http_request_from, split_path_and_auth_token, try_parse_http_request,
};
pub use http::{RpcHttpServer, http_response_text};
pub use node::RpcNode;
use parse::{parse_address, parse_hash};
pub use types::{RpcRequest, RpcResponse};
#[cfg(test)]
use websocket::{
    base64_encode, json_string_field, json_usize_field, read_websocket_text_frame,
    websocket_accept_key, write_websocket_frame,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::{Chain, ChainParams, HardwareClass, JobState};
    use crate::faucet::Faucet;
    use crate::jobs::{LinearTrainingStepJob, LinearTrainingStepSpec, MatmulJob, TensorOpReceipt};
    use crate::profile::ChainProfile;
    use crate::tensor::{DType, Tensor};
    use crate::types::{address, hash_bytes};
    use crate::verify::FreivaldsParams;

    mod http;
    mod routes;
    mod tensors;
    mod websocket;
}
