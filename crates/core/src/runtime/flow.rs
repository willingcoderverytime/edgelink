use std::sync::{Arc, Weak};

use dashmap::DashMap;
use itertools::Itertools;
use serde::Deserialize;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use super::context::Context;
use super::group::Group;
use crate::runtime::engine::FlowEngine;
use crate::runtime::env::*;
use crate::runtime::model::json::*;
use crate::runtime::model::*;
use crate::runtime::nodes::*;
use crate::runtime::registry::Registry;
use crate::EdgelinkError;

const NODE_MSG_CHANNEL_CAPACITY: usize = 32;

pub type FlowNodeTask = tokio::task::JoinHandle<()>;

#[derive(Debug, Clone, Deserialize)]
pub struct FlowArgs {
    node_msg_queue_capacity: usize,
}

impl FlowArgs {
    pub fn load(cfg: Option<&config::Config>) -> crate::Result<Self> {
        match cfg {
            Some(cfg) => {
                let res = cfg.get::<Self>("runtime.flow")?;
                Ok(res)
            }
            _ => Ok(FlowArgs::default()),
        }
    }
}

impl Default for FlowArgs {
    fn default() -> Self {
        Self { node_msg_queue_capacity: 16 }
    }
}

#[derive(Debug)]
pub struct SubflowOutputPort {
    index: usize,
    owner: Weak<Flow>,
    msg_tx: MsgSender,
    msg_rx: MsgReceiverHolder,
}

#[derive(Debug)]
pub struct SubflowState {
    instance_node: Option<Arc<dyn FlowNodeBehavior>>,
    in_nodes: Vec<Arc<dyn FlowNodeBehavior>>,
    tx_tasks: JoinSet<()>,
    tx_ports: Vec<Arc<SubflowOutputPort>>,
}

#[derive(Debug)]
pub(crate) struct FlowState {
    pub(crate) groups: DashMap<ElementId, Arc<Group>>,
    pub(crate) nodes: DashMap<ElementId, Arc<dyn FlowNodeBehavior>>,
    pub(crate) complete_nodes_map: DashMap<ElementId, Vec<Arc<dyn FlowNodeBehavior>>>,
    pub(crate) catch_nodes: DashMap<ElementId, Arc<dyn FlowNodeBehavior>>,
    pub(crate) _context: RwLock<Variant>,
    pub(crate) node_tasks: Mutex<JoinSet<()>>,
}

#[derive(Debug, Clone)]
pub enum FlowKind {
    GlobalFlow,
    Subflow,
}

#[derive(Debug)]
pub struct Flow {
    pub id: ElementId,
    pub parent: Option<Weak<Self>>,
    pub label: String,
    pub disabled: bool,
    pub args: FlowArgs,
    pub ordering: usize,
    pub type_str: &'static str,

    pub engine: Weak<FlowEngine>,

    pub stop_token: CancellationToken,

    state: FlowState,
    subflow_state: Option<std::sync::RwLock<SubflowState>>,
    envs: Arc<EnvStore>,
    context: Arc<Context>,
}

impl FlowsElement for Flow {
    fn id(&self) -> ElementId {
        self.id
    }

    fn name(&self) -> &str {
        &self.label
    }

    fn type_str(&self) -> &'static str {
        self.type_str
    }

    fn ordering(&self) -> usize {
        self.ordering
    }

    fn parent_element(&self) -> Option<Arc<dyn FlowsElement>> {
        self.parent.as_ref().and_then(|weak_parent| weak_parent.upgrade()).map(|arc| arc as Arc<dyn FlowsElement>)
    }

    fn context(&self) -> Arc<Context> {
        self.context.clone()
    }

    fn as_any(&self) -> &dyn ::std::any::Any {
        self
    }
}

impl SubflowOutputPort {
    async fn tx_task(&self, stop_token: CancellationToken) {
        while !stop_token.is_cancelled() {
            match self.msg_rx.recv_msg(stop_token.clone()).await {
                Ok(msg) => {
                    // Find out the subflow:xxx node
                    let instance_node = {
                        let flow =
                            self.owner.upgrade().expect("The owner of this sub-flow node has been released already!!!");

                        let subflow_state = flow.subflow_state.as_ref().expect("Subflow must have a subflow_state!");

                        let subflow_state_guard =
                            subflow_state.read().expect("Cannot acquire the lock of field `subflow_state`!!!");

                        subflow_state_guard.instance_node.clone()
                    };

                    if let Some(instance_node) = instance_node {
                        let instance_node = instance_node.clone();
                        let envelope = Envelope { port: self.index, msg };
                        if let Err(e) = instance_node.fan_out_one(&envelope, stop_token.clone()).await {
                            log::warn!("Failed to fan-out message: {:?}", e);
                        }
                    } else {
                        log::warn!("The sub-flow does not have a subflow node");
                    }
                }

                Err(e) => {
                    log::error!("Failed to receive msg in subflow_tx_task: {:?}", e);
                }
            }
        }
    }
}

