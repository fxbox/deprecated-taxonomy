//!
//! Values manipulated by services
//!
use parse::*;
use serialize::*;
use util::*;

use std::cmp::{ PartialOrd, Ordering };
use std::collections::HashMap;
use std::fmt::Debug;
use std::str::FromStr;
use std::sync::Arc;
use std::{ error, fmt };

use chrono::{ Duration as ChronoDuration, DateTime, Local, TimeZone, UTC };

use serde_json;
use serde::ser::{ Serialize, Serializer };
use serde::de::{ Deserialize, Deserializer, Error, Visitor as DeserializationVisitor };

/// Representation of a type error.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TypeError {
    /// The type we expected.
    pub expected: Type,

    /// The type we actually got.
    pub got: Type,
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Expected {:?} but got {:?}", self.expected, self.got)
    }
}

impl error::Error for TypeError {
    fn description(&self) -> &str {
        "Expected a type but got another type"
    }

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

///
/// The type of values manipulated by endpoints.
///
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub enum Type {
    ///
    /// # Trivial values
    ///

    /// An empty value. Used for instance to inform that a countdown
    /// has reached 0 or that a device is ready.
    Unit,

    ///
    /// # Boolean values
    ///

    /// A boolean on/off state. Used for various two-states switches.
    OnOff,

    /// A boolean open/closed state. Used for instance for doors,
    /// windows, etc.
    OpenClosed,

    ///
    /// # Time
    ///

    /// A duration. Used for instance in countdowns.
    Duration,

    /// A precise timestamp. Used for instance to determine when an
    /// event has taken place.
    TimeStamp,

    ThinkerbellRule,

    Temperature,
    String,
    ///
    /// ...
    ///
    Color,
    Json,
    Binary,

    ExtBool,
    ExtNumeric,
}
impl Parser<Type> for Type {
    fn description() -> String {
        "Type".to_owned()
    }
    fn parse(path: Path, source: &mut JSON) -> Result<Self, ParseError> {
        use self::Type::*;
        match *source {
            JSON::String(ref string) => match &*string as &str {
                "Unit" => Ok(Unit),
                "OnOff" => Ok(OnOff),
                "OpenClosed" => Ok(OpenClosed),
                "Duration" => Ok(Duration),
                "TimeStamp" => Ok(TimeStamp),
                "Temperature" => Ok(Temperature),
                "ThinkerbellRule" => Ok(ThinkerbellRule),
                "String" => Ok(String),
                "Color" => Ok(Color),
                "Json" => Ok(Json),
                "Binary" => Ok(Binary),
                "ExtBool" => Ok(ExtBool),
                "ExtNumeric" => Ok(ExtNumeric),
                _ => Err(ParseError::unknown_constant(string, &path))
            },
            _ => Err(ParseError::type_error("Type", &path, "string"))
        }
    }
}
impl ToJSON for Type {
    fn to_json(&self, parts: &mut BinaryParts) -> JSON {
        use self::Type::*;
        let key = match *self {
            Unit => "Unit",
            OnOff => "OnOff",
            OpenClosed => "OpenClosed",
            Duration => "Duration",
            TimeStamp => "TimeStamp",
            Temperature => "Temperature",
            ThinkerbellRule => "ThinkerbellRule",
            String => "String",
            Color => "Color",
            Json => "Json",
            Binary => "Binary",
            ExtBool => "ExtBool",
            ExtNumeric => "ExtNumeric",
        };
        key.to_json(parts)
    }
}

impl Type {
    /// Determine whether using `Range::Eq` for this type is
    /// appropriate. Typically, using `Range::Eq` for a floating point
    /// number is a bad idea.
    pub fn supports_eq(&self) -> bool {
        use self::Type::*;
        match *self {
            Duration | TimeStamp | Temperature | ExtNumeric | Color | ThinkerbellRule => false,
            Unit | String | Json | Binary | OnOff | OpenClosed | ExtBool => true,
        }
    }

