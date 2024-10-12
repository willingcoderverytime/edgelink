use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tokio::net::UdpSocket;

use base64::prelude::*;
use serde::Deserialize;

use crate::runtime::flow::Flow;
use crate::runtime::nodes::*;
use edgelink_macro::*;
use validator::{Validate, ValidationError};





#[derive(Debug)]
#[flow_node("file")]
struct FileNode {
    base: FlowNode,
    config: FileNodeConfig,
}

impl FileNode {
    fn build(_flow: &Flow, state: FlowNode, config: &RedFlowNodeConfig) -> crate::Result<Box<dyn FlowNodeBehavior>> {
        let udp_config = FileNodeConfig::deserialize(&config.rest)?;



        let node = FileNode { base: state, config: udp_config };
        Ok(Box::new(node))
    }
}

#[derive(Deserialize,Validate, Debug)]
struct FileNodeConfig {
    #[validate(length(min = 1))]
    filename: String,
    #[serde(rename = "filenameType")]
    filename_type: Option<String>,
    #[serde( rename= "appendNewline")]
    append_new_line: String,
    #[serde( rename= "overwriteFile")]
    overwrite_file: String,
    create_dir: bool,
    encoding: String,

}

impl FileNode {
    async fn uow(&self, msg: MsgHandle, socket: &UdpSocket) -> crate::Result<()> {
        let msg_guard = msg.read().await;
        if let Some(payload) = msg_guard.get("payload") {
            let remote_addr = std::net::SocketAddr::new(
                self.config.addr.unwrap(), // TODO FIXME
                self.config.port.unwrap(),
            );

            if let Some(bytes) = payload.as_bytes() {
                if self.config.base64 {
                    let b64_str = BASE64_STANDARD.encode(bytes);
                    let bytes = b64_str.as_bytes();
                    socket.send_to(bytes, remote_addr).await?;
                } else {
                    socket.send_to(bytes, remote_addr).await?;
                }
            }
            if let Some(bytes) = payload.to_bytes() {
                socket.send_to(&bytes, remote_addr).await?;
            } else {
                log::warn!("Failed to convert payload into bytes");
            }
        }

        Ok(())
    }
}

#[async_trait]
impl FlowNodeBehavior for FileNode {
    fn get_node(&self) -> &FlowNode {
        &self.base
    }

    async fn run(self: Arc<Self>, stop_token: CancellationToken) {
        let local_addr: SocketAddr = match self.config.outport {
            Some(port) => SocketAddr::new(self.config.iface.unwrap(), port),
            _ => match self.config.ipv {
                UdpIpV::V4 => "0.0.0.0:0".parse().unwrap(),
                UdpIpV::V6 => "[::]:0".parse().unwrap(),
            },
        };

        match tokio::net::UdpSocket::bind(local_addr).await {
            Ok(socket) => {
                let socket = Arc::new(socket);
                while !stop_token.is_cancelled() {
                    let cloned_socket = socket.clone();

                    let node = self.clone();
                    with_uow(node.as_ref(), stop_token.clone(), |node, msg| async move {
                        node.uow(msg, &cloned_socket).await
                    })
                        .await;
                }
            }

            Err(e) => {
                log::error!("Can not bind local address: {:?}", e);
            }
        }
    }
}