impl SubflowState {
    fn populate_in_nodes(&mut self, flow_state: &FlowState, flow_config: &RedFlowConfig) -> crate::Result<()> {
        // this is a subflow with in ports
        for wire_obj in flow_config.in_ports.iter().flat_map(|x| x.wires.iter()) {
            if let Some(node) = flow_state.nodes.get(&wire_obj.id) {
                self.in_nodes.push(node.clone());
            } else {
                log::warn!("Can not found node(id='{}')", wire_obj.id);
            }
        }
        Ok(())
    }

    fn start_tx_tasks(&mut self, stop_token: CancellationToken) -> crate::Result<()> {
        for tx_port in self.tx_ports.iter() {
            let child_stop_token = stop_token.clone();
            let port_cloned = tx_port.clone();
            self.tx_tasks.spawn(async move {
                port_cloned.tx_task(child_stop_token.clone()).await;
            });
        }
        Ok(())
    }

    /*
    async fn stop_tx_tasks(&mut self) -> crate::Result<()> {
        while self.tx_tasks.join_next().await.is_some() {
            //
        }
        Ok(())
    }
    */
}

impl FlowState {
    async fn start_nodes(&self, stop_token: CancellationToken) -> crate::Result<()> {
        let nodes_ordering =
            self.nodes.iter().sorted_by(|a, b| a.ordering().cmp(&b.ordering())).map(|x| x.value().clone());

        for node in nodes_ordering.into_iter() {
            if node.get_node().disabled {
                log::warn!("------ Skipping disabled node {}.", node);
                continue;
            }

            // Start the async-task of each flow node
            log::info!("------ Starting node {}...", node,);

            let child_stop_token = stop_token.clone();
            node.on_starting().await;
            self.node_tasks.lock().await.spawn(async move {
                let node_ref = node.as_ref();
                let _ = node.clone().run(child_stop_token.child_token()).await;
                log::info!("------ {} has been stopped.", node_ref,);
            });
        }

        Ok(())
    }

    pub async fn stop_nodes(&self) -> crate::Result<()> {
        while self.node_tasks.lock().await.join_next().await.is_some() {
            //
        }
        Ok(())
    }
}