    pub fn ensure_eq(&self, other: &Self) -> Result<(), TypeError> {
        if self == other {
            Ok(())
        } else {
            Err(TypeError {
                expected: self.clone(),
                got: other.clone(),
            })
        }
    }
}

/// An on/off state.
///
/// # JSON
///
/// This kind is represented by strings "On" | "Off".
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum OnOff {
    /// # JSON
    ///
    /// Represented by "On".
    ///
    /// ```
    /// use foxbox_taxonomy::values::*;
    /// use foxbox_taxonomy::parse::*;
    /// use foxbox_taxonomy::serialize::*;
    ///
    /// let parsed = OnOff::from_str("\"On\"").unwrap();
    /// assert_eq!(parsed, OnOff::On);
    ///
    /// let serialized: JSON = OnOff::On.to_json(&mut MultiPart::new());
    /// assert_eq!(serialized.as_string().unwrap(), "On");
    /// ```
    On,

    /// # JSON
    ///
    /// Represented by "Off".
    ///
    /// ```
    /// use foxbox_taxonomy::values::*;
    /// use foxbox_taxonomy::parse::*;
	/// use foxbox_taxonomy::serialize::*;
    ///
    /// let parsed = OnOff::from_str("\"On\"").unwrap();
    /// assert_eq!(parsed, OnOff::On);
    ///
    /// let serialized: JSON = OnOff::On.to_json(&mut MultiPart::new());
    /// assert_eq!(serialized.as_string().unwrap(), "On");
    /// ```
    Off,
}

impl OnOff {
    fn as_bool(&self) -> bool {
        match *self {
            OnOff::On => true,
            OnOff::Off => false,
        }
    }
}

impl Parser<OnOff> for OnOff {
    fn description() -> String {
        "OnOff".to_owned()
    }
    fn parse(path: Path, source: &mut JSON) -> Result<Self, ParseError> {
        match source.as_string() {
            Some("On") => Ok(OnOff::On),
            Some("Off") => Ok(OnOff::Off),
            Some(str) => Err(ParseError::unknown_constant(str, &path)),
            None => Err(ParseError::type_error("OnOff", &path, "string"))
        }
    }
}

impl ToJSON for OnOff {
    fn to_json(&self, parts: &mut BinaryParts) -> JSON {
        match *self {
            OnOff::On => "On".to_json(parts),
            OnOff::Off => "Off".to_json(parts),
        }
    }
}
impl Into<Value> for OnOff {
    fn into(self) -> Value {
        Value::OnOff(self)
    }
}

impl PartialOrd for OnOff {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OnOff {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_bool().cmp(&other.as_bool())
    }
}

///
/// # (De)serialization
///
/// Values of this type are represented by strings "On" | "Off".
///
/// ```
/// extern crate serde;
/// extern crate serde_json;
/// extern crate foxbox_taxonomy;
///
/// let on = serde_json::to_string(&foxbox_taxonomy::values::OnOff::On).unwrap();
/// assert_eq!(on, "\"On\"");
///
/// let on : foxbox_taxonomy::values::OnOff = serde_json::from_str("\"On\"").unwrap();
/// assert_eq!(on, foxbox_taxonomy::values::OnOff::On);
///
/// let off = serde_json::to_string(&foxbox_taxonomy::values::OnOff::Off).unwrap();
/// assert_eq!(off, "\"Off\"");
///
/// let off : foxbox_taxonomy::values::OnOff = serde_json::from_str("\"Off\"").unwrap();
/// assert_eq!(off, foxbox_taxonomy::values::OnOff::Off);
/// ```
impl Serialize for OnOff {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error> where S: Serializer {
        match *self {
            OnOff::On => "On".serialize(serializer),
            OnOff::Off => "Off".serialize(serializer)
        }
    }
}
impl Deserialize for OnOff {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error> where D: Deserializer {
        deserializer.visit_string(TrivialEnumVisitor::new(|source| {
            match source {
                "On" => Ok(OnOff::On),
                "Off" => Ok(OnOff::Off),
                _ => Err(())
            }
        }))
    }
}

/// An open/closed state.
///
/// # JSON
///
/// Values of this type are represented by strings "Open" | "Closed".
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum OpenClosed {
    /// # JSON
    ///
    /// Represented by "Open".
    ///
    /// ```
    /// use foxbox_taxonomy::values::*;
    /// use foxbox_taxonomy::parse::*;
	/// use foxbox_taxonomy::serialize::*;
    ///
    /// let parsed = OpenClosed::from_str("\"Open\"").unwrap();
    /// assert_eq!(parsed, OpenClosed::Open);
    ///
    /// let serialized: JSON = OpenClosed::Open.to_json(&mut MultiPart::new());
    /// assert_eq!(serialized.as_string().unwrap(), "Open");
    /// ```
    Open,

    /// # JSON
    ///
    /// Represented by "Closed".
    ///
    /// ```
    /// use foxbox_taxonomy::values::*;
    /// use foxbox_taxonomy::parse::*;
	/// use foxbox_taxonomy::serialize::*;
    ///
    /// let parsed = OpenClosed::from_str("\"Closed\"").unwrap();
    /// assert_eq!(parsed, OpenClosed::Closed);
    ///
    /// let serialized: JSON = OpenClosed::Closed.to_json(&mut MultiPart::new());
    /// assert_eq!(serialized.as_string().unwrap(), "Closed");
    /// ```
    Closed,
}

impl OpenClosed {
    fn as_bool(&self) -> bool {
        match *self {
            OpenClosed::Open => true,
            OpenClosed::Closed => false,
        }
    }
}

impl Parser<OpenClosed> for OpenClosed {
    fn description() -> String {
        "OpenClosed".to_owned()
    }
    fn parse(path: Path, source: &mut JSON) -> Result<Self, ParseError> {
        match source.as_string() {
            Some("Open") => Ok(OpenClosed::Open),
            Some("Closed") => Ok(OpenClosed::Closed),
            Some(str) => Err(ParseError::unknown_constant(str, &path)),
            None => Err(ParseError::type_error("OpenClosed", &path, "string"))
        }
    }
}

impl ToJSON for OpenClosed {
    fn to_json(&self, parts: &mut BinaryParts) -> JSON {
        match *self {
            OpenClosed::Open => "Open".to_json(parts),
            OpenClosed::Closed => "Closed".to_json(parts)
        }
    }
}
impl Into<Value> for OpenClosed {
    fn into(self) -> Value {
        Value::OpenClosed(self)
    }
}

impl PartialOrd for OpenClosed {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OpenClosed {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_bool().cmp(&other.as_bool())
    }
}

///
/// # (De)serialization
///
/// Values of this state are represented by strings "Open"|"Closed".
///
/// ```
/// extern crate serde;
/// extern crate serde_json;
/// extern crate foxbox_taxonomy;
///
/// let open = serde_json::to_string(&foxbox_taxonomy::values::OpenClosed::Open).unwrap();
/// assert_eq!(open, "\"Open\"");
///
/// let open : foxbox_taxonomy::values::OpenClosed = serde_json::from_str("\"Open\"").unwrap();
/// assert_eq!(open, foxbox_taxonomy::values::OpenClosed::Open);
///
/// let closed = serde_json::to_string(&foxbox_taxonomy::values::OpenClosed::Closed).unwrap();
/// assert_eq!(closed, "\"Closed\"");
///
/// let closed : foxbox_taxonomy::values::OpenClosed = serde_json::from_str("\"Closed\"").unwrap();
/// assert_eq!(closed, foxbox_taxonomy::values::OpenClosed::Closed);
/// ```
impl Serialize for OpenClosed {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error> where S: Serializer {
        match *self {
            OpenClosed::Open => "Open".serialize(serializer),
            OpenClosed::Closed => "Closed".serialize(serializer)
        }
    }
}
impl Deserialize for OpenClosed {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error> where D: Deserializer {
        deserializer.visit_string(TrivialEnumVisitor::new(|source| {
            match source {
                "Open" | "open" => Ok(OpenClosed::Open),
                "Closed" | "closed" => Ok(OpenClosed::Closed),
                _ => Err(())
            }
        }))
    }
}

/// A temperature. Internal representation may be either Fahrenheit or
/// Celcius. The FoxBox adapters are expected to perform conversions
/// to the format requested by their devices.
///
/// # JSON
///
/// Values of this type are represented by objects `{F; float}` or `{C: float}`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Temperature {
    /// Fahrenheit
    ///
    /// # JSON
    ///
    /// ```
    /// use foxbox_taxonomy::values::*;
    /// use foxbox_taxonomy::parse::*;
	/// use foxbox_taxonomy::serialize::*;
    ///
    /// let source = "{
    ///   \"F\": 100
    /// }";
    /// let parsed = Temperature::from_str(source).unwrap();
    /// if let Temperature::F(100.) = parsed {
    ///    // As expected
    /// } else {
    ///    panic!()
    /// }
    ///
    /// let serialized : JSON = parsed.to_json(&mut MultiPart::new());
    /// let val = serialized.find("F").unwrap().as_f64().unwrap();
    /// assert_eq!(val, 100.)
    /// ```
    F(f64),

    /// Celcius
    ///
    /// # JSON
    ///
    /// ```
    /// use foxbox_taxonomy::values::*;
    /// use foxbox_taxonomy::parse::*;
	/// use foxbox_taxonomy::serialize::*;
    ///
    /// let source = "{
    ///   \"C\": 100
    /// }";
    /// let parsed = Temperature::from_str(source).unwrap();
    /// if let Temperature::C(100.) = parsed {
    ///    // As expected
    /// } else {
    ///    panic!()
    /// }
    ///
    /// let serialized : JSON = parsed.to_json(&mut MultiPart::new());
    /// let val = serialized.find("C").unwrap().as_f64().unwrap();
    /// assert_eq!(val, 100.)
    /// ```
    C(f64),
}

impl Temperature {
    /// Get a temperature in Fahrenheit.
    pub fn as_f(&self) -> f64 {
        unimplemented!();
    }

    /// Get a temperature in Celcius.
    pub fn as_c(&self) -> f64 {
        unimplemented!();
    }
}

impl Parser<Temperature> for Temperature {
    fn description() -> String {
        "Temperature".to_owned()
    }
    fn parse(path: Path, source: &mut JSON) -> Result<Self, ParseError> {
        if !source.is_object() {
            return Err(ParseError::type_error("Temperature", &path, "object"));
        }
        if let Some(result) = path.push("F", |path| f64::take_opt(path, source, "F")) {
            return result.map(Temperature::F);
        }
        if let Some(result) = path.push("C", |path| f64::take_opt(path, source, "C")) {
            return result.map(Temperature::C);
        }
        Err(ParseError::missing_field("C|F", &path))
    }
}
impl ToJSON for Temperature {
    fn to_json(&self, parts: &mut BinaryParts) -> JSON {
        let obj = match *self {
            Temperature::C(val) => {
                ("C", val.to_json(parts))
            }
            Temperature::F(val) => {
                ("F", val.to_json(parts))
            }
        };
        vec![obj].to_json(parts)
    }
}
impl PartialOrd for Temperature {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.as_c().partial_cmp(&other.as_c())
    }
}

