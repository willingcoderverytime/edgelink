use itertools::Itertools;
use jsonschema::Validator;
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::RwLockWriteGuard;

use crate::runtime::flow::Flow;
use crate::runtime::nodes::*;
use edgelink_macro::*;

#[derive(Debug)]
#[flow_node("json")]
struct JsonNode {
    base: FlowNode,
    config: JsonNodeConfig,
}
#[derive(Deserialize, Debug)]
struct JsonNodeConfig {
    #[serde(skip, default)]
    pub pretty: bool,
    #[serde(skip, default)]
    pub action: String,
    #[serde(skip, default)]
    pub property: String,
    pub scheme: String,
    #[serde(skip, default)]
    pub compile_schema: Option<Validator>,
}
impl JsonNode {
    fn build(_flow: &Flow, state: FlowNode, config: &RedFlowNodeConfig) -> crate::Result<Box<dyn FlowNodeBehavior>> {
        let mut json_config = JsonNodeConfig::deserialize(&config.rest)?;
        let schema = serde_json::from_str(json_config.scheme.as_str())?;
        if let Ok(validator) = jsonschema::options().build(&schema) {
            json_config.compile_schema = Some(validator);
        }
        if let Some(pro) = config.rest.get("property").map(|x| {
            if let Value::String(pro) = x {
                pro.to_string()
            } else {
                "payload".to_string()
            }
        }) {
            json_config.property = pro;
        }

        if let Some(action) = config.rest.get("action").map(|x| {
            if let Value::String(act) = x {
                act.to_string()
            } else {
                "".to_string()
            }
        }) {
            json_config.action = action;
        }
        if let Some(pretty) =
            config.rest.get("pretty").map(|x| matches!(x,&Value::Null))
        {
            json_config.pretty = pretty;
        }
        let node = JsonNode { base: state, config: json_config };
        Ok(Box::new(node))
    }
}

impl JsonNode {
    async fn uow(&self, msg: MsgHandle) -> crate::Result<()> {
        let mut msg_guard = msg.write().await;

        let mut validate = false;
        let valid = self.config.compile_schema.as_ref().unwrap();
        match msg_guard.get("schema") {
            None => {}
            Some(schema) => {
                 if let    Variant::String(schema) =schema {
                        if schema != &self.config.scheme {
                            let schema = serde_json::from_str(schema)?;
                            if let Ok(_validator) = jsonschema::options().build(&schema) {
                                // todo json node 需要进行对node的更改.
                                // self.config.compile_schema=validator;
                            }
                        }
                }
                validate = true;
            }
        }

        let message_key = &self.config.property;
        let option = msg_guard.get(message_key).cloned();
        match option {
            None => {}
            Some(message) => {
                match message {
                    Variant::String(message) => {
                        if self.config.action.is_empty() || self.config.action == "obj" {
                            match serde_yaml::from_str(&message) {
                                Ok(Value::Object(value)) => {
                                    let variant = Value::Object(value);
                                    if validate {
                                        match valid.validate(&variant) {
                                            Ok(_) => {
                                                msg_guard.remove("schema");
                                            }
                                            Err(err) => msg_guard.set(
                                                "schemaError".to_string(),
                                                Variant::from(err.into_iter().map(|e| e.to_string()).join(",")),
                                            ),
                                        };
                                    }
                                    msg_guard.set(message_key.to_string(), Variant::from(variant));
                                }
                                Ok(_) => {}
                                Err(err) => {
                                    return Err(err.into());
                                }
                            }
                        } else if validate {
                                match serde_yaml::from_str(&message) {
                                    Ok(Variant::String(value)) => {
                                        let value1 = serde_json::Value::String(value);
                                        match valid.validate(&value1) {
                                            Ok(_) => {
                                                msg_guard.remove("schema");
                                            }
                                            Err(err) => {
                                                msg_guard.set(
                                                    "schemaError".to_string(),
                                                    Variant::from(err.into_iter().map(|e| e.to_string()).join(",")),
                                                )
                                                // todo done(`${RED._("json.errors.schema-error")}: ${ajv.errorsText(this.compiledSchema.errors)}`);
                                            }
                                        };
                                    }
                                    _ => {
                                        // todo done(`${RED._("json.errors.schema-error")}: ${ajv.errorsText(this.compiledSchema.errors)}`);
                                    }
                                }
                            }
                    }
                    Variant::Object(message_obj) => {
                        self.handle_deser(&mut msg_guard, validate, valid, message_key, Variant::Object(message_obj));
                    }
                    Variant::Bool(message_obj) => {
                        self.handle_deser(&mut msg_guard, validate, valid, message_key, Variant::Bool(message_obj));
                    }
                    Variant::Number(message_obj) => {
                        self.handle_deser(&mut msg_guard, validate, valid, message_key, Variant::Number(message_obj));
                    }
                    _ => {
                        // send(msg); done();
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_deser(
        &self,
        msg_guard: &mut RwLockWriteGuard<Msg>,
        validate: bool,
        valid: &Validator,
        message_key: &String,
        message_obj: Variant,
    ) {
        let value: Value = serde_json::from_str(&message_obj.to_string().unwrap()).unwrap();
        if self.config.action.is_empty() || self.config.action == "str" {
            if validate {
                match valid.validate(&value) {
                    Ok(_) => {
                        msg_guard.set(
                            message_key.to_string(),
                            Variant::String(serde_json::to_string(&value).unwrap_or_default()),
                        );
                        msg_guard.remove("schema");
                    }
                    Err(err) => {
                        msg_guard.set(
                            "schemaError".to_string(),
                            Variant::from(err.into_iter().map(|e| e.to_string()).join(",")),
                        )
                        // todo done(`${RED._("json.errors.schema-error")}: ${ajv.errorsText(this.compiledSchema.errors)}`);
                    }
                };
            } else {
                msg_guard
                    .set(message_key.to_string(), Variant::String(serde_json::to_string(&value).unwrap_or_default()));
            }
        } else if validate {
                match valid.validate(&value) {
                    Ok(_) => {
                        msg_guard.remove("schema");
                    }
                    Err(err) => {
                        msg_guard.set(
                            "schemaError".to_string(),
                            Variant::from(err.into_iter().map(|e| e.to_string()).join(",")),
                        )
                        // todo done(`${RED._("json.errors.schema-error")}: ${ajv.errorsText(this.compiledSchema.errors)}`);
                    }
                };
            }
    }
}

#[async_trait]
impl FlowNodeBehavior for JsonNode {
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