impl Flow {
    pub(crate) fn new(
        engine: Arc<FlowEngine>,
        flow_config: &RedFlowConfig,
        reg: Arc<dyn Registry>,
        options: Option<&config::Config>,
    ) -> crate::Result<Arc<Self>> {
        let flow_kind = match flow_config.type_name.as_str() {
            "tab" => FlowKind::GlobalFlow,
            "subflow" => FlowKind::Subflow,
            _ => return Err(EdgelinkError::BadFlowsJson("Unsupported flow type".to_string()).into()),
        };

        let subflow_instance = flow_config.subflow_node_id.and_then(|x| engine.find_flow_node_by_id(&x));

        let mut envs_builder = EnvStoreBuilder::default();
        envs_builder = match flow_kind {
            FlowKind::GlobalFlow => envs_builder.with_parent(&engine.get_envs()),
            FlowKind::Subflow => {
                if let Some(ref instance) = subflow_instance {
                    envs_builder.with_parent(&instance.get_envs())
                } else {
                    log::warn!("Cannot found the instance node of the subflow: id='{}'", flow_config.id);
                    envs_builder.with_parent(&engine.get_envs())
                }
            }
        };
        if let Some(env_json) = flow_config.json.get("env") {
            envs_builder = envs_builder.load_json(env_json);
        }
        envs_builder = match flow_kind {
            FlowKind::GlobalFlow => envs_builder.extends([
                ("NR_FLOW_ID".into(), flow_config.id.to_string().into()),
                ("NR_FLOW_NAME".into(), flow_config.label.clone().into()),
            ]),
            FlowKind::Subflow => {
                if subflow_instance.is_none() {
                    return Err(
                        EdgelinkError::BadFlowsJson("The ID of Sub-flow instance node is None".to_string()).into()
                    );
                }
                let subflow_instance = subflow_instance.as_ref().unwrap().clone();
                envs_builder.extends([
                    ("NR_SUBFLOW_ID".into(), subflow_instance.id().to_string().into()),
                    ("NR_SUBFLOW_NAME".into(), subflow_instance.name().into()),
                    (
                        "NR_SUBFLOW_PATH".into(),
                        format!("{}/{}", subflow_instance.get_flow().upgrade().unwrap().id(), subflow_instance.id())
                            .into(),
                    ),
                ])
            }
        };
        let envs = envs_builder.build();

        let context = engine.get_context_manager().new_context(Some(&engine.get_context()), flow_config.id.to_string());

        let flow: Arc<Flow> = Arc::new(Flow {
            id: flow_config.id,
            parent: None, //TODO FIXME
            engine: Arc::downgrade(&engine),
            label: flow_config.label.clone(),
            disabled: flow_config.disabled,
            ordering: flow_config.ordering,
            args: FlowArgs::load(options)?,
            type_str: match flow_kind {
                FlowKind::GlobalFlow => "flow",
                FlowKind::Subflow => "subflow",
            },
            state: FlowState {
                groups: DashMap::new(),
                nodes: DashMap::new(),
                complete_nodes_map: DashMap::new(),
                catch_nodes: DashMap::new(),
                _context: RwLock::new(Variant::empty_object()),
                node_tasks: Mutex::new(JoinSet::new()),
            },

            subflow_state: match flow_kind {
                FlowKind::Subflow => Some(std::sync::RwLock::new(SubflowState {
                    instance_node: None,
                    in_nodes: Vec::new(),
                    tx_tasks: JoinSet::new(),
                    tx_ports: Vec::new(),
                })),
                FlowKind::GlobalFlow => None,
            },
            envs,
            context,
            stop_token: CancellationToken::new(),
            // groups: HashMap::new(), //   flow_config.groups.iter().map(|g| Group::new_flow_group(config, flow))
        });

        // Add empty subflow forward ports
        if let Some(ref subflow_state) = flow.subflow_state {
            let mut subflow_state = subflow_state.write().unwrap();
            subflow_state.instance_node = subflow_instance;

            for (index, _) in flow_config.out_ports.iter().enumerate() {
                let (msg_root_tx, msg_rx) = tokio::sync::mpsc::channel(flow.args.node_msg_queue_capacity);

                subflow_state.tx_ports.push(Arc::new(SubflowOutputPort {
                    index,
                    owner: Arc::downgrade(&flow),
                    msg_tx: msg_root_tx.clone(),
                    msg_rx: MsgReceiverHolder::new(msg_rx),
                }));
            }
        }

        {
            let flow = flow.clone();
            flow.clone().populate_groups(flow_config)?;

            flow.clone().populate_nodes(flow_config, reg.as_ref(), engine.as_ref())?;
        }

        if let Some(subflow_state) = &flow.subflow_state {
            let mut subflow_state = subflow_state.write().unwrap();

            subflow_state.populate_in_nodes(&flow.state, flow_config)?;
        }

        Ok(flow)
    }

    fn populate_groups(self: Arc<Self>, flow_config: &RedFlowConfig) -> crate::Result<()> {
        if !self.state.groups.is_empty() {
            self.state.groups.clear();
        }
        // Adding root groups
        let root_group_configs = flow_config.groups.iter().filter(|gc| gc.z == self.id());
        for gc in root_group_configs {
            let group = match &gc.g {
                // Subgroup
                Some(parent_id) => Group::new_subgroup(
                    gc,
                    &self,
                    &self.state.groups.get(parent_id).unwrap(), //FIXME
                )?,

                // Root group
                None => Group::new_flow_group(gc, &self)?,
            };
            self.state.groups.insert(group.id, Arc::new(group));
        }
        Ok(())
    }

