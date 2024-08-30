use rquickjs::{self as js, FromIteratorJs, IntoAtom, IntoJs};
use std::borrow::BorrowMut;
use std::collections::BTreeMap;
use thiserror::Error;

use crate::runtime::model::propex;
use crate::EdgeLinkError;

use super::propex::PropexSegment;

#[derive(Error, Clone, Debug, PartialEq, PartialOrd)]
pub enum VariantError {
    #[error("Wrong type")]
    WrongType,

    #[error("Out of range error")]
    OutOfRange,
}

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Variant {
    /// Null
    Null,

    /// A float
    Rational(f64),

    /// A 32-bit integer
    Integer(i32),

    /// A string
    String(String),

    /// A boolean
    Bool(bool),

    /// Bytes
    Bytes(Vec<u8>),

    /// An array
    Array(Vec<Variant>),

    /// A object
    Object(BTreeMap<String, Variant>),
}

impl Variant {
    pub fn new_empty_object() -> Variant {
        Variant::Object(BTreeMap::new())
    }

    pub fn new_empty_array() -> Variant {
        Variant::Array(Vec::<Variant>::new())
    }

    pub fn is_bytes(&self) -> bool {
        matches!(self, Variant::Bytes(..))
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Variant::Bytes(ref bytes) => Some(bytes),
            Variant::String(ref s) => Some(s.as_bytes()),
            _ => None,
        }
    }

    pub fn to_bytes(&self) -> Option<Vec<u8>> {
        match self {
            Variant::Bytes(ref bytes) => Some(bytes.clone()),
            Variant::String(ref s) => Some(s.bytes().collect()),
            Variant::Rational(f) => Some(f.to_string().bytes().collect()),
            Variant::Integer(i) => Some(i.to_string().bytes().collect()),
            _ => None,
        }
    }

    pub fn as_bytes_mut(&mut self) -> Option<&mut Vec<u8>> {
        match self {
            Variant::Bytes(ref mut bytes) => Some(bytes),
            _ => None,
        }
    }

    pub fn into_bytes(self) -> Result<Vec<u8>, Self> {
        match self {
            Variant::Bytes(vec) => Ok(vec),
            other => Err(other),
        }
    }

    pub fn as_u8(&self) -> Result<u8, VariantError> {
        match self {
            Variant::Rational(fvalue) => Ok(fvalue.round() as u8), //FIXME
            _ => Err(VariantError::WrongType),
        }
    }

    pub fn is_number(&self) -> bool {
        matches!(self, Variant::Rational(..) | Variant::Integer(..))
    }

    pub fn as_number(&self) -> Option<f64> {
        match *self {
            Variant::Rational(f) => Some(f),
            Variant::Integer(f) => Some(f as f64),
            _ => None,
        }
    }

    pub fn into_number(self) -> Result<f64, Self> {
        match self {
            Variant::Rational(f) => Ok(f),
            Variant::Integer(f) => Ok(f as f64),
            other => Err(other),
        }
    }

    pub fn is_integer(&self) -> bool {
        matches!(self, Variant::Integer(..))
    }

    pub fn as_integer(&self) -> Option<i32> {
        match *self {
            Variant::Integer(f) => Some(f),
            _ => None,
        }
    }

    pub fn into_integer(self) -> Result<i32, Self> {
        match self {
            Variant::Integer(v) => Ok(v),
            other => Err(other),
        }
    }

    pub fn is_string(&self) -> bool {
        matches!(self, Variant::String(..))
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            Variant::String(ref s) => Some(s),
            _ => None,
        }
    }

    pub fn as_string_mut(&mut self) -> Option<&mut String> {
        match self {
            Variant::String(ref mut s) => Some(s),
            _ => None,
        }
    }

    pub fn into_string(self) -> Result<String, Self> {
        match self {
            Variant::String(s) => Ok(s),
            Variant::Rational(f) => Ok(f.to_string()),
            Variant::Integer(i) => Ok(i.to_string()),
            other => Err(other),
        }
    }

    pub fn is_bool(&self) -> bool {
        matches!(self, Variant::Bool(..))
    }

    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            Variant::Bool(b) => Some(b),
            _ => None,
        }
    }

    pub fn into_bool(self) -> Result<bool, Self> {
        match self {
            Variant::Bool(b) => Ok(b),
            other => Err(other),
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Variant::Null)
    }

    pub fn is_array(&self) -> bool {
        matches!(self, Variant::Array(..))
    }

    pub fn as_array(&self) -> Option<&Vec<Variant>> {
        match self {
            Variant::Array(ref array) => Some(array),
            _ => None,
        }
    }

    pub fn as_array_mut(&mut self) -> Option<&mut Vec<Variant>> {
        match self {
            Variant::Array(ref mut list) => Some(list),
            _ => None,
        }
    }

    pub fn into_array(self) -> Result<Vec<Variant>, Self> {
        match self {
            Variant::Array(vec) => Ok(vec),
            other => Err(other),
        }
    }

    pub fn is_object(&self) -> bool {
        matches!(self, Variant::Object(..))
    }

    pub fn as_object(&self) -> Option<&BTreeMap<String, Variant>> {
        match self {
            Variant::Object(ref object) => Some(object),
            _ => None,
        }
    }

    pub fn as_object_mut(&mut self) -> Option<&mut BTreeMap<String, Variant>> {
        match self {
            Variant::Object(ref mut object) => Some(object),
            _ => None,
        }
    }

    pub fn into_object(self) -> Result<BTreeMap<String, Variant>, Self> {
        match self {
            Variant::Object(object) => Ok(object),
            other => Err(other),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Variant::Null => 0,
            Variant::Object(object) => object.len(),
            Variant::Array(array) => array.len(),
            Variant::Bytes(bytes) => bytes.len(),
            Variant::String(s) => s.len(),
            _ => 1,
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Variant::Null => true,
            Variant::Object(object) => object.is_empty(),
            Variant::Array(array) => array.is_empty(),
            Variant::Bytes(bytes) => bytes.is_empty(),
            Variant::String(s) => s.is_empty(),
            Variant::Rational(f) => f.is_nan(),
            _ => false,
        }
    }

    pub fn bytes_from_json_value(jv: &serde_json::Value) -> crate::Result<Variant> {
        match jv {
            serde_json::Value::Array(array) => {
                let mut bytes = Vec::with_capacity(array.len());
                for e in array.iter() {
                    if let Some(byte) = e.as_i64() {
                        if !(0..=0xFF).contains(&byte) {
                            return Err(EdgeLinkError::NotSupported(
                                "Invalid byte value".to_owned(),
                            )
                            .into());
                        }
                        bytes.push(byte as u8)
                    } else {
                        return Err(EdgeLinkError::NotSupported(
                            "Invalid byte JSON value type".to_owned(),
                        )
                        .into());
                    }
                }
                Ok(Variant::Bytes(bytes))
            }
            serde_json::Value::String(string) => Ok(Variant::from(string.as_bytes())),
            _ => Err(EdgeLinkError::NotSupported("Invalid byte JSON Value".to_owned()).into()),
        }
    }

    pub fn get_item_by_propex_segment(&self, pseg: &PropexSegment) -> Option<&Variant> {
        match pseg {
            PropexSegment::IntegerIndex(index) => self.get_array_item(*index),
            PropexSegment::StringIndex(prop) => self.get_object_property(prop),
        }
    }

    pub fn get_item_by_propex_segment_mut(&mut self, pseg: &PropexSegment) -> Option<&mut Variant> {
        match pseg {
            PropexSegment::IntegerIndex(index) => self.get_array_item_mut(*index),
            PropexSegment::StringIndex(prop) => self.get_object_property_mut(prop),
        }
    }

    pub fn get_item_by_propex_segments(&self, psegs: &[PropexSegment]) -> Option<&Variant> {
        match psegs.len() {
            0 => None,
            1 => self.get_item_by_propex_segment(&psegs[0]),
            _ => {
                let mut prev = self;
                for pseg in psegs {
                    if let Some(cur) = prev.get_item_by_propex_segment(pseg) {
                        prev = cur;
                    } else {
                        return None;
                    }
                }
                Some(prev)
            }
        }
    }

    pub fn get_item_by_propex_segments_mut(
        &mut self,
        psegs: &[PropexSegment],
    ) -> Option<&mut Variant> {
        match psegs.len() {
            0 => None,
            1 => self.get_item_by_propex_segment_mut(&psegs[0]),
            _ => {
                let mut prev = self;
                for pseg in psegs {
                    if let Some(cur) = prev.get_item_by_propex_segment_mut(pseg) {
                        prev = cur;
                    } else {
                        return None;
                    }
                }
                Some(prev)
            }
        }
    }

    pub fn get_object_property(&self, prop: &str) -> Option<&Variant> {
        match self {
            Variant::Object(obj) => obj.get(prop),
            _ => None,
        }
    }

    pub fn get_object_property_mut(&mut self, prop: &str) -> Option<&mut Variant> {
        match self {
            Variant::Object(obj) => obj.get_mut(prop),
            _ => None,
        }
    }

    pub fn get_array_item(&self, index: usize) -> Option<&Variant> {
        match self {
            Variant::Array(arr) => arr.get(index),
            _ => None,
        }
    }

    pub fn get_array_item_mut(&mut self, index: usize) -> Option<&mut Variant> {
        match self {
            Variant::Array(arr) => arr.get_mut(index),
            _ => None,
        }
    }

    pub fn get_object_nav_property(&self, expr: &str) -> Option<&Variant> {
        if let Ok(prop_segs) = propex::parse(expr) {
            self.get_item_by_propex_segments(&prop_segs)
        } else {
            None
        }
    }

    pub fn set_object_property(
        &mut self,
        prop: String,
        value: Variant,
    ) -> Result<(), VariantError> {
        match self {
            Variant::Object(ref mut this_obj) => {
                this_obj.insert(prop, value);
                Ok(())
            }
            _ => Err(VariantError::WrongType),
        }
    }

    pub fn set_array_item(&mut self, index: usize, value: Variant) -> Result<(), VariantError> {
        match self {
            Variant::Array(ref mut this_arr) => {
                if let Some(existed) = this_arr.get_mut(index) {
                    *existed = value;
                    Ok(())
                } else if index == this_arr.len() {
                    // insert to tail
                    this_arr.push(value);
                    Ok(())
                } else {
                    Err(VariantError::OutOfRange)
                }
            }
            Variant::Bytes(ref mut this_bytes) => {
                if let Some(existed) = this_bytes.get_mut(index) {
                    *existed = value.as_u8()?;
                    Ok(())
                } else if index == this_bytes.len() {
                    // insert to tail
                    this_bytes.push(value.as_u8()?);
                    Ok(())
                } else {
                    Err(VariantError::OutOfRange)
                }
            }
            _ => Err(VariantError::WrongType),
        }
    }

    pub fn set_property_by_propex_segment(
        &mut self,
        pseg: &PropexSegment,
        value: Variant,
    ) -> Result<(), VariantError> {
        match pseg {
            PropexSegment::IntegerIndex(index) => self.set_array_item(*index, value),
            PropexSegment::StringIndex(prop) => self.set_object_property(prop.to_string(), value),
        }
    }

    pub fn set_property_by_propex_segments(
        &mut self,
        psegs: &[PropexSegment],
        value: Variant,
        create_missing: bool,
    ) -> Result<(), VariantError> {
        if psegs.is_empty() {
            return Err(VariantError::OutOfRange);
        }

        if psegs.len() == 1 {
            self.set_property_by_propex_segment(&psegs[0], value)?;
            return Ok(());
        }

        for nlevel in 0..psegs.len() - 1 {
            let psegs_slice = &psegs[..=nlevel];
            let pseg = &psegs[nlevel];

            {
                let cur = self.get_item_by_propex_segments(psegs_slice);
                if cur.is_some() {
                    continue;
                }
            }

            if create_missing {
                if let Some(next_pseg) = psegs.get(nlevel + 1) {
                    let mut prev = self.borrow_mut();
                    if nlevel > 0 {
                        prev = self
                            .get_item_by_propex_segments_mut(&psegs[0..=nlevel - 1])
                            .ok_or(VariantError::OutOfRange)?;
                    }
                    match next_pseg {
                        PropexSegment::StringIndex(_) => {
                            prev.set_property_by_propex_segment(pseg, Variant::new_empty_object())?
                        }
                        PropexSegment::IntegerIndex(_) => {
                            prev.set_property_by_propex_segment(pseg, Variant::new_empty_array())?
                        }
                    }
                } else {
                    return Err(VariantError::OutOfRange);
                };
            } else {
                return Err(VariantError::OutOfRange);
            }
        }

        // 如果最后一个属性存在
        if let Some(terminal_obj) = self.get_item_by_propex_segments_mut(psegs) {
            *terminal_obj = value;
            Ok(())
        } else if let Some(parent_obj) =
            self.get_item_by_propex_segments_mut(&psegs[0..=psegs.len() - 2])
        {
            // 没有就获取上一个属性
            parent_obj
                .set_property_by_propex_segment(psegs.last().expect("We're so over"), value)?;
            Ok(())
        } else {
            Err(VariantError::OutOfRange)
        }
    }

    pub fn set_object_nav_property(
        &mut self,
        expr: &str,
        value: Variant,
        create_missing: bool,
    ) -> Result<(), VariantError> {
        if let Ok(prop_segs) = propex::parse(expr) {
            self.set_property_by_propex_segments(&prop_segs, value, create_missing)
        } else {
            Err(VariantError::OutOfRange)
        }
    }

    pub fn as_js_value<'js>(&self, ctx: &js::context::Ctx<'js>) -> crate::Result<js::Value<'js>> {
        match self {
            Variant::Array(_) => Ok(js::Value::from_array(self.as_js_array(ctx)?)),

            Variant::Bool(b) => Ok(js::Value::new_bool(ctx.clone(), *b)),

            Variant::Bytes(bytes) => {
                Ok(js::ArrayBuffer::new_copy(ctx.clone(), bytes)?.into_value())
            }

            Variant::Integer(i) => Ok(js::Value::new_int(ctx.clone(), *i)),

            Variant::Null => Ok(js::Value::new_null(ctx.clone())),

            Variant::Object(_) => Ok(js::Value::from_object(self.as_js_object(ctx)?)),

            Variant::String(s) => s.into_js(ctx).map_err(|e| e.into()),

            Variant::Rational(f) => f.into_js(ctx).map_err(|e| e.into()),
        }
    }

    pub fn as_js_array<'js>(&self, ctx: &js::Ctx<'js>) -> crate::Result<js::Array<'js>> {
        if let Variant::Array(items) = self {
            let iter = items.iter().map(|e| e.as_js_value(ctx).unwrap()); // TODO FIXME
            js::Array::from_iter_js(ctx, iter)
                .map_err(|e| EdgeLinkError::InvalidData(e.to_string()).into())
        } else {
            Err(crate::EdgeLinkError::InvalidOperation("Bad variant type".to_string()).into())
        }
    }

    pub fn as_js_object<'js>(&self, ctx: &js::context::Ctx<'js>) -> crate::Result<js::Object<'js>> {
        if let Variant::Object(map) = self {
            let obj = js::Object::new(ctx.clone())?;
            for (k, v) in map {
                let prop_name = k
                    .into_atom(ctx)
                    .map_err(|e| EdgeLinkError::InvalidData(e.to_string()))?;

                let prop_value = v
                    .as_js_value(ctx)
                    .map_err(|e| EdgeLinkError::InvalidData(e.to_string()))?;

                obj.set(prop_name, prop_value)
                    .map_err(|e| EdgeLinkError::InvalidData(e.to_string()))?;
            }
            Ok(obj)
        } else {
            Err(crate::EdgeLinkError::InvalidOperation("Bad variant type".to_string()).into())
        }
    }
} // struct Variant