/// A color. Internal representation may vary. The FoxBox adapters are
/// expected to perform conversions to the format requested by their
/// device.
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum Color {
    /// # JSON
    ///
    /// Values are represented as an object {r: float, f: float, b: float, a: float},
    /// where each color is between 0 and 1. Field `a` may be omitted, in which case
    /// it is taken to be 0.
    ///
    /// ```
    /// use foxbox_taxonomy::values::*;
    /// use foxbox_taxonomy::parse::*;
	/// use foxbox_taxonomy::serialize::*;
    ///
    /// println!("Testing parsing");
    /// let source = "{
    ///   \"r\": 0.1,
    ///   \"g\": 0.2,
    ///   \"b\": 0.4,
    ///   \"a\": 0.8
    /// }";
    ///
    /// let parsed = Color::from_str(source).unwrap();
    /// let Color::RGBA(r, g, b, a) = parsed;
    /// assert_eq!(r, 0.1);
    /// assert_eq!(g, 0.2);
    /// assert_eq!(b, 0.4);
    /// assert_eq!(a, 0.8);
    ///
    /// println!("Testing serialization");
    /// let serialized : JSON = parsed.to_json(&mut MultiPart::new());
    /// let r = serialized.find("r").unwrap().as_f64().unwrap();
    /// assert_eq!(r, 0.1);
    /// let g = serialized.find("g").unwrap().as_f64().unwrap();
    /// assert_eq!(g, 0.2);
    /// let b = serialized.find("b").unwrap().as_f64().unwrap();
    /// assert_eq!(b, 0.4);
    /// let a = serialized.find("a").unwrap().as_f64().unwrap();
    /// assert_eq!(a, 0.8);
    ///
    ///
    /// println!("Testing parsing error (value not in [0, 1])");
    /// // This source will not parse.
    /// let source_2 = "{
    ///   \"r\": 100,
    ///   \"g\": 0.2,
    ///   \"b\": 0.4,
    ///   \"a\": 0.9
    /// }";
    ///
    /// match Color::from_str(source_2) {
    ///   Err(ParseError::TypeError{..}) => {},
    ///   other => panic!("Unexpected result {:?}", other)
    /// }
    ///
    ///
    /// println!("Testing auto-added alpha");
    /// // This source does not specify alpha, so alpha is 0.
    /// let source_3 = "{
    ///   \"r\": 0.1,
    ///   \"g\": 0.2,
    ///   \"b\": 0.4
    /// }";
    ///
    /// let parsed = Color::from_str(source_3).unwrap();
    /// let Color::RGBA(r, g, b, a) = parsed;
    /// assert_eq!(r, 0.1);
    /// assert_eq!(g, 0.2);
    /// assert_eq!(b, 0.4);
    /// assert_eq!(a, 0.);
    ///
    ///
    /// println!("Testing parsing error (missing field)");
    /// // This source does not specify b, so it will not parse.
    /// let source_4 = "{
    ///   \"r\": 0.1,
    ///   \"g\": 0.2
    /// }";
    ///
    /// match Color::from_str(source_4) {
    ///   Err(ParseError::MissingField{ref name, ..}) if &name as &str == "b" => {},
    ///   other => panic!("Unexpected result {:?}", other)
    /// }
    /// ```
    RGBA(f64, f64, f64, f64)
}
impl Parser<Color> for Color {
    fn description() -> String {
        "Color".to_owned()
    }
    fn parse(path: Path, source: &mut JSON) -> Result<Self, ParseError> {
        let r = try!(path.push("r", |path| f64::take(path, source, "r")));
        let g = try!(path.push("g", |path| f64::take(path, source, "g")));
        let b = try!(path.push("b", |path| f64::take(path, source, "b")));
        let a = try!(match path.push("a", |path| f64::take_opt(path, source, "a")) {
            None => Ok(0.),
            Some(a) => a
        });
        for &(val, ref name) in &vec![(&r, "r"), (&g, "g"), (&b, "b"), (&a, "a")] {
            if *val < 0. || *val > 1. {
                return Err(ParseError::type_error(name, &path, "a number in [0, 1]"));
            }
        }
        Ok(Color::RGBA(r, g, b, a))
    }
}

