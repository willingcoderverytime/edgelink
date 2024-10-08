use crate::runtime::model::*;
use serde;

pub struct RedTypeValue<'a> {
    pub red_type: &'a str,
    pub id: Option<ElementId>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, serde::Deserialize, PartialOrd)]
pub enum RedPropertyType {
    #[serde(rename = "str")]
    #[default]
    Str,

    #[serde(rename = "num")]
    Num,

    #[serde(rename = "json")]
    Json,

    #[serde(rename = "re")]
    Re,

    #[serde(rename = "date")]
    Date,

    #[serde(rename = "bin")]
    Bin,

    #[serde(rename = "msg")]
    Msg,

    #[serde(rename = "flow")]
    Flow,

    #[serde(rename = "global")]
    Global,

    #[serde(rename = "bool")]
    Bool,

    #[serde(rename = "jsonata")]
    Jsonata,

    #[serde(rename = "env")]
    Env,
}

impl RedPropertyType {
    pub fn is_constant(&self) -> bool {
        matches!(
            self,
            RedPropertyType::Str
                | RedPropertyType::Num
                | RedPropertyType::Json
                | RedPropertyType::Bin
                | RedPropertyType::Bool
        )
    }
}
