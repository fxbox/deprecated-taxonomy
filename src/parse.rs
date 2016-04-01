//! Utilities for defining a JSON parser.

use std::cell::RefCell;
use std::error::Error as StdError;
use std::fmt::{ Display, Debug, Error as FmtError, Formatter };
use std::rc::Rc;
use std::sync::Arc;

use serde_json::error;
use serde::ser::{ Serialize, Serializer };
use serde_json;
pub use serde_json::value::Value as JSON;
use serde::de::{ Deserializer, Error };

/// Utility function: Make sure that we have consumed all the fields of an object.
pub fn check_fields(path: Path, json: &JSON) -> Result<(), ParseError> {
    if let JSON::Object(ref obj) = *json {
        if obj.is_empty() {
            Ok(())
        } else {
            Err(ParseError::unknown_fields(obj.keys().cloned().collect(), &path))
        }
    } else {
        Ok(())
    }
}

/// A path in the JSON tree. Used for displaying error messages.
#[derive(Clone, Debug)]
pub struct Path {
    buf: Rc<RefCell<String>>,
    len: usize,
}
impl Path {
    /// Create an empty Path.
    pub fn new() -> Self {
        Path {
            buf: Rc::new(RefCell::new(String::new())),
            len: 0,
        }
    }

    /// Push a suffix after a path.
    pub fn push_str<F, T>(&self, suffix: &str, cb: F) -> T
        where F: FnOnce(Path) -> T
    {
        let buf = self.buf.clone();
        let len;
        {
            let mut str = buf.borrow_mut();
            str.push_str(suffix);
            len = str.len();
        }
        let path = Path {
            buf: buf,
            len: len,
        };
        let result = cb(path);
        {
            let mut str = self.buf.borrow_mut();
            str.truncate(self.len)
        }
        result
    }

    pub fn push_index<F, T>(&self, index: usize, cb: F) -> T
        where F: FnOnce(Path) -> T
    {
        self.push_str(&format!("[{}]", index), cb)
    }

    pub fn push<F, T>(&self, suffix: &str, cb: F) -> T
        where F: FnOnce(Path) -> T
    {
        self.push_str(&format!(".{}", suffix), cb)
    }

    pub fn to_string(&self) -> String {
        let mut str = self.buf.borrow().clone();
        str.truncate(self.len);
        str
    }
}

/// An error during parsing.
#[derive(Debug)]
pub enum ParseError {
    JSON(JSONError),
    MissingField {
        name: String,
        at: String,
    },
    UnknownFields {
        names: Vec<String>,
        at: String,
    },
    TypeError {
        name: String,
        at: String,
        expected: String,
    },
    EmptyObject {
        at: String
    },
    UnknownConstant {
        at: String,
        constant: String,
    }
}

impl Display for ParseError {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), FmtError> {
        (self as &Debug).fmt(formatter)
    }
}
impl StdError for ParseError {
    fn description(&self) -> &str {
        "Error while parsing to JSON"
    }
}

impl ParseError {
    pub fn missing_field(name: &str, at: &Path) -> Self {
        ParseError::MissingField {
            name: name.to_owned(),
            at: at.to_string(),
        }
    }
    pub fn type_error(name: &str, at: &Path, expected: &str) -> Self {
        ParseError::TypeError {
            name: name.to_owned(),
            at: at.to_string(),
            expected: expected.to_owned()
        }
    }
    pub fn unknown_fields(names: Vec<String>, at: &Path) -> Self {
        ParseError::UnknownFields {
            names: names,
            at: at.to_string()
        }
    }
    pub fn unknown_constant(value: &str, at: &Path) -> Self {
        ParseError::UnknownConstant {
            constant: value.to_owned(),
            at: at.to_string()
        }
    }
    pub fn empty_object(at: &Path) -> Self {
        ParseError::EmptyObject {
            at: at.to_string(),
        }
    }
    pub fn json(error: error::Error) -> Self {
        ParseError::JSON(JSONError(error))
    }
}

#[derive(Debug)]
pub struct JSONError(error::Error);

impl Serialize for JSONError {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer
    {
        serializer.visit_str(&format!("{:?}", self))
    }
}

/// An object that knows how to parse values from JSON into type T.
///
/// The JSON object is expected to be consumed along the way. A successful parse will
/// typically leave an empty JSON object.
pub trait Parser<T: Sized> {
    fn description() -> String;
    fn from_str(source: &str) -> Result<T, ParseError> {
        match serde_json::from_str(source) {
            Err(err) => Err(ParseError::json(err)),
            Ok(mut json) => Self::parse(Path::new(), &mut json)
        }
    }

    /// Parse a single value from JSON, consuming as much as necessary from JSON.
    fn parse(path: Path, source: &mut JSON) -> Result<T, ParseError>;

    /// Parse a field from JSON, consuming it.
    fn take(path: Path, source: &mut JSON, field_name: &str) -> Result<T, ParseError> {
        match Self::take_opt(path.clone(), source, field_name) {
            Some(result) => result,
            None => Err(ParseError::missing_field(field_name, &path))
        }
    }

    /// Parse a field from JSON, consuming it.
    fn take_opt(path: Path, source: &mut JSON, field_name: &str) -> Option<Result<T, ParseError>> {
        if let JSON::Object(ref mut obj) = *source {
            if let Some(mut v) = obj.remove(field_name) {
                Some(Self::parse(path, &mut v))
            } else {
                None
            }
        } else {
            Some(Err(ParseError::type_error(field_name, &path, "object")))
        }
    }