impl ToJSON for Color {
    fn to_json(&self, parts: &mut BinaryParts) -> JSON {
        let Color::RGBA(ref r, ref g, ref b, ref a) = *self;
        vec![("r", r), ("g", g), ("b", b), ("a", a)].to_json(parts)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThinkerbellRule {
    pub name: String,
    pub source: String,
}

impl Parser<ThinkerbellRule> for ThinkerbellRule {
    fn description() -> String {
        "ThinkerbellRuleSource".to_owned()
    }
    fn parse(path: Path, source: &mut JSON) -> Result<Self, ParseError> {
        let name = try!(path.push("name", |path| String::take(path, source, "name")));
        let script_source = try!(path.push("source", |path| String::take(path, source, "source")));
        Ok(ThinkerbellRule { name: name, source: script_source })
    }
}
impl ToJSON for ThinkerbellRule {
    fn to_json(&self, parts: &mut BinaryParts) -> JSON {
        vec![
            ("name", &self.name),
            ("source", &self.source),
        ].to_json(parts)
    }
}

/// Representation of an object in JSON. It is often (albeit not
/// always) possible to choose a more precise data structure for
/// representing values send/accepted by a service. If possible,
/// adapters should rather pick such more precise data structure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Json(pub serde_json::value::Value);

impl Parser<Json> for Json {
    fn description() -> String {
        "Json value".to_owned()
    }
    fn parse(_: Path, source: &mut JSON) -> Result<Self, ParseError> {
        Ok(Json(source.clone()))
    }
}
impl ToJSON for Json {
    fn to_json(&self, _: &mut BinaryParts) -> JSON {
        self.0.clone()
    }
}

impl PartialOrd for Json {
    /// Two Json objects are never comparable to each other.
    fn partial_cmp(&self, _: &Self) -> Option<Ordering> {
        None
    }
}

/// A data structure holding a boolean value of a type that has not
/// been standardized yet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtValue<T> where T: Debug + Clone + PartialEq + PartialOrd + Serialize + Deserialize {
    pub value: T,

    /// The vendor. Used for namespacing purposes, to avoid
    /// confusing two incompatible extensions with similar
    /// names. For instance, "foxlink@mozilla.com".
    pub vendor: Id<VendorId>,

    /// Identification of the adapter introducing this value.
    /// Designed to aid with tracing and debugging.
    pub adapter: Id<AdapterId>,

    /// A string describing the nature of the value, designed to
    /// aid with type-checking.
    ///
    /// Examples: `"PresenceDetected"`.
    pub kind: Id<KindId>,
}

impl<T> Parser<ExtValue<T>> for ExtValue<T>
    where T: Debug + Clone + PartialEq + PartialOrd + Serialize + Deserialize + Parser<T>
{
    fn description() -> String {
        format!("ExtValue<{}>", T::description())
    }
    fn parse(path: Path, source: &mut JSON) -> Result<Self, ParseError> {
        let vendor = try!(path.push("vendor", |path| Id::take(path, source, "vendor")));
        let adapter = try!(path.push("adapter", |path| Id::take(path, source, "adapter")));
        let kind = try!(path.push("kind", |path| Id::take(path, source, "kind")));
        let value = try!(path.push("value", |path| T::take(path, source, "value")));
        Ok(ExtValue {
            vendor: vendor,
            adapter: adapter,
            kind: kind,
            value: value
        })
    }
}

impl<T> ToJSON for ExtValue<T>
    where T: Debug + Clone + PartialEq + PartialOrd + Serialize + Deserialize + ToJSON
{
    fn to_json(&self, parts: &mut BinaryParts) -> JSON {
        vec![
            ("value", self.value.to_json(parts)),
            ("vendor", self.vendor.to_json(parts)),
            ("adapter", self.adapter.to_json(parts)),
            ("kind", self.kind.to_json(parts))
        ].to_json(parts)
    }
}

impl<T> PartialEq<ExtValue<T>> for ExtValue<T>
    where T: Debug + Clone + PartialEq + PartialOrd + Serialize + Deserialize
{
    fn eq(&self, other: &Self) -> bool {
        if self.vendor != other.vendor
        || self.kind != other.kind {
            false
        } else {
            self.value.eq(&other.value)
        }
    }
}

impl<T> PartialOrd<ExtValue<T>> for ExtValue<T>
    where T: Debug + Clone + PartialEq + PartialOrd + Serialize + Deserialize
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.vendor != other.vendor
        || self.kind != other.kind {
            None
        } else {
            self.value.partial_cmp(&other.value)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Binary {
   /// The actual data. We put it behind an `Arc` to make sure
   /// that cloning remains inexpensive.
   pub data: Arc<Vec<u8>>,

   /// The mime type. Should probably be an Id<MimeTypeId>.
   pub mimetype: Id<MimeTypeId>,
}

impl Parser<Binary> for Binary {
    fn description() -> String {
        "Binary".to_owned()
    }
    fn parse(path: Path, source: &mut JSON) -> Result<Self, ParseError> {
        let data = try!(path.push("data", |path| Vec::<u8>::take(path, source, "data")));
        let mimetype = try!(path.push("mimetype", |path| Id::take(path, source, "mimetype")));
        Ok(Binary {
            data: Arc::new(data),
            mimetype: mimetype
        })
    }
}

impl ToJSON for Binary {
    fn to_json(&self, parts: &mut BinaryParts) -> JSON {
        parts.push(self.mimetype.clone(), &*self.data)
    }
}

/// Representation of an actual value that can be sent to/received
/// from a service.
///
/// # JSON
///
/// Values of this state are represented by an object `{ key: value }`, where key is one of
/// `Unit`, `OnOff`, `OpenClosed`, ... The `value` for `Unit` is ignored.
///
/// # Other forms of (de)serialization
///
/// Values of this state are represented by an object `{ key: value }`, where key is one of
/// `Unit`, `OnOff`, `OpenClosed`, ... The `value` for `Unit` is ignored.
///
/// ```
/// extern crate serde;
/// extern crate serde_json;
/// extern crate foxbox_taxonomy;
///
/// # fn main() {
/// use foxbox_taxonomy::values::Value::*;
/// use foxbox_taxonomy::values::OnOff::*;
/// use foxbox_taxonomy::values::OpenClosed::*;
///
/// let unit = serde_json::to_string(&Unit).unwrap();
/// assert_eq!(unit, "{\"Unit\":[]}");
///
/// let unit : foxbox_taxonomy::values::Value = serde_json::from_str("{\"Unit\":[]}").unwrap();
/// assert_eq!(unit, Unit);
///
/// let on = serde_json::to_string(&OnOff(On)).unwrap();
/// assert_eq!(on, "{\"OnOff\":\"On\"}");
///
/// let on : foxbox_taxonomy::values::Value = serde_json::from_str("{\"OnOff\":\"On\"}").unwrap();
/// assert_eq!(on, OnOff(On));
///
/// let open = serde_json::to_string(&OpenClosed(Open)).unwrap();
/// assert_eq!(open, "{\"OpenClosed\":\"Open\"}");
///
/// let open : foxbox_taxonomy::values::Value = serde_json::from_str("{\"OpenClosed\":\"Open\"}").unwrap();
/// assert_eq!(open, OpenClosed(Open));
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    /// An absolute time and date.
    ///
    /// # JSON
    ///
    /// Represented as `{"TimeStamp": string}`, where `string` is formatted as RFC 3339 such as
    /// `"2014-11-28T21:45:59.324310806+09:00"`.
    ///
    /// ```
    /// extern crate foxbox_taxonomy;
    ///
    /// use foxbox_taxonomy::values::*;
    /// use foxbox_taxonomy::parse::*;
	/// use foxbox_taxonomy::serialize::*;
    ///
    /// # fn main() {
    ///
    /// let source = "{
    ///   \"Unit\": []
    /// }";
    ///
    /// let parsed = Value::from_str(source).unwrap();
    /// if let Value::Unit = parsed {
    ///   // ok
    /// } else {
    ///   panic!();
    /// }
    ///
    ///
    /// let serialized: JSON = parsed.to_json(&mut MultiPart::new());
    /// if let JSON::Object(ref obj) = serialized {
    ///   let serialized = obj.get("Unit").unwrap();
    ///   assert!(serialized.is_null());
    /// }
    /// # }
    /// ```
    Unit,

    /// An on/off value.
    ///
    /// # JSON
    ///
    /// Represented as `{"OnOff": string}`, where `string` is "On" or "Off".
    ///
    /// ```
    /// extern crate foxbox_taxonomy;
    ///
    /// use foxbox_taxonomy::values::*;
    /// use foxbox_taxonomy::parse::*;
	/// use foxbox_taxonomy::serialize::*;
    ///
    /// # fn main() {
    ///
    /// let source = "{
    ///   \"OnOff\": \"On\"
    /// }";
    ///
    /// let parsed = Value::from_str(source).unwrap();
    /// if let Value::OnOff(OnOff::On) = parsed {
    ///   // ok
    /// } else {
    ///   panic!();
    /// }
    ///
    ///
    /// let serialized: JSON = parsed.to_json(&mut MultiPart::new());
    /// if let JSON::Object(ref obj) = serialized {
    ///   let serialized = obj.get("OnOff").unwrap();
    ///   assert_eq!(serialized.as_string().unwrap(), "On");
    /// }
    /// # }
    /// ```
    OnOff(OnOff),

    /// An open/closed value.
    ///
    /// # JSON
    ///
    /// Represented as `{"OpenClosed": string}`, where `string` is "Open" or "Closed".
    ///
    /// ```
    /// extern crate foxbox_taxonomy;
    ///
    /// use foxbox_taxonomy::values::*;
    /// use foxbox_taxonomy::parse::*;
	/// use foxbox_taxonomy::serialize::*;
    ///
    /// # fn main() {
    ///
    /// let source = "{
    ///   \"OpenClosed\": \"Open\"
    /// }";
    ///
    /// let parsed = Value::from_str(source).unwrap();
    /// if let Value::OpenClosed(OpenClosed::Open) = parsed {
    ///   // ok
    /// } else {
    ///   panic!();
    /// }
    ///
    ///
    /// let serialized: JSON = parsed.to_json(&mut MultiPart::new());
    /// if let JSON::Object(ref obj) = serialized {
    ///   let serialized = obj.get("OpenClosed").unwrap();
    ///   assert_eq!(serialized.as_string().unwrap(), "Open");
    /// }
    /// # }
    /// ```
    OpenClosed(OpenClosed),

    /// An absolute time and date.
    ///
    /// # JSON
    ///
    /// Represented as `{"TimeStamp": string}`, where `string` is formatted as RFC 3339 such as
    /// `"2014-11-28T21:45:59.324310806+09:00"`.
    ///
    /// ```
    /// extern crate chrono;
    /// extern crate foxbox_taxonomy;
    ///
    /// use foxbox_taxonomy::values::*;
    /// use foxbox_taxonomy::parse::*;
	/// use foxbox_taxonomy::serialize::*;
    /// use chrono::Datelike;
    ///
    /// # fn main() {
    ///
    /// let source = "{
    ///   \"TimeStamp\": \"2014-11-28T21:45:59.324310806+09:00\"
    /// }";
    ///
    /// let parsed = Value::from_str(source).unwrap();
    /// if let Value::TimeStamp(ref ts) = parsed {
    ///   let date_time = ts.as_datetime();
    ///   assert_eq!(date_time.year(), 2014);
    ///   assert_eq!(date_time.month(), 11);
    ///   assert_eq!(date_time.day(), 28);
    /// } else {
    ///   panic!();
    /// }
    ///
    ///
    /// let serialized: JSON = parsed.to_json(&mut MultiPart::new());
    /// if let JSON::Object(ref obj) = serialized {
    ///   let serialized = obj.get("TimeStamp").unwrap();
    ///   assert!(serialized.as_string().unwrap().starts_with("2014-11-28"));
    /// } else {
    ///   panic!();
    /// }
    /// # }
    /// ```
    TimeStamp(TimeStamp),

    /// A duration, also used to represent a time of day.
    ///
    /// # JSON
    ///
    /// Represented by `{Duration: float}`, where the number, is a (floating-point)
    /// number of seconds. If this value use used for time of day, the duration is
    /// since the start of the day, in local time.
    ///
    /// ```
    /// extern crate foxbox_taxonomy;
    /// extern crate chrono;
    ///
    /// use foxbox_taxonomy::values::*;
    /// use foxbox_taxonomy::parse::*;
	/// use foxbox_taxonomy::serialize::*;
    /// use chrono::Duration as ChronoDuration;
    ///
    /// # fn main() {
    ///
    /// let parsed = Value::from_str("{\"Duration\": 60.01}").unwrap();
    /// if let Value::Duration(d) = parsed.clone() {
    ///   let duration : ChronoDuration = d.into();
    ///   assert_eq!(duration.num_seconds(), 60);
    ///   assert_eq!(duration.num_milliseconds(), 60010);
    /// } else {
    ///   panic!();
    /// }
    ///
    ///
    /// let serialized: JSON = parsed.to_json(&mut MultiPart::new());
    /// if let JSON::Object(ref obj) = serialized {
    ///   let serialized = obj.get("Duration").unwrap();
    ///   assert!(serialized.as_f64().unwrap() >= 60. && serialized.as_f64().unwrap() < 61.);
    /// } else {
    ///   panic!();
    /// }
    /// # }
    /// ```
    Duration(Duration),

    /// A temperature.
    ///
    /// # JSON
    ///
    /// Represented by `{Temperature: {C: float}}` or `{Temperature: {F: float}}`.
    ///
    /// ```
    /// extern crate foxbox_taxonomy;
    /// extern crate chrono;
    ///
    /// use foxbox_taxonomy::values::*;
    /// use foxbox_taxonomy::parse::*;
	/// use foxbox_taxonomy::serialize::*;
    ///
    /// # fn main() {
    ///
    /// let source = "{
    ///   \"Temperature\": {
    ///     \"C\": 2.0
    ///   }
    /// }";
    /// let parsed = Value::from_str(source).unwrap();
    /// if let Value::Temperature(Temperature::C(ref val)) = parsed {
    ///   assert_eq!(*val, 2.0);
    /// } else {
    ///   panic!();
    /// }
    ///
    ///
    /// let serialized: JSON = parsed.to_json(&mut MultiPart::new());
    /// let val = serialized.find_path(&["Temperature", "C"]).unwrap().as_f64().unwrap();
    /// assert_eq!(val, 2.0);
    /// # }
    /// ```
    Temperature(Temperature),

    /// A color.
    ///
    /// # JSON
    ///
    /// Represented by `{Color: {r: float, g: float, b: float, a: float}}` where each
    /// of `r`, `g`, `b`, `a` is in [0, 1]. Value `a` can be omitted, in which case it
    /// is assumed to be 0.
    ///
    /// ```
    /// extern crate foxbox_taxonomy;
    /// extern crate chrono;
    ///
    /// use foxbox_taxonomy::values::*;
    /// use foxbox_taxonomy::parse::*;
	/// use foxbox_taxonomy::serialize::*;
    ///
    /// # fn main() {
    ///
    /// let source = "{
    ///   \"Color\": {
    ///     \"r\": 0.1,
    ///     \"g\": 0.2,
    ///     \"b\": 0.4
    ///   }
    /// }";
    /// let parsed = Value::from_str(source).unwrap();
    /// if let Value::Color(Color::RGBA(0.1, 0.2, 0.4, 0.0)) = parsed {
    ///   // Ok.
    /// } else {
    ///   panic!();
    /// }
    ///
    ///
    /// let serialized: JSON = parsed.to_json(&mut MultiPart::new());
    /// let val = serialized.find_path(&["Color", "g"]).unwrap().as_f64().unwrap();
    /// assert_eq!(val, 0.2);
    /// # }
    /// ```
    Color(Color),

    /// A string.
    ///
    /// # JSON
    ///
    /// Represented by `{String: string}`.
    ///
    /// ```
    /// extern crate foxbox_taxonomy;
    /// extern crate chrono;
    ///
    /// use foxbox_taxonomy::values::*;
    /// use foxbox_taxonomy::parse::*;
	/// use foxbox_taxonomy::serialize::*;
    ///
    /// # fn main() {
    ///
    /// let source = "{
    ///   \"String\": \"foobar\"
    /// }";
    /// let parsed = Value::from_str(source).unwrap();
    /// if let Value::String(ref str) = parsed {
    ///   assert_eq!(&*str as &str, "foobar");
    /// } else {
    ///   panic!();
    /// }
    ///
    ///
    /// let serialized: JSON = parsed.to_json(&mut MultiPart::new());
    /// let val = serialized.find_path(&["String"]).unwrap().as_string().unwrap();
    /// assert_eq!(&val as &str, "foobar");
    /// # }
    /// ```
    String(Arc<String>),

    // FIXME: Add more as we identify needs

    ThinkerbellRule(ThinkerbellRule),

    /// A boolean value representing a unit that has not been
    /// standardized yet into the API.
    ExtBool(ExtValue<bool>),

    /// A numeric value representing a unit that has not been
    /// standardized yet into the API.
    ExtNumeric(ExtValue<f64>),

    /// A Json value. We put it behind an `Arc` to make sure that
    /// cloning remains inexpensive.
    ///
    /// # JSON
    ///
    /// Represented by `{Json: JSON}` where `JSON` is a JSON object.
    ///
    /// ```
    /// extern crate foxbox_taxonomy;
    /// extern crate chrono;
    ///
    /// use foxbox_taxonomy::values::*;
    /// use foxbox_taxonomy::parse::*;
	/// use foxbox_taxonomy::serialize::*;
    ///
    /// # fn main() {
    ///
    /// let source = "{
    ///   \"Json\": { \"foo\": \"bar\" }
    /// }";
    /// let parsed = Value::from_str(source).unwrap();
    /// if let Value::Json(ref obj) = parsed {
    ///   assert_eq!(obj.0.find_path(&["foo"]).unwrap().as_string().unwrap(), "bar")
    /// } else {
    ///   panic!();
    /// }
    ///
    ///
    /// let serialized: JSON = parsed.to_json(&mut MultiPart::new());
    /// let val = serialized.find_path(&["Json", "foo"]).unwrap().as_string().unwrap();
    /// assert_eq!(val, "bar");
    /// # }
    /// ```
    Json(Arc<Json>),

    /// Binary data.
    ///
    /// # JSON
    ///
    /// Represented by `{Binary: {part: number}}`, where `number` represents
    /// the number of the multipart containing the binary payload.
    /// ```
    Binary(Binary),
}


lazy_static! {
    static ref VALUE_PARSER:
        HashMap<&'static str, Box<Fn(Path, &mut JSON) -> Result<Value, ParseError> + Sync>> =
    {
        use self::Value::*;
        use std::string::String as StdString;
        let mut map : HashMap<&'static str, Box<Fn(Path, &mut JSON) -> Result<Value, ParseError> + Sync>> = HashMap::new();
        map.insert("Unit", Box::new(|_, _| Ok(Unit)));
        map.insert("OnOff", Box::new(|path, v| {
            let value = try!(path.push("OnOff", |path| self::OnOff::parse(path, v)));
            Ok(OnOff(value))
        }));
        map.insert("OpenClosed", Box::new(|path, v| {
            let value = try!(path.push("OpenClosed", |path| self::OpenClosed::parse(path, v)));
            Ok(OpenClosed(value))
        }));
        map.insert("Duration", Box::new(|path, v| {
            let value = try!(path.push("Duration", |path| self::Duration::parse(path, v)));
            Ok(Duration(value))
        }));
        map.insert("TimeStamp", Box::new(|path, v| {
            let value = try!(path.push("TimeStamp", |path| self::TimeStamp::parse(path, v)));
            Ok(TimeStamp(value))
        }));
        map.insert("Temperature", Box::new(|path, v| {
            let value = try!(path.push("Temperature", |path| self::Temperature::parse(path, v)));
            Ok(Temperature(value))
        }));
        map.insert("ThinkerbellRule", Box::new(|path, v| {
            let value = try!(path.push("ThinkerbellRule", |path| self::ThinkerbellRule::parse(path, v)));
            Ok(ThinkerbellRule(value))
        }));
        map.insert("Color", Box::new(|path, v| {
            let value = try!(path.push("Color", |path| self::Color::parse(path, v)));
            Ok(Color(value))
        }));
        map.insert("String", Box::new(|path, v| {
            let value = try!(path.push("String", |path| Arc::<StdString>::parse(path, v)));
            Ok(String(value))
        }));
        map.insert("Json", Box::new(|path, v| {
            let value = try!(path.push("Json", |path| Arc::<self::Json>::parse(path, v)));
            Ok(Json(value))
        }));
        map.insert("ExtBool", Box::new(|path, v| {
            let value = try!(path.push("ExtBool", |path| self::ExtValue::<bool>::parse(path, v)));
            Ok(ExtBool(value))
        }));
        map.insert("ExtNumeric", Box::new(|path, v| {
            let value = try!(path.push("ExtNumeric", |path| self::ExtValue::<f64>::parse(path, v)));
            Ok(ExtNumeric(value))
        }));
        map.insert("Binary", Box::new(|path, v| {
            let value = try!(path.push("Binary", |path| self::Binary::parse(path, v)));
            Ok(Binary(value))
        }));
        map
    };
    static ref VALUE_KEYS: String = {
        let vec : Vec<_> = VALUE_PARSER.keys().cloned().collect();
        format!("{:?}", vec)
    };
}

impl Parser<Value> for Value {
    fn description() -> String {
        "Value".to_owned()
    }
    fn parse(path: Path, source: &mut JSON) -> Result<Self, ParseError> {
        match *source {
            JSON::Null => Ok(Value::Unit),
            JSON::String(ref str) if &*str == "Unit" => Ok(Value::Unit),
            JSON::Object(ref mut obj) if obj.len() == 1 => {
                let mut vec : Vec<_> = obj.iter_mut().collect();
                let (k, v) = vec.pop().unwrap(); // We checked the length just above.
                match VALUE_PARSER.get(&k as &str) {
                    None => Err(ParseError::type_error("Value", &path, &&*self::VALUE_KEYS)),
                    Some(parser) => path.push(k, |path| parser(path, v))
                }
            }
            _ => Err(ParseError::type_error("Value", &path, "object with a single field"))
        }
    }
}

impl ToJSON for Value {
    fn to_json(&self, parts: &mut BinaryParts) -> JSON {
        use self::Value::*;
        let obj = match *self {
            Unit => ("Unit", JSON::Null),
            OnOff(ref val) => ("OnOff", val.to_json(parts)),
            OpenClosed(ref val) => ("OpenClosed", val.to_json(parts)),
            Duration(ref val) => ("Duration", val.to_json(parts)),
            TimeStamp(ref val) => ("TimeStamp", val.to_json(parts)),
            Color(ref val) => ("Color", val.to_json(parts)),
            String(ref val) => ("String", val.to_json(parts)),
            Json(ref val) => ("Json", val.to_json(parts)),
            Binary(ref val) => ("Binary", val.to_json(parts)),
            Temperature(ref val) => ("Temperature", val.to_json(parts)),
            ThinkerbellRule(ref val) => ("ThinkerbellRule", val.to_json(parts)),
            ExtBool(ref val) => ("ExtBool", val.to_json(parts)),
            ExtNumeric(ref val) => ("ExtNumeric", val.to_json(parts)),
        };
        vec![obj].to_json(parts)
    }
}


impl Value {
    pub fn get_type(&self) -> Type {
        match *self {
            Value::Unit => Type::Unit,
            Value::OnOff(_) => Type::OnOff,
            Value::OpenClosed(_) => Type::OpenClosed,
            Value::String(_) => Type::String,
            Value::Duration(_) => Type::Duration,
            Value::TimeStamp(_) => Type::TimeStamp,
            Value::Temperature(_) => Type::Temperature,
            Value::Color(_) => Type::Color,
            Value::Json(_) => Type::Json,
            Value::Binary(_) => Type::Binary,
            Value::ExtBool(_) => Type::ExtBool,
            Value::ExtNumeric(_) => Type::ExtNumeric,
            Value::ThinkerbellRule(_) => Type::ThinkerbellRule,
        }
    }