    fn populate_nodes(
        self: Arc<Self>,
        flow_config: &RedFlowConfig,
        reg: &dyn Registry,
        engine: &FlowEngine,
    ) -> crate::Result<()> {
        // Adding nodes
        for node_config in flow_config.nodes.iter() {
            let meta_node = if let Some(meta_node) = reg.get(&node_config.type_name) {
                meta_node
            } else if node_config.type_name.starts_with("subflow:") {
                reg.get("subflow").expect("The `subflow` node must be existed")
            } else {
                log::warn!(
                    "Unknown flow node type: (type='{}', id='{}', name='{}')",
                    node_config.type_name,
                    node_config.id,
                    node_config.name
                );
                reg.get("unknown.flow").expect("The `unknown.flow` node must be existed")
            };

            let node = match meta_node.factory {
                NodeFactory::Flow(factory) => {
                    let mut node_state =
                        self.clone().new_flow_node_state(meta_node, &self.state, node_config, engine).map_err(|e| {
                            log::error!("Failed to create flow node(id='{}'): {:?}", node_config.id, e);
                            e
                        })?;

                    // Redirect all the output node wires in the subflow to the output port of the subflow.
                    if let Some(subflow_state) = &self.subflow_state {
                        let subflow_state = subflow_state.read().unwrap();
                        for (subflow_port_index, red_port) in flow_config.out_ports.iter().enumerate() {
                            let red_wires = red_port.wires.iter().filter(|x| x.id == node_state.id);
                            for red_wire in red_wires {
                                // Makre sure the target node has one port at least!
                                if node_state.ports.is_empty() {
                                    node_state.ports.push(Port::empty());
                                }
                                if let Some(node_port) = node_state.ports.get_mut(red_wire.port) {
                                    let subflow_tx_port = &subflow_state.tx_ports[subflow_port_index];
                                    let node_wire = PortWire { msg_sender: subflow_tx_port.msg_tx.clone() };
                                    node_port.wires.push(node_wire)
                                } else {
                                    return Err(EdgelinkError::BadFlowsJson(format!(
                                        "Invalid port '{}' for subflow: {:?}",
                                        red_wire.port, subflow_state
                                    ))
                                    .into());
                                }
                            }
                        }
                    }

                    match factory(&self, node_state, node_config) {
                        Ok(node) => {
                            log::debug!("------ The node {} has been built.", node);
                            node
                        }
                        Err(err) => {
                            log::error!("Failed to build node from {}: {}", node_config, err);
                            log::debug!("Node JSON:\n{}", node_config.json);
                            return Err(err);
                        }
                    }
                }
                NodeFactory::Global(_) => {
                    return Err(EdgelinkError::NotSupported(format!(
                        "Must be a flow node: Node(id={0}, type='{1}')",
                        flow_config.id, flow_config.type_name
                    ))
                    .into())
                }
            };

            let arc_node: Arc<dyn FlowNodeBehavior> = Arc::from(node);
            arc_node.on_loaded();
            self.state.nodes.insert(node_config.id, arc_node.clone());

            log::debug!("------ {} has been loaded!", arc_node);

            self.register_internal_node(arc_node, node_config)?;
        }
        Ok(())
    }

    fn register_internal_node(
        &self,
        node: Arc<dyn FlowNodeBehavior>,
        node_config: &RedFlowNodeConfig,
    ) -> crate::Result<()> {
        match node.get_node().type_str {
            "complete" => self.register_complete_node(node, node_config)?,
            "catch" => {
                self.state.catch_nodes.insert(node_config.id, node.clone());
            }
            // ignore normal nodes
            &_ => {}
        }
        Ok(())
    }

    fn register_complete_node(
        &self,
        node: Arc<dyn FlowNodeBehavior>,
        node_config: &RedFlowNodeConfig,
    ) -> crate::Result<()> {
        if let Some(scope) = node_config.json.get("scope").and_then(|x| x.as_array()) {
            for src_id in scope {
                if let Some(src_id) = helpers::parse_red_id_value(src_id) {
                    if let Some(ref mut complete_nodes) = self.state.complete_nodes_map.get_mut(&src_id) {
                        if !complete_nodes.iter().any(|x| x.id() == node.id()) {
                            complete_nodes.push(node.clone());
                        } else {
                            return Err(EdgelinkError::InvalidOperation(format!(
                                "The connection of the {} to the `complete` node already existed!",
                                node
                            ))
                            .into());
                        }
                    } else {
                        self.state.complete_nodes_map.insert(src_id, Vec::from([node.clone()]));
                    }
                }
            }
            Ok(())
        } else {
            Err(EdgelinkError::BadFlowsJson(format!("CompleteNode has no 'scope' property: {}", node)).into())
        }
    }

    pub fn id(&self) -> ElementId {
        self.id
    }