impl From<&Variant> for String {
    #[inline]
    fn from(var: &Variant) -> Self {
        match var {
            Variant::Integer(i) => i.to_string(),
            Variant::Rational(f) => f.to_string(),
            Variant::String(s) => s.clone(),
            _ => "".to_string(),
        }
    }
}

macro_rules! implfrom {
    ($($v:ident($t:ty)),+ $(,)?) => {
        $(
            impl From<$t> for Variant {
                #[inline]
                fn from(value: $t) -> Self {
                    Self::$v(value.into())
                }
            }
        )+
    };
}

implfrom! {
    Integer(i32),
    Integer(u16),
    Integer(i16),
    Integer(u8),
    Integer(i8),

    Bytes(Vec<u8>),

    Rational(f32),
    Rational(f64),

    String(String),
    String(&str),

    Bool(bool),

    Array(&[Variant]),
    Array(Vec<Variant>),

    // Object(&[(String, Variant)]),
    // Object(&[(&str, Variant)]),
    Object(BTreeMap<String, Variant>),
    // Object(&BTreeMap<String, Variant>),
    // Object(BTreeMap<&str, Variant>),
}

impl From<char> for Variant {
    #[inline]
    fn from(value: char) -> Self {
        Variant::String(value.to_string())
    }
}

impl From<&[(String, Variant)]> for Variant {
    #[inline]
    fn from(value: &[(String, Variant)]) -> Self {
        let map: BTreeMap<String, Variant> =
            value.iter().map(|x| (x.0.clone(), x.1.clone())).collect();
        Variant::Object(map)
    }
}