    pub fn as_timestamp(&self) -> Result<&TimeStamp, TypeError> {
        match *self {
            Value::TimeStamp(ref x) => Ok(x),
            _ => Err(TypeError {expected: Type::TimeStamp, got: self.get_type()})
        }
    }

    pub fn as_duration(&self) -> Result<&Duration, TypeError> {
        match *self {
            Value::Duration(ref x) => Ok(x),
            _ => Err(TypeError {expected: Type::Duration, got: self.get_type()})
        }
    }
}

impl PartialOrd for Value {
    /// Two values of the same type can be compared using the usual
    /// comparison for values of this type. Two values of distinct
    /// types cannot be compared.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        use self::Value::*;
        use std::cmp::Ordering::*;
        match (self, other) {
            (&Unit, &Unit) => Some(Equal),
            (&Unit, _) => None,

            (&OnOff(ref a), &OnOff(ref b)) => a.partial_cmp(b),
            (&OnOff(_), _) => None,

            (&OpenClosed(ref a), &OpenClosed(ref b)) => a.partial_cmp(b),
            (&OpenClosed(_), _) => None,

            (&Duration(ref a), &Duration(ref b)) => a.partial_cmp(b),
            (&Duration(_), _) => None,

            (&TimeStamp(ref a), &TimeStamp(ref b)) => a.partial_cmp(b),
            (&TimeStamp(_), _) => None,

            (&Temperature(ref a), &Temperature(ref b)) => a.partial_cmp(b),
            (&Temperature(_), _) => None,

            (&Color(ref a), &Color(ref b)) => a.partial_cmp(b),
            (&Color(_), _) => None,

            (&ExtBool(ref a), &ExtBool(ref b)) => a.partial_cmp(b),
            (&ExtBool(_), _) => None,

            (&ExtNumeric(ref a), &ExtNumeric(ref b)) => a.partial_cmp(b),
            (&ExtNumeric(_), _) => None,

            (&String(ref a), &String(ref b)) => a.partial_cmp(b),
            (&String(_), _) => None,

            (&Json(ref a), &Json(ref b)) => a.partial_cmp(b),
            (&Json(_), _) => None,

            (&ThinkerbellRule(ref a), &ThinkerbellRule(ref b)) => a.name.partial_cmp(&b.name),
            (&ThinkerbellRule(_), _) => None,

            (&Binary(self::Binary {mimetype: ref a_mimetype, data: ref a_data}),
             &Binary(self::Binary {mimetype: ref b_mimetype, data: ref b_data})) if a_mimetype == b_mimetype => a_data.partial_cmp(b_data),
            (&Binary(_), _) => None,
        }
    }
}

