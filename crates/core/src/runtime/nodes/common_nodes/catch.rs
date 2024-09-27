use std::sync::Arc;

use crate::runtime::flow::Flow;
use crate::runtime::nodes::*;
use edgelink_macro::*;

#[flow_node("catch")]
struct CatchNode {
    base: FlowNode,
}

impl CatchNode {
    fn build(_flow: &Flow, state: FlowNode, _config: &RedFlowNodeConfig) -> crate::Result<Box<dyn FlowNodeBehavior>> {
        let node = CatchNode { base: state };
        Ok(Box::new(node))
    }
}

#[async_trait]
impl FlowNodeBehavior for CatchNode {
    fn get_node(&self) -> &FlowNode {
        &self.base
    }

    async fn run(self: Arc<Self>, stop_token: CancellationToken) {
        while !stop_token.is_cancelled() {
            let cancel = stop_token.child_token();
            with_uow(self.as_ref(), cancel.child_token(), |node, msg| async move {
                node.fan_out_one(&Envelope { port: 0, msg }, cancel.child_token()).await?;
                Ok(())
            })
            .await;
        }
    }
}