use serde::Deserialize;

use crate::runtime::flow::Flow;
use crate::runtime::nodes::*;
use edgelink_macro::*;

#[derive(Debug)]
#[flow_node("yaml")]
struct YamlNode {
    base: FlowNode,
    config: YamlNodeConfig,
}

impl YamlNode {
    fn build(_flow: &Flow, state: FlowNode, config: &RedFlowNodeConfig) -> crate::Result<Box<dyn FlowNodeBehavior>> {
        let mut yaml_config = YamlNodeConfig::deserialize(&config.rest)?;
        // yaml_config.property =
        if let Some(pro) = config.rest.get("property").map(|x| {
            if let serde_json::Value::String(pro) = x {
                pro.to_string()
            } else {
                "payload".to_string()
            }
        }) {
            yaml_config.property = pro;
        }
        let node = YamlNode { base: state, config: yaml_config };
        Ok(Box::new(node))
    }
}

#[derive(Deserialize, Debug)]
struct YamlNodeConfig {
    #[serde(skip, default)]
    pub property: String,
}

impl YamlNode {
    async fn uow(&self, msg: MsgHandle) -> crate::Result<()> {
        let mut msg_guard = msg.write().await;
        let message_key = &self.config.property;
        let option = msg_guard.get(message_key);
        match option {
            None => {}
            Some(message) => {
                match message {
                    Variant::String(message) => {
                        if let Ok(Variant::Object(value)) = serde_yaml::from_str(message) {
                            msg_guard.set(message_key.to_owned(), Variant::Object(value));
                        }
                    }
                    Variant::Object(message_obj) => {
                        let yaml_str = serde_yaml::to_string(&message_obj)?;
                        msg_guard.set(message_key.to_owned(), Variant::String(yaml_str));
                    }
                    // Variant::Bytes(_) => {}
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

#[async_trait]
impl FlowNodeBehavior for YamlNode {
    fn get_node(&self) -> &FlowNode {
        &self.base
    }

    async fn run(self: Arc<Self>, stop_token: CancellationToken) {
        while !stop_token.is_cancelled() {
            let node = self.clone();
            with_uow(node.as_ref(), stop_token.clone(), |node, msg| async move { node.uow(msg).await }).await;
        }
    }
}