    /// Parse a field containing an array from JSON, consuming the field.
    fn take_vec_opt(path: Path, source: &mut JSON, field_name: &str) -> Option<Result<Vec<T>, ParseError>>
    {
        if let JSON::Object(ref mut obj) = *source {
            if let Some(ref mut json) = obj.remove(field_name) {
                if let JSON::Array(ref mut vec) = *json {
                    let mut result = Vec::with_capacity(vec.len());
                    for (json, i) in vec.iter_mut().zip(0..) {
                        match path.push(&format!("{}#{}", field_name, i),
                            |path| Self::parse(path, json)
                        ) {
                            Err(error) => return Some(Err(error)),
                            Ok(parsed) => result.push(parsed)
                        }
                    }
                    Some(Ok(result))
                } else {
                    Some(Err(ParseError::type_error(field_name, &path, "array")))
                }
            } else {
                None
            }
        } else {
            Some(Err(ParseError::missing_field(field_name, &path)))
        }
    }

    fn take_vec(path: Path, source: &mut JSON, field_name: &str) -> Result<Vec<T>, ParseError> {
        match Self::take_vec_opt(path.clone(), source, field_name) {
            Some(result) => result,
            None => Err(ParseError::missing_field(field_name, &path))
        }
    }
}

impl Parser<f64> for f64 {
    fn description() -> String {
        "Number".to_owned()
    }
    fn parse(path: Path, source: &mut JSON) -> Result<Self, ParseError> {
        match *source {
            JSON::I64(val) => Ok(val as f64),
            JSON::F64(val) => Ok(val),
            JSON::U64(val) => Ok(val as f64),
            _ => Err(ParseError::type_error("as float", &path, "number"))
        }
    }
}

impl Parser<bool> for bool {
    fn description() -> String {
        "bool".to_owned()
    }
    fn parse(path: Path, source: &mut JSON) -> Result<Self, ParseError> {
        match *source {
            JSON::Bool(ref b) => Ok(*b),
            JSON::U64(0) | JSON::I64(0) => Ok(false),
            JSON::U64(1) | JSON::I64(1) => Ok(true),
            JSON::String(ref str) if str == "true" => Ok(true),
            JSON::String(ref str) if str == "false" => Ok(false),
            _ => Err(ParseError::type_error("as bool", &path, "boolean"))
        }
    }
}

impl Parser<u8> for u8 {
    fn description() -> String {
        "byte".to_owned()
    }
    fn parse(path: Path, source: &mut JSON) -> Result<Self, ParseError> {
        match source.as_u64() {
            None => Err(ParseError::type_error("as byte", &path, "positive integer")),
            Some(ref val) if *val > u8::max_value() as u64 =>
                Err(ParseError::type_error("as byte", &path, "positive integer")),
            Some(ref val) => Ok(*val as u8)
        }
    }
}

impl<T> Parser<Vec<T>> for Vec<T> where T: Parser<T> {
    fn description() -> String {
        format!("Array<{}>", T::description())
    }
    fn parse(path: Path, source: &mut JSON) -> Result<Self, ParseError> {
        // Otherwise, parse as an actual array.
        match *source {
            JSON::Array(ref mut array) => {
                let mut result = Vec::with_capacity(array.len());
                for (source, i) in array.iter_mut ().zip(0..) {
                    let value = try!(path.push_index(i, |path| T::parse(path, source)));
                    result.push(value)
                }
                Ok(result)
            }
            JSON::Null => {
                // Accept `null` as an empty array.
                Ok(vec![])
            }
            _ => {
                // Attempt to promote the value to an array.
                let single = try!(path.push_str("", |path| T::parse(path, source)));
                Ok(vec![single])
            }
        }
    }
}

/*
impl<T, U> Parser<(T, U)> for (T, U) where T: Parser<T>, U: Parser<U> {
    fn description() -> String {
        format!("({}, {})", T::description(), U::description())
    }
    fn parse(path: Path, source: &mut JSON) -> Result<Self, ParseError> {
        match *source {
            JSON::Array(ref mut array) if array.len() == 2 => {
                let mut right = array.pop().unwrap(); // We just checked that length == 2
                let mut left = array.pop().unwrap(); // We just checked that length == 2
                let left_parsed = try!(path.push(&T::description() as &str, |path| {T::parse(path, &mut left)}));
                let right_parsed = try!(path.push(&U::description() as &str, |path| {U::parse(path, &mut right)}));
                Ok((left_parsed, right_parsed))
            }
            _ => Err(ParseError::type_error("pair of values", &path, "array"))
        }
    }
}
*/
impl Parser<String> for String {
    fn description() -> String {
        "String".to_owned()
    }
    fn parse(path: Path, source: &mut JSON) -> Result<Self, ParseError> {
        match source.as_string() {
            Some(str) => Ok(str.to_owned()),
            None => Err(ParseError::type_error("string", &path, "string"))
        }
    }
}


impl<T> Parser<Arc<T>> for Arc<T> where T: Parser<T> {
    fn description() -> String {
        T::description()
    }
    fn parse(path: Path, source: &mut JSON) -> Result<Self, ParseError> {
        Ok(Arc::new(try!(T::parse(path, source))))
    }
}

/*
impl<T> Parser<T> for T where T: Deserialize {
    fn parse(_: Path, source: &mut JSON) -> Result<T, ParseError> {
        use serde_json;
        serde_json::from_value(source.clone()).map_err(ParseError::json)
    }
}
*/