    pub fn parent(&self) -> &Option<Weak<Flow>> {
        &self.parent
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn disabled(&self) -> bool {
        self.disabled
    }

    pub fn is_subflow(&self) -> bool {
        self.subflow_state.is_some()
    }

    pub fn get_all_flow_nodes(&self) -> Vec<Arc<dyn FlowNodeBehavior>> {
        self.state.nodes.iter().map(|x| x.value().clone()).collect()
    }

    pub fn get_node_by_id(&self, id: &ElementId) -> Option<Arc<dyn FlowNodeBehavior>> {
        self.state.nodes.get(id).map(|x| x.value().clone())
    }

    pub fn get_node_by_name(&self, name: &str) -> crate::Result<Option<Arc<dyn FlowNodeBehavior>>> {
        let iter = self.state.nodes.iter().filter(|val| val.name() == name);
        let nfound = iter.clone().count();
        if nfound == 1 {
            Ok(iter.clone().next().map(|x| x.clone()))
        } else if nfound == 0 {
            Ok(None)
        } else {
            Err(EdgelinkError::InvalidOperation(format!("There are multiple node with name '{}'", name)).into())
        }
    }

    pub fn get_envs(&self) -> Arc<EnvStore> {
        self.envs.clone()
    }

    pub fn get_env(&self, key: &str) -> Option<Variant> {
        self.envs.evalute_env(key)
    }

    /*
    pub fn eval_envs(
        &self,
        env_entries: &[RedEnvEntry],
    ) -> crate::Result<HashMap<String, Variant>> {
        let mut evaluated_entries: HashMap<String, Variant> = HashMap::new();

        // preprocessing
        for e in env_entries.iter().filter(|&x| x.name != "env") {
            let parsed_value = eval::evaluate_node_property(
                &e.value,
                e.type_,
                None,
                None,
            )?;
            evaluated_entries.insert(e.name.clone(), parsed_value);
        }

        // TODO JSONATA

        for e in env_entries.iter().filter(|&x| x.name == "env") {
            let env_value_text = if e.name == e.value {
                format!("$parent.${}", e.name)
            } else {
                e.value.clone()
            };
            let mut parsed_value =
                if let Some(existed_value) = evaluated_entries.get(&env_value_text) {
                    existed_value.clone()
                } else {
                    eval::evaluate_node_property(
                        &env_value_text,
                        e.type_,
                        None,
                        None,
                    )?
                };
            parsed_value = if let Some(parsed_obj) = parsed_value.as_object() {
                if !parsed_obj.contains_key("__clone__") {
                    Variant::from([("value", parsed_value), ("__clone__", Variant::Bool(true))])
                } else {
                    parsed_value
                }
            } else {
                parsed_value
            };
            evaluated_entries.insert(e.name.clone(), parsed_value);
        }

        Ok(evaluated_entries)
    }
    */

    pub async fn start(&self) -> crate::Result<()> {
        // let mut state = self.shared.state.write().await;

        if self.is_subflow() {
            log::info!("---- Starting Subflow (id={})...", self.id);
        } else {
            log::info!("---- Starting Flow (id={})...", self.id);
        }

        if let Some(subflow_state) = &self.subflow_state {
            log::info!("------ Starting the forward tasks of the subflow...");
            let mut subflow_state = subflow_state.write().unwrap();
            subflow_state.start_tx_tasks(self.stop_token.clone())?;
        }

        {
            self.state.start_nodes(self.stop_token.clone()).await?;
        }

        Ok(())
    }

    pub async fn stop(&self) -> crate::Result<()> {
        if self.is_subflow() {
            log::info!("---- Stopping Subflow (id={})...", self.id);
        } else {
            log::info!("---- Stopping Flow (id={})...", self.id);
        }

        self.stop_token.cancel();

        // Wait all subflow senders to stop
        /*
        if let Some(ss) = &self.subflow_state {
            let mut ss = ss.write().unwrap();
            ss.stop_tx_tasks().await?;
        }
        */

        // Wait all nodes
        {
            self.state.stop_nodes().await?;
        }
        log::info!("---- All node in flow/subflow(id='{}') has been stopped.", self.id);

        Ok(())
    }

    pub async fn notify_node_uow_completed(&self, emitter_id: &ElementId, msg: &Msg, cancel: CancellationToken) {
        if let Some(complete_nodes) = self.state.complete_nodes_map.get(emitter_id) {
            for complete_node in complete_nodes.iter() {
                let to_send = Arc::new(RwLock::new(msg.clone()));
                match complete_node.inject_msg(to_send, cancel.child_token()).await {
                    Ok(()) => {}
                    Err(err) => {
                        log::warn!("Failed to inject msg in notify_node_completed(): {}", err.to_string());
                    }
                }
            }
        }
    }

    pub async fn inject_msg(&self, msg: Arc<RwLock<Msg>>, cancel: CancellationToken) -> crate::Result<()> {
        tokio::select! {
            result = self.inject_msg_internal(msg, cancel.clone()) => result,

            _ = cancel.cancelled() => {
                // The token was cancelled
                Err(EdgelinkError::TaskCancelled.into())
            }
        }
    }

    async fn inject_msg_internal(&self, msg: Arc<RwLock<Msg>>, cancel: CancellationToken) -> crate::Result<()> {
        if let Some(subflow_state) = &self.subflow_state {
            let in_nodes = {
                let subflow_state = subflow_state.read().unwrap();
                subflow_state.in_nodes.clone()
            };
            let mut msg_sent = false;
            for node in in_nodes {
                if !msg_sent {
                    node.inject_msg(msg.clone(), cancel.clone()).await?;
                } else {
                    let to_clone = msg.read().await;
                    node.inject_msg(Arc::new(RwLock::new(to_clone.clone())), cancel.clone()).await?;
                }
                msg_sent = true;
            }
            Ok(())
        } else {
            Err(EdgelinkError::InvalidOperation("This is not a subflow!".into()).into())
        }
    }

    fn new_flow_node_state(
        self: Arc<Self>,
        meta_node: &MetaNode,
        state: &FlowState,
        node_config: &RedFlowNodeConfig,
        engine: &FlowEngine,
    ) -> crate::Result<FlowNode> {
        let mut ports = Vec::new();
        let (tx_root, rx) = tokio::sync::mpsc::channel(NODE_MSG_CHANNEL_CAPACITY);
        // Convert the Node-RED wires elements to ours
        for red_port in node_config.wires.iter() {
            let mut wires = Vec::new();
            for nid in red_port.node_ids.iter() {
                // First we find the node in this flow
                let node_in_flow = state.nodes.get(nid).map(|x| x.value().clone());
                // Next we find the node in the entire engine, otherwise there is an error
                let node_in_engine = engine.find_flow_node_by_id(nid);
                let node_entry = node_in_flow.or(node_in_engine).ok_or(EdgelinkError::InvalidData(format!(
                    "Referenced node not found [this_node.id='{}' this_node.name='{}', referenced_node.id='{}']",
                    node_config.id, node_config.name, nid
                )))?;
                let tx = node_entry.get_node().msg_tx.to_owned();
                let pw = PortWire {
                    // target_node_id: *nid,
                    // target_node: Arc::downgrade(node_entry),
                    msg_sender: tx,
                };
                wires.push(pw);
            }
            let port = Port { wires };
            ports.push(port);
        }

        let group = match &node_config.g {
            Some(gid) => match state.groups.get(gid) {
                Some(g) => Some(g.value().clone()),
                None => {
                    return Err(EdgelinkError::InvalidData(format!(
                        "Can not found the group id in groups: id='{}'",
                        gid
                    ))
                    .into());
                }
            },
            None => None,
        };

        let mut envs_builder = EnvStoreBuilder::default();
        if let Some(ref g) = group {
            envs_builder = envs_builder.with_parent(&g.get_envs());
        } else {
            envs_builder = envs_builder.with_parent(&self.get_envs());
        }
        if let Some(env_json) = node_config.json.get("env") {
            envs_builder = envs_builder.load_json(env_json);
        }
        let envs = envs_builder
            .extends([
                ("NR_NODE_ID".into(), Variant::String(node_config.id.to_string())),
                ("NR_NODE_NAME".into(), Variant::String(node_config.name.clone())),
                ("NR_NODE_PATH".into(), Variant::String(format!("{}/{}", self.id, node_config.id))),
            ])
            .build();
        let context = engine.get_context_manager().new_context(Some(&self.context), self.id.to_string());

        Ok(FlowNode {
            id: node_config.id,
            name: node_config.name.clone(),
            type_str: meta_node.type_,
            ordering: node_config.ordering,
            disabled: node_config.disabled,
            flow: Arc::downgrade(&self),
            msg_tx: tx_root,
            msg_rx: MsgReceiverHolder::new(rx),
            ports,
            group: group.map(|g| Arc::downgrade(&g)),
            envs,
            context,
            on_received: MsgEventSender::new(1),
            on_completed: MsgEventSender::new(1),
            on_error: MsgEventSender::new(1),
        })
    }
}
