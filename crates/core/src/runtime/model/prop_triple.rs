use crate::runtime::model::*;
use serde::{self, Deserialize};

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

#[derive(Debug, Clone, Copy)]
pub enum RedPropertyTypeCategory {
    Propex = 0,
    Literal,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedPropertyTriple {
    pub p: String,

    #[serde(default)]
    pub vt: RedPropertyType,

    #[serde(default)]
    pub v: String,
}

impl RedPropertyType {
    pub fn category(&self) -> RedPropertyTypeCategory {
        match self {
            RedPropertyType::Msg | RedPropertyType::Flow | RedPropertyType::Global => {
                RedPropertyTypeCategory::Propex
            }
            _ => RedPropertyTypeCategory::Literal,
        }
    }
}