/// An absolute time and date.
///
/// # JSON
///
/// Represented by a string. This data structure accepts string formatted as RFC 3339 such as
/// `"2014-11-28T21:45:59.324310806+09:00"`.
///
/// ```
/// extern crate chrono;
/// extern crate foxbox_taxonomy;
///
/// use foxbox_taxonomy::values::*;
/// use foxbox_taxonomy::parse::*;
	/// use foxbox_taxonomy::serialize::*;
/// use chrono::Datelike;
///
/// # fn main() {
///
/// let parsed = TimeStamp::from_str("\"2014-11-28T21:45:59.324310806+09:00\"").unwrap();
/// let date_time = parsed.as_datetime().clone();
/// assert_eq!(date_time.year(), 2014);
/// assert_eq!(date_time.month(), 11);
/// assert_eq!(date_time.day(), 28);
///
///
/// let serialized: JSON = parsed.to_json(&mut MultiPart::new());
/// assert!(serialized.as_string().unwrap().starts_with("2014-11-28"));
///
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct TimeStamp(DateTime<UTC>);
impl TimeStamp {
    pub fn from_datetime(datetime: DateTime<UTC>) -> Self {
        TimeStamp(datetime)
    }
    pub fn as_datetime(&self) -> &DateTime<UTC> {
        &self.0
    }
    pub fn from_s(s: i64) -> Self {
        use chrono;
        let naive = chrono::naive::datetime::NaiveDateTime::from_timestamp(s, 0);
        let date = DateTime::<UTC>::from_utc(naive, UTC);
        TimeStamp(date)
    }
}
impl Parser<TimeStamp> for TimeStamp {
    fn description() -> String {
        "TimeStamp".to_owned()
    }
    fn parse(path: Path, source: &mut JSON) -> Result<Self, ParseError> {
        if let JSON::String(ref str) = *source {
            if let Ok(dt) = DateTime::<UTC>::from_str(str) {
                return Ok(TimeStamp(dt));
            }
        }
        Err(ParseError::type_error("TimeStamp", &path, "date string"))
    }
}
impl ToJSON for TimeStamp {
    fn to_json(&self, parts: &mut BinaryParts) -> JSON {
        self.0.to_rfc3339().to_json(parts)
    }
}
impl Into<DateTime<UTC>> for TimeStamp  {
    fn into(self) -> DateTime<UTC> {
        self.0
    }
}
impl Into<DateTime<Local>> for TimeStamp  {
    fn into(self) -> DateTime<Local> {
        self.0.with_timezone(&Local)
    }
}
impl<T> From<DateTime<T>> for TimeStamp where T: TimeZone {
    fn from(date: DateTime<T>) -> Self {
        TimeStamp(date.with_timezone(&UTC))
    }
}

