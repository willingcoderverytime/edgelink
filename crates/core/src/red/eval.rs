use std::sync::Weak;

use serde_json::Value as JsonValue;

use crate::{
    runtime::{
        flow::Flow,
        model::{Msg, Variant},
        nodes::FlowNodeBehavior,
    },
    utils, EdgelinkError,
};

use super::json::RedPropertyType;

pub struct ParsedContextStoreProperty<'a> {
    pub store: &'a str,
    pub key: &'a str,
}

fn context_store_parser(
    input: &str,
) -> nom::IResult<&str, ParsedContextStoreProperty<'_>, nom::error::VerboseError<&str>> {
    use crate::utils::parser::*;
    use nom::{
        bytes::complete::tag,
        character::complete::{char, multispace0},
        sequence::{delimited, preceded},
    };

    let (input, _) = tag("#:")(input)?;

    let (input, store) = delimited(
        char('('),
        delimited(multispace0, identifier, multispace0),
        char(')'),
    )(input)?;

    let (input, key) = preceded(tag("::"), identifier)(input)?;

    Ok((input, ParsedContextStoreProperty { store, key }))
}

/// Parses a context property string, as generated by the TypedInput, to extract
/// the store name if present.
///
/// # Examples
/// For example, `#:(file)::foo` results in ` ParsedContextStoreProperty{ store: "file", key: "foo" }`.
/// ```
/// use edgelink::runtime::red::eval::parse_context_store;
///
/// let res = parse_context_store("#:(file)::foo").unwrap();
/// assert_eq!("file", res.store);
/// assert_eq!("foo", res.key);
/// ```
/// @param  {String} key - the context property string to parse
/// @return {Object} The parsed property
/// @memberof @node-red/util_util
pub fn parse_context_store(key: &str) -> crate::Result<ParsedContextStoreProperty<'_>> {
    match context_store_parser(key) {
        Ok(res) => Ok(res.1),
        Err(e) => Err(EdgelinkError::BadArguments(
            format!("Can not parse the key: '{0}'", e).to_owned(),
        )
        .into()),
    }
}

/**
 * Get value of environment variable.
 * @param {Node} node - accessing node
 * @param {String} name - name of variable
 * @return {String} value of env var
 */
fn get_setting(
    name: &str,
    node: Option<&dyn FlowNodeBehavior>,
    flow: Option<&Weak<Flow>>,
) -> Variant {
    if let Some(node) = node {
        match name {
            "NR_NODE_NAME" => return Variant::String(node.name().into()),
            "NR_NODE_ID" => return Variant::String(node.id().to_string()),
            "NR_NODE_PATH" => return Variant::Null,
            &_ => (),
        };
    }

    if let Some(flow_ref) = flow.or_else(|| node.map(|n| &n.state().flow)) {
        if let Some(node) = node {
            if let Some(group) = node.group().upgrade() {
                return group.get_setting(name);
            }
        }

        let flow = flow_ref.upgrade().expect("No way this happened");
        return flow.get_setting(name);
    }

    // TODO FIXME
    // We should use the snapshot in the FlowEngine
    std::env::var(name)
        .map(Variant::String)
        .unwrap_or(Variant::Null)
}

/**
 * Checks if a String contains any Environment Variable specifiers and returns
 * it with their values substituted in place.
 *
 * For example, if the env var `WHO` is set to `Joe`, the string `Hello ${WHO}!`
 * will return `Hello Joe!`.
 * @param  {String} value - the string to parse
 * @param  {Node} node - the node evaluating the property
 * @return {String} The parsed string
 */
fn evaluate_env_property(value: &str, node: Option<&dyn FlowNodeBehavior>) -> Variant {
    let flow = node.map(|n| &n.state().flow);
    if value.starts_with("${") && value.ends_with("}") {
        // ${ENV_VAR}
        let name = &value[2..(value.len() - 1)];
        get_setting(name, node, flow)
    } else if !value.contains("${") {
        // ENV_VAR
        get_setting(value, node, flow)
    } else {
        // FOO${ENV_VAR}BAR
        Variant::String(super::env::replace_vars(
            value,
            |env_name| match get_setting(env_name, node, flow) {
                Variant::String(v) => v,
                _ => "".to_string(),
            },
        ))
    }
}

/**
 * Evaluates a property value according to its type.
 *
 * @param  {String}   value    - the raw value
 * @param  {String}   _type     - the type of the value
 * @param  {Node}     node     - the node evaluating the property
 * @param  {Object}   msg      - the message object to evaluate against
 * @param  {Function} callback - (optional) called when the property is evaluated
 * @return {any} The evaluted property, if no `callback` is provided
 */
pub fn evaluate_node_property(
    value: &str,
    _type: &RedPropertyType,
    node: Option<&dyn FlowNodeBehavior>,
    _msg: Option<&Msg>,
) -> crate::Result<Variant> {
    let evaluated = match _type {
        RedPropertyType::Str => Variant::String(value.to_string()),

        RedPropertyType::Num => {
            let fv = value.parse::<f64>().unwrap_or(0.0);
            Variant::Rational(fv)
        }

        RedPropertyType::Json => {
            let root_jv: JsonValue = serde_json::from_str(value)?;
            Variant::from(root_jv)
        }

        RedPropertyType::Re => todo!(), // TODO FIXME

        RedPropertyType::Date => match value {
            "" => Variant::Rational(utils::time::unix_now() as f64),
            "object" => todo!(),
            "iso" => Variant::String(utils::time::iso_now()),
            _ => Variant::String(utils::time::millis_now()),
        },

        RedPropertyType::Bin => {
            let jv: JsonValue = serde_json::from_str(value)?;
            Variant::bytes_from_json_value(&jv)?
        }

        RedPropertyType::Msg => {
            if let Some(msg) = _msg {
                msg.get_trimmed_nav_property(value)
                    .unwrap_or(&Variant::Null)
                    .clone()
            } else {
                Variant::Null
            }
        }

        RedPropertyType::Flow | RedPropertyType::Global => todo!(),

        RedPropertyType::Bool => Variant::Bool(value.parse::<bool>().unwrap_or(false)),

        RedPropertyType::Jsonata => todo!(),

        RedPropertyType::Env => evaluate_env_property(value, node),
    };

    Ok(evaluated)
}
