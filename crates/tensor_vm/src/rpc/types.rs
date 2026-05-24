#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RpcRequest {
    pub method: String,
    pub path: String,
    pub body: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RpcResponse {
    pub status: u16,
    pub body: String,
}