impl<const N: usize> From<[(&str, Variant); N]> for Variant {
    #[inline]
    fn from(value: [(&str, Variant); N]) -> Self {
        let map: BTreeMap<String, Variant> = value
            .iter()
            .map(|x| (x.0.to_string(), x.1.clone()))
            .collect();
        Variant::Object(map)
    }
}

impl From<&[u8]> for Variant {
    fn from(array: &[u8]) -> Self {
        Variant::Bytes(array.to_vec())
    }
}

impl From<serde_json::Value> for Variant {
    fn from(jv: serde_json::Value) -> Self {
        match jv {
            serde_json::Value::Null => Variant::Null,
            serde_json::Value::Bool(boolean) => Variant::from(boolean),
            serde_json::Value::Number(number) => {
                //FIXME TODO
                Variant::Rational(number.as_f64().unwrap_or(f64::NAN))
            }
            serde_json::Value::String(string) => Variant::String(string.to_owned()),
            serde_json::Value::Array(array) => {
                Variant::Array(array.iter().map(Variant::from).collect())
            }
            serde_json::Value::Object(object) => {
                let new_map: BTreeMap<String, Variant> = object
                    .iter()
                    .map(|(k, v)| (k.to_owned(), Variant::from(v)))
                    .collect();
                Variant::Object(new_map)
            }
        }
    }
}