impl Serialize for TimeStamp {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer {
        let str = self.0.to_rfc3339();
        str.serialize(serializer)
    }
}
impl Deserialize for TimeStamp {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: Deserializer {
        let str = try!(String::deserialize(deserializer));
        match DateTime::<UTC>::from_str(&str) {
            Ok(dt) => Ok(TimeStamp(dt)),
            Err(_) => Err(D::Error::syntax("Invalid date"))
        }
    }
}

/// A comparison between two values.
///
/// # JSON
///
/// A range is an object with one field `{key: value}`.
///
#[derive(Clone, Deserialize, Serialize, Debug, PartialEq)]
pub enum Range {
    /// Leq(x) accepts any value v such that v <= x.
    ///
    /// # JSON
    ///
    /// ```
    /// extern crate foxbox_taxonomy;
    /// extern crate serde_json;
    ///
    /// use foxbox_taxonomy::values::*;
    /// use foxbox_taxonomy::parse::*;
	/// use foxbox_taxonomy::serialize::*;
    ///
    /// # fn main() {
    ///
    /// let source = "{
    ///   \"Leq\": { \"OnOff\": \"On\" }
    /// }";
    ///
    /// let parsed = Range::from_str(source).unwrap();
    /// if let Range::Leq(ref leq) = parsed {
    ///   assert_eq!(*leq, Value::OnOff(OnOff::On));
    /// } else {
    ///   panic!();
    /// }
    ///
    /// let as_json = parsed.to_json(&mut MultiPart::new());
    /// let as_string = serde_json::to_string(&as_json).unwrap();
    /// assert_eq!(as_string, "{\"Leq\":{\"OnOff\":\"On\"}}");
    ///
    /// # }
    /// ```
    Leq(Value),

    /// Geq(x) accepts any value v such that v >= x.
    Geq(Value),

    /// BetweenEq {min, max} accepts any value v such that `min <= v`
    /// and `v <= max`. If `max < min`, it never accepts anything.
    BetweenEq { min:Value, max:Value },

    /// OutOfStrict {min, max} accepts any value v such that `v < min`
    /// or `max < v`
    OutOfStrict { min:Value, max:Value },

    /// Eq(x) accespts any value v such that v == x
    Eq(Value),
}

