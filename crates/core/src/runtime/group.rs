use std::{collections::HashMap, sync::Weak};

use super::{
    flow::Flow,
    model::{ElementId, Variant},
};
use crate::red::json::RedGroupConfig;

#[derive(Debug)]
pub enum GroupParent {
    Flow(Weak<Flow>),
    Group(Weak<Group>),
}

#[derive(Debug)]
pub struct Group {
    pub id: ElementId,
    pub name: String,
    pub flow: Weak<Flow>,
    pub parent: GroupParent,
    pub env: HashMap<String, Variant>,
}

impl Group {
    pub(crate) fn new_flow_group(config: &RedGroupConfig, flow: Weak<Flow>) -> crate::Result<Self> {
        let group = Group {
            id: config.id,
            name: config.name.clone(),
            flow: flow.clone(),
            parent: GroupParent::Flow(flow.clone()),
            env: HashMap::new(),
        };
        Ok(group)
    }

    pub(crate) fn new_subgroup(
        config: &RedGroupConfig,
        flow: Weak<Flow>,
        parent: Weak<Group>,
    ) -> crate::Result<Self> {
        let group = Group {
            id: config.id,
            name: config.name.clone(),
            flow: flow.clone(),
            parent: GroupParent::Group(parent.clone()),
            env: HashMap::new(),
        };
        Ok(group)
    }

    pub fn get_setting(&self, key: &str) -> Variant {
        if key == "NR_GROUP_NAME" {
            return Variant::String(self.name.clone());
        } else if key == "NR_GROUP_ID" {
            return Variant::String(self.id.to_string());
        }
        /*
        else if !key.starts_with("$parent.") {
            if (this._env.hasOwnProperty(key)) {
                return (this._env[key] && Object.hasOwn(this._env[key], 'value') && this._env[key].__clone__) ? clone(this._env[key].value) : this._env[key]
            }
        } else {
            key = key.substring(8);
        }
        */
        match &self.parent {
            GroupParent::Flow(parent_flow) => parent_flow
                .upgrade()
                .map(|x| x.get_setting(key))
                .unwrap_or(Variant::Null),

            GroupParent::Group(parent_group) => parent_group
                .upgrade()
                .map(|x| x.get_setting(key))
                .unwrap_or(Variant::Null),
        }
    }
}