impl From<&serde_json::Value> for Variant {
    fn from(jv: &serde_json::Value) -> Self {
        match jv {
            serde_json::Value::Null => Variant::Null,
            serde_json::Value::Bool(boolean) => Variant::from(*boolean),
            serde_json::Value::Number(number) => {
                // FIXME TODO
                Variant::Rational(number.as_f64().unwrap_or(f64::NAN))
            }
            serde_json::Value::String(string) => Variant::String(string.clone()),
            serde_json::Value::Array(array) => {
                Variant::Array(array.iter().map(Variant::from).collect())
            }
            serde_json::Value::Object(object) => {
                let new_map: BTreeMap<String, Variant> = object
                    .iter()
                    .map(|(k, v)| (k.clone(), Variant::from(v)))
                    .collect();
                Variant::Object(new_map)
            }
        }
    }
}

impl<'js> From<&js::Value<'js>> for Variant {
    fn from(jv: &js::Value<'js>) -> Self {
        match jv.type_of() {
            js::Type::Undefined => Variant::Null,

            js::Type::Null => Variant::Null,

            js::Type::Bool => jv.as_bool().map_or(Variant::Null, Variant::Bool),

            js::Type::Int => jv.as_int().map_or(Variant::Null, Variant::Integer),

            js::Type::Float => jv.as_float().map_or(Variant::Null, Variant::Rational),

            js::Type::String => jv.as_string().map_or(Variant::Null, |s| {
                s.to_string().map_or(Variant::Null, Variant::String)
            }),

            js::Type::Symbol => jv.as_string().map_or(Variant::Null, |s| {
                s.to_string().map_or(Variant::Null, Variant::String)
            }),

            js::Type::Array => match jv.as_array() {
                Some(arr) => match arr.as_array_buffer() {
                    Some(buf) => Variant::from(buf),
                    _ => Variant::from(arr),
                },
                None => Variant::Null,
            },

            js::Type::Object => match jv.as_object() {
                Some(jo) => Variant::from(jo),
                None => Variant::Null,
            },

            _ => Variant::Null,
        }
    }
}

