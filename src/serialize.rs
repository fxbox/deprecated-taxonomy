//! Utilities for serializing data to JSON.
use util::*;

use std::collections::{ BTreeMap, HashMap, HashSet };
use std::hash::Hash;

use serde_json::value::Value as JSON;

/// A store to put binary parts.
pub trait BinaryParts {
    fn push(&mut self, mimetype: Id<MimeTypeId>, binary: &[u8]) -> JSON;
}

/// An imnplementation of `MultiPart` that stores everything in a HashMap and returns
/// JSON objects `"{part: i}"`, where `i` is the (integer) key in the HashMap.
pub struct MultiPart {
    pub buf: Vec<(Id<MimeTypeId>, Vec<u8>)>,
}
impl MultiPart {
    pub fn new() -> Self {
        MultiPart {
            buf: Vec::new(),
        }
    }
}
impl BinaryParts for MultiPart {
    fn push(&mut self, mimetype: Id<MimeTypeId>, binary: &[u8]) -> JSON {
        let mut vec = Vec::with_capacity(binary.len());
        vec.extend_from_slice(binary);
        self.buf.push((mimetype, vec));
        let mut map = BTreeMap::new();
        map.insert("part".to_owned(), JSON::U64(self.buf.len() as u64));
        JSON::Object(map)
    }
}

pub trait ToJSON {
    fn to_json(&self, parts: &mut BinaryParts) -> JSON;
}

impl ToJSON for String {
    fn to_json(&self, _: &mut BinaryParts) -> JSON {
        JSON::String(self.clone())
    }
}

impl ToJSON for bool {
    fn to_json(&self, _: &mut BinaryParts) -> JSON {
        JSON::Bool(*self)
    }
}

impl ToJSON for f64 {
    fn to_json(&self, _: &mut BinaryParts) -> JSON {
        JSON::F64(*self)
    }
}

impl ToJSON for usize {
    fn to_json(&self, _: &mut BinaryParts) -> JSON {
        JSON::U64(*self as u64)
    }
}

impl ToJSON for JSON {
    fn to_json(&self, _: &mut BinaryParts) -> JSON {
        self.clone()
    }
}

impl<T> ToJSON for HashSet<T> where T: ToJSON + Eq + Hash {
    fn to_json(&self, parts: &mut BinaryParts) -> JSON {
        JSON::Array((*self).iter().map(|x| x.to_json(parts)).collect())
    }
}

impl<T> ToJSON for HashMap<String, T> where T: ToJSON {
    fn to_json(&self, parts: &mut BinaryParts) -> JSON {
        JSON::Object(self.iter().map(|(k, v)| (k.clone(), T::to_json(v, parts))).collect())
    }
}

impl<T> ToJSON for Vec<T> where T: ToJSON {
    fn to_json(&self, parts: &mut BinaryParts) -> JSON {
        JSON::Array(self.iter().map(|x| x.to_json(parts)).collect())
    }
}

impl<'a, T> ToJSON for Vec<(&'a str, T)> where T: ToJSON {
    fn to_json(&self, parts: &mut BinaryParts) -> JSON {
        JSON::Object(self.iter().map(|&(ref k, ref v)| {
            ((*k).to_owned(), v.to_json(parts))
        }).collect())
    }
}

impl <'a> ToJSON for &'a str {
    fn to_json(&self, _: &mut BinaryParts) -> JSON {
        JSON::String((*self).to_owned())
    }
}

impl<'a, T> ToJSON for &'a T where T: ToJSON {
    fn to_json(&self, parts: &mut BinaryParts) -> JSON {
        (**self).to_json(parts)
    }
}

impl<K, T, V> ToJSON for HashMap<Id<K>, Result<T, V>> where T: ToJSON, V: ToJSON {
    fn to_json(&self, parts: &mut BinaryParts) -> JSON {
        JSON::Object(self.iter().map(|(k, result)| {
            let k = k.to_string();
            let result = match *result {
                Ok(ref ok) => ok.to_json(parts),
                Err(ref err) => vec![("Error", err)].to_json(parts)
            };
            (k, result)
        }).collect())
    }
}

impl<T> ToJSON for Option<T> where T: ToJSON {
    fn to_json(&self, parts: &mut BinaryParts) -> JSON {
        match *self {
            None => JSON::Null,
            Some(ref result) => result.to_json(parts)
        }
    }
}

impl ToJSON for () {
    fn to_json(&self, _: &mut BinaryParts) -> JSON {
        JSON::Null
    }
}