impl Parser<Range> for Range {
    fn description() -> String {
        "Range".to_owned()
    }
    fn parse(path: Path, source: &mut JSON) -> Result<Self, ParseError> {
        use self::Range::*;
        match *source {
            JSON::Object(ref mut obj) if obj.len() == 1 => {
                if let Some(leq) = obj.get_mut("Leq") {
                    return Ok(Leq(try!(path.push("Leq", |path| Value::parse(path, leq)))))
                }
                if let Some(geq) = obj.get_mut("Geq") {
                    return Ok(Geq(try!(path.push("Geq", |path| Value::parse(path, geq)))))
                }
                if let Some(eq) = obj.get_mut("Eq") {
                    return Ok(Eq(try!(path.push("eq", |path| Value::parse(path, eq)))))
                }
                if let Some(between) = obj.get_mut("BetweenEq") {
                    let mut bounds = try!(path.push("BetweenEq", |path| Vec::<Value>::parse(path, between)));
                    if bounds.len() == 2 {
                        let max = bounds.pop().unwrap();
                        let min = bounds.pop().unwrap();
                        return Ok(BetweenEq {
                            min: min,
                            max: max
                        })
                    } else {
                        return Err(ParseError::type_error("BetweenEq", &path, "an array of two values"))
                    }
                }
                if let Some(outof) = obj.get_mut("OutOfStrict") {
                    let mut bounds = try!(path.push("OutOfStrict", |path| Vec::<Value>::parse(path, outof)));
                    if bounds.len() == 2 {
                        let max = bounds.pop().unwrap();
                        let min = bounds.pop().unwrap();
                        return Ok(OutOfStrict {
                            min: min,
                            max: max
                        })
                    } else {
                        return Err(ParseError::type_error("OutOfStrict", &path, "an array of two values"))
                    }
                }
                Err(ParseError::type_error("Range", &path, "a field Eq, Leq, Geq, BetweenEq or OutOfStrict"))
            }
            _ => Err(ParseError::type_error("Range", &path, "object"))
        }
    }
}

impl ToJSON for Range {
    fn to_json(&self, parts: &mut BinaryParts) -> JSON {
        let obj = match *self {
            Range::Eq(ref val) => ("Eq", val.to_json(parts)),
            Range::Geq(ref val) => ("Geq", val.to_json(parts)),
            Range::Leq(ref val) => ("Leq", val.to_json(parts)),
            Range::BetweenEq { ref min, ref max } => ("BetweenEq", JSON::Array(vec![min.to_json(parts), max.to_json(parts)])),
            Range::OutOfStrict { ref min, ref max } => ("OutOfStrict", JSON::Array(vec![min.to_json(parts), max.to_json(parts)])),
        };
        vec![obj].to_json(parts)
    }
}

impl Range {
    /// Determine if a value is accepted by this range.
    pub fn contains(&self, value: &Value) -> bool {
        use self::Range::*;
        match *self {
            Leq(ref max) => value <= max,
            Geq(ref min) => value >= min,
            BetweenEq { ref min, ref max } => min <= value && value <= max,
            OutOfStrict { ref min, ref max } => value < min || max < value,
            Eq(ref val) => value == val,
        }
    }

    /// Get the type associated to this range.
    ///
    /// If this range has a `min` and a `max` with conflicting types,
    /// produce an error.
    pub fn get_type(&self) -> Result<Type, TypeError> {
        use self::Range::*;
        match *self {
            Leq(ref v) | Geq(ref v) | Eq(ref v) => Ok(v.get_type()),
            BetweenEq {ref min, ref max} | OutOfStrict {ref min, ref max} => {
                let min_typ = min.get_type();
                let max_typ = max.get_type();
                if min_typ == max_typ {
                    Ok(min_typ)
                } else {
                    Err(TypeError {
                        expected: min_typ,
                        got: max_typ
                    })
                }
            }
        }
    }
}


/// A duration, also used to represent a time of day.
///
/// # JSON
///
/// Represented by a (floating-point) number of seconds.
///
/// ```
/// extern crate foxbox_taxonomy;
/// extern crate chrono;
///
/// use foxbox_taxonomy::values::*;
/// use foxbox_taxonomy::parse::*;
	/// use foxbox_taxonomy::serialize::*;
/// use chrono::Duration as ChronoDuration;
///
/// # fn main() {
///
/// let parsed = Duration::from_str("60.01").unwrap();
/// let duration : ChronoDuration = parsed.clone().into();
/// assert_eq!(duration.num_seconds(), 60);
/// assert_eq!(duration.num_milliseconds(), 60010);
///
///
/// let serialized: JSON = parsed.to_json(&mut MultiPart::new());
/// assert_eq!(serialized.as_f64().unwrap(), 60.01);
///
/// # }
/// ```
#[derive(Clone, Debug, PartialOrd, Ord, PartialEq, Eq)]
pub struct Duration(ChronoDuration);

impl Parser<Duration> for Duration {
    fn description() -> String {
        "Duration".to_owned()
    }
    fn parse(path: Path, source: &mut JSON) -> Result<Self, ParseError> {
        let val = try!(f64::parse(path, source));
        Ok(Duration(ChronoDuration::milliseconds((val * 1000.) as i64)))
    }
}

impl ToJSON for Duration {
    fn to_json(&self, parts: &mut BinaryParts) -> JSON {
        let val = self.0.num_milliseconds() as f64 / 1000 as f64;
        val.to_json(parts)
    }
}

impl Into<Value> for Duration {
    fn into(self) -> Value {
        Value::Duration(self)
    }
}

///
/// # Serialization
///
/// Values are deserialized to a floating-point number of seconds.
///
/// ```
/// extern crate serde;
/// extern crate serde_json;
/// extern crate foxbox_taxonomy;
/// extern crate chrono;
///
/// # fn main() {
/// use foxbox_taxonomy::values::*;
///
/// let duration = Duration::from(chrono::Duration::milliseconds(3141));
///
/// let duration_as_json = serde_json::to_string(&duration).unwrap();
/// assert_eq!(duration_as_json, "3.141");
///
/// let duration_back : Duration = serde_json::from_str(&duration_as_json).unwrap();
/// assert_eq!(duration, duration_back);
/// # }
/// ```
impl Serialize for Duration {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer
     {
         serializer.visit_f64(self.0.num_milliseconds() as f64 / 1000 as f64)
     }
}
impl From<ChronoDuration> for Duration {
    fn from(source: ChronoDuration) -> Self {
        Duration(source)
    }
}
impl Into<ChronoDuration> for Duration {
    fn into(self) -> ChronoDuration {
        self.0
    }
}

impl Deserialize for Duration {
    /// Deserialize this value given this `Deserializer`.
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: Deserializer
    {
        struct DurationVisitor;
        impl DeserializationVisitor for DurationVisitor
        {
            type Value = Duration;
            fn visit_f64<E>(&mut self, v: f64) -> Result<Self::Value, E>
                where E: Error,
            {
                Ok(Duration(ChronoDuration::milliseconds((v * 1000.) as i64)))
            }
            fn visit_i64<E>(&mut self, v: i64) -> Result<Self::Value, E>
                where E: Error,
            {
                Ok(Duration(ChronoDuration::milliseconds(v * 1000)))
            }
            fn visit_u64<E>(&mut self, v: u64) -> Result<Self::Value, E>
                where E: Error,
            {
                self.visit_i64(v as i64)
            }
        }
        deserializer.visit_f64(DurationVisitor)
            .or_else(|_| deserializer.visit_i64(DurationVisitor))
    }
}