impl<'js> From<&js::Object<'js>> for Variant {
    fn from(jo: &js::Object<'js>) -> Self {
        let mut map = BTreeMap::new();
        for result in jo.props::<String, js::Value>() {
            match result {
                Ok((k, v)) => {
                    map.insert(k, Variant::from(&v));
                }
                Err(e) => {
                    eprintln!("Error occurred: {:?}", e);
                    panic!();
                }
            }
        }
        Variant::Object(map)
    }
}

impl<'js> From<&js::Array<'js>> for Variant {
    fn from(ja: &js::Array<'js>) -> Self {
        let items = ja
            .iter::<js::Value>()
            .filter(|x| x.is_ok())
            .map(|x| Variant::from(&x.unwrap()));
        Variant::Array(items.collect())
    }
}

impl<'js> From<&js::ArrayBuffer<'js>> for Variant {
    fn from(buf: &js::ArrayBuffer<'js>) -> Self {
        buf.as_bytes()
            .map_or(Variant::Null, |x| Variant::Bytes(x.to_vec()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variant_clone_should_be_ok() {
        let var1 = Variant::Array(vec![
            Variant::Integer(123),
            Variant::Integer(333),
            Variant::Array(vec![Variant::Integer(901), Variant::Integer(902)]),
        ]);
        let mut var2 = var1.clone();

        let inner_array = var2.as_array_mut().unwrap()[2].as_array_mut().unwrap();
        inner_array[0] = Variant::Integer(999);

        let value1 = var1.as_array().unwrap()[2].as_array().unwrap()[0]
            .as_integer()
            .unwrap();
        let value2 = var2.as_array().unwrap()[2].as_array().unwrap()[0]
            .as_integer()
            .unwrap();

        assert_eq!(value1, 901);
        assert_eq!(value2, 999);
        assert_ne!(value1, value2);
    }

    #[test]
    fn variant_propex_readonly_accessing_should_be_ok() {
        /*
        let obj = Variant::Object(vec![
            Variant::Integer(123),
            Variant::Integer(333),
            Variant::Array(vec![Variant::Integer(901), Variant::Integer(902)]),
        ]);


        */
        let obj1 = Variant::from([
            ("value1", Variant::Integer(123)),
            ("value2", Variant::Rational(123.0)),
            (
                "value3",
                Variant::from([
                    ("aaa", Variant::Integer(333)),
                    ("bbb", Variant::Integer(444)),
                    ("ccc", Variant::Integer(555)),
                    ("ddd", Variant::Integer(999)),
                ]),
            ),
        ]);

        let value1 = obj1
            .get_object_nav_property("value1")
            .unwrap()
            .as_integer()
            .unwrap();
        assert_eq!(value1, 123);

        let ccc_1 = obj1
            .get_object_nav_property("value3.ccc")
            .unwrap()
            .as_integer()
            .unwrap();
        assert_eq!(ccc_1, 555);

        let ccc_2 = obj1
            .get_object_nav_property("['value3'].ccc")
            .unwrap()
            .as_integer()
            .unwrap();
        assert_eq!(ccc_2, 555);

        let ccc_3 = obj1
            .get_object_nav_property("['value3'][\"ccc\"]")
            .unwrap()
            .as_integer()
            .unwrap();
        assert_eq!(ccc_3, 555);

        let ddd_1 = obj1
            .get_object_nav_property("value3.ddd")
            .unwrap()
            .as_integer()
            .unwrap();
        assert_eq!(ddd_1, 999);
    }

    #[test]
    fn variant_propex_set_nav_property_with_empty_object_should_be_ok() {
        let mut obj1 = Variant::new_empty_object();

        obj1.set_object_nav_property("address.country", Variant::String("US".to_string()), true)
            .unwrap();
        obj1.set_object_nav_property("address.zip", Variant::String("12345".to_string()), true)
            .unwrap();

        obj1.set_object_nav_property("array_field[0]", Variant::String("11111".to_string()), true)
            .unwrap();
        obj1.set_object_nav_property("array_field[1]", Variant::String("22222".to_string()), true)
            .unwrap();

        let obj_address = obj1.get_object_property("address").unwrap();

        assert!(obj_address.is_object());
        assert_eq!(
            obj_address
                .get_object_property("country")
                .unwrap()
                .as_string()
                .unwrap(),
            "US"
        );
        assert_eq!(
            obj_address
                .get_object_property("zip")
                .unwrap()
                .as_string()
                .unwrap(),
            "12345"
        );

        assert_eq!(obj_address.len(), 2);
    }
}