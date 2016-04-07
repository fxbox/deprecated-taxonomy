//! Selectors for services and channels.
//!
//! The high-level API of Project Link always offers access by selectors, rather than by individual
//! services/channels. This allows operations such as sending a temperature to all heaters in the
//! living room (that's a selector), rather than needing to access every single heater one by one.

pub use parse::*;
use services::{ Service, ChannelKind, Channel, Getter, Setter };
use util::*;
use values::Duration;

use std::cmp;
use std::hash::Hash;
use std::collections::HashSet;

fn merge<T>(mut a: HashSet<T>, b: Vec<T>) -> HashSet<T> where T: Hash + Eq {
    for x in b {
        a.insert(x);
    }
    a
}

pub trait SelectedBy<T> {
    fn matches(&self, &T) -> bool;
}

/// A trait used to let `ServiceSelector` work on complex data structures
/// that are not necessarily exactly Selector.
pub trait ServiceLike {
    fn id(&self) -> &Id<ServiceId>;
    fn adapter(&self) -> &Id<AdapterId>;
    fn with_tags<F>(&self, f: F) -> bool where F: Fn(&HashSet<Id<TagId>>) -> bool;
    fn has_getters<F>(&self, f: F) -> bool where F: Fn(&Channel<Getter>) -> bool;
    fn has_setters<F>(&self, f: F) -> bool where F: Fn(&Channel<Setter>) -> bool;
}

impl ServiceLike for Service {
    fn id(&self) -> &Id<ServiceId> {
        &self.id
    }
    fn adapter(&self) -> &Id<AdapterId> {
        &self.adapter
    }
    fn with_tags<F>(&self, f: F) -> bool where F: Fn(&HashSet<Id<TagId>>) -> bool {
        f(&self.tags)
    }
    fn has_getters<F>(&self, f: F) -> bool where F: Fn(&Channel<Getter>) -> bool {
        for chan in self.getters.values() {
            if f(chan) {
                return true;
            }
        }
        false
    }
    fn has_setters<F>(&self, f: F) -> bool where F: Fn(&Channel<Setter>) -> bool {
        for chan in self.setters.values() {
            if f(chan) {
                return true;
            }
        }
        false
    }

}

/// A selector for one or more services.
///
///
/// # Example
///
/// ```
/// use foxbox_taxonomy::selector::*;
/// use foxbox_taxonomy::services::*;
///
/// let selector = ServiceSelector::new()
///   .with_tags(vec![Id::<TagId>::new("entrance")])
///   .with_getters(vec![GetterSelector::new() /* can be more restrictive */]);
/// ```
///
/// # JSON
///
/// A selector is an object with the following fields:
///
/// - (optional) string `id`: accept only a service with a given id;
/// - (optional) array of string `tags`:  accept only services with all the tags in the array;
/// - (optional) array of objects `getters` (see `GetterSelector`): accept only services with
///    channels matching all the selectors in this array;
/// - (optional) array of objects `setters` (see `SetterSelector`): accept only services with
///    channels matching all the selectors in this array;
///
/// While each field is optional, at least one field must be provided.
///
/// ```
/// use foxbox_taxonomy::selector::*;
///
/// // A selector with all fields defined.
/// let json_selector = "{
///   \"id\": \"setter 1\",
///   \"tags\": [\"tag 1\", \"tag 2\"],
///   \"getters\": [{
///     \"kind\": \"Ready\"
///   }],
///   \"setters\": [{
///     \"tags\": [\"tag 3\"]
///   }]
/// }";
///
/// ServiceSelector::from_str(json_selector).unwrap();
///
/// // The following will be rejected because no field is provided:
/// let json_empty = "{}";
/// match ServiceSelector::from_str(json_empty) {
///   Err(ParseError::EmptyObject {..}) => { /* as expected */ },
///   other => panic!("Unexpected result {:?}", other)
/// }
/// ```
#[derive(Clone, Debug, Deserialize, Default)]
pub struct ServiceSelector {
    /// If `Exactly(id)`, return only the service with the corresponding id.
    pub id: Exactly<Id<ServiceId>>,

    ///  Restrict results to services that have all the tags in `tags`.
    pub tags: HashSet<Id<TagId>>,

    /// Restrict results to services that have all the getters in `getters`.
    pub getters: Vec<GetterSelector>,

    /// Restrict results to services that have all the setters in `setters`.
    pub setters: Vec<SetterSelector>,

    /// Make sure that we can't instantiate from another crate.
    private: (),
}

impl Parser<ServiceSelector> for ServiceSelector {
    fn description() -> String {
        "ServiceSelector".to_owned()
    }
    fn parse(path: Path, source: &mut JSON) -> Result<Self, ParseError> {
        let mut is_empty = true;
        let id = try!(match path.push("id", |path| Exactly::take_opt(path, source, "id")) {
            None => Ok(Exactly::Always),
            Some(result) => {
                is_empty = false;
                result
            }
        });
        let tags : HashSet<_> = match path.push("tags", |path| Id::take_vec_opt(path, source, "tags")) {
            None => HashSet::new(),
            Some(Ok(mut vec)) => {
                is_empty = false;
                vec.drain(..).collect()
            }
            Some(Err(err)) => return Err(err),
        };
        let getters = match path.push("getters", |path| GetterSelector::take_vec_opt(path, source, "getters")) {
            None => vec![],
            Some(Ok(vec)) => {
                is_empty = false;
                vec
            }
            Some(Err(err)) => return Err(err)
        };
        let setters = match path.push("setters", |path| SetterSelector::take_vec_opt(path, source, "setters")) {
            None => vec![],
            Some(Ok(vec)) => {
                is_empty = false;
                vec
            }
            Some(Err(err)) => return Err(err)
        };

        if is_empty {
            Err(ParseError::empty_object(&path))
        } else {
            Ok(ServiceSelector {
                id: id,
                tags: tags,
                getters: getters,
                setters: setters,
                private: ()
            })
        }
    }
}

impl ServiceSelector {
    /// Create a new selector that accepts all services.
    pub fn new() -> Self {
        Self::default()
    }

    /// Selector for a service with a specific id.
    pub fn with_id(self, id: Id<ServiceId>) -> Self {
        ServiceSelector {
            id: self.id.and(Exactly::Exactly(id)),
            .. self
        }
    }

    ///  Restrict results to services that have all the tags in `tags`.
    pub fn with_tags(self, tags: Vec<Id<TagId>>) -> Self {
        ServiceSelector {
            tags: merge(self.tags, tags),
            .. self
        }
    }

    /// Restrict results to services that have all the getters in `getters`.
    pub fn with_getters(mut self, mut getters: Vec<GetterSelector>) -> Self {
        ServiceSelector {
            getters: {self.getters.append(&mut getters); self.getters},
            .. self
        }
    }

    /// Restrict results to services that have all the setters in `setters`.
    pub fn with_setters(mut self, mut setters: Vec<SetterSelector>) -> Self {
        ServiceSelector {
            setters: {self.setters.append(&mut setters); self.setters},
            .. self
        }
    }

    /// Restrict results to services that are accepted by two selector.
    pub fn and(mut self, mut other: ServiceSelector) -> Self {
        ServiceSelector {
            id: self.id.and(other.id),
            tags: self.tags.union(&other.tags).cloned().collect(),
            getters: {self.getters.append(&mut other.getters); self.getters},
            setters: {self.setters.append(&mut other.setters); self.setters},
            private: (),
        }
    }

    pub fn matches<T>(&self, service: &T) -> bool
        where T: ServiceLike
    {
        if !self.id.matches(service.id()) {
            return false;
        }
        if !service.with_tags(|tags| has_selected_tags(&self.tags, tags)) {
            return false;
        }
        // If any of the getter selectors doesn't find a getter,
        // we don't match.
        let getters_fail = self.getters.iter().any(|selector| {
            !service.has_getters(|channel| {
                selector.matches(&self.tags, channel)
            })
        });
        if getters_fail {
            return false;
        }
        // If any of the setter selectors doesn't find a setter,
        // we don't match.
        let setters_fail = self.setters.iter().any(|selector| {
            !service.has_setters(|channel| {
                selector.matches(&self.tags, channel)
            })
        });
        if setters_fail {
            return false;
        }
        true
    }
}

impl SelectedBy<ServiceSelector> for Service {
    fn matches(&self, selector: &ServiceSelector) -> bool {
        selector.matches(self)
    }
}


/// A selector for one or more getter channels.
///
///
/// # Example
///
/// ```
/// use foxbox_taxonomy::selector::*;
/// use foxbox_taxonomy::services::*;
///
/// let selector = GetterSelector::new()
///   .with_parent(Id::new("foxbox"))
///   .with_kind(ChannelKind::CurrentTimeOfDay);
/// ```
///
/// # JSON
///
/// A selector is an object with the following fields:
///
/// - (optional) string `id`: accept only a channel with a given id;
/// - (optional) string `service`: accept only channels of a service with a given id;
/// - (optional) array of string `tags`:  accept only channels with all the tags in the array;
/// - (optional) array of string `service_tags`:  accept only channels of a service with all the
///        tags in the array;
/// - (optional) string|object `kind` (see `ChannelKind`): accept only channels of a given kind.
///
/// While each field is optional, at least one field must be provided.
///
/// ```
/// use foxbox_taxonomy::selector::*;
///
/// // A selector with all fields defined.
/// let json_selector = "{                         \
///   \"id\": \"setter 1\",                        \
///   \"service\": \"service 1\",                  \
///   \"tags\": [\"tag 1\", \"tag 2\"],            \
///   \"service_tags\": [\"tag 3\", \"tag 4\"],    \
///   \"kind\": \"Ready\"                          \
/// }";
///
/// GetterSelector::from_str(json_selector).unwrap();
///
/// // The following will be rejected because no field is provided:
/// let json_empty = "{}";
/// match GetterSelector::from_str(json_empty) {
///   Err(ParseError::EmptyObject {..}) => { /* as expected */ },
///   other => panic!("Unexpected result {:?}", other)
/// }
/// ```
#[derive(Clone, Debug, Deserialize, Default)]
pub struct GetterSelector {
    /// If `Exactly(id)`, return only the channel with the corresponding id.
    pub id: Exactly<Id<Getter>>,

    /// If `Eactly(id)`, return only channels that are children of
    /// service `id`.
    pub parent: Exactly<Id<ServiceId>>,

    ///  Restrict results to channels that have all the tags in `tags`.
    pub tags: HashSet<Id<TagId>>,

    ///  Restrict results to channels offered by a service that has all the tags in `tags`.
    pub service_tags: HashSet<Id<TagId>>,

    /// If `Exatly(k)`, restrict results to channels that produce values
    /// of kind `k`.
    pub kind: Exactly<ChannelKind>,

    /// Make sure that we can't instantiate from another crate.
    private: (),
}

impl Parser<GetterSelector> for GetterSelector {
    fn description() -> String {
        "GetterSelector".to_owned()
    }
    fn parse(path: Path, source: &mut JSON) -> Result<Self, ParseError> {
        let mut is_empty = true;
        let id = try!(match path.push("id", |path| Exactly::take_opt(path, source, "id")) {
            None => Ok(Exactly::Always),
            Some(result) => {
                is_empty = false;
                result
            }
        });
        let service_id = try!(match path.push("service", |path| Exactly::take_opt(path, source, "service")) {
            None => Ok(Exactly::Always),
            Some(result) => {
                is_empty = false;
                result
            }
        });
        let tags : HashSet<_> = match path.push("tags", |path| Id::take_vec_opt(path, source, "tags")) {
            None => HashSet::new(),
            Some(Ok(mut vec)) => {
                is_empty = false;
                vec.drain(..).collect()
            }
            Some(Err(err)) => return Err(err),
        };
        let service_tags : HashSet<_> = match path.push("service_tags", |path| Id::take_vec_opt(path, source, "service_tags")) {
            None => HashSet::new(),
            Some(Ok(mut vec)) => {
                is_empty = false;
                vec.drain(..).collect()
            }
            Some(Err(err)) => return Err(err),
        };
        let kind = try!(match path.push("kind", |path| Exactly::take_opt(path, source, "kind")) {
            None => Ok(Exactly::Always),
            Some(result) => {
                is_empty = false;
                result
            }
        });
        if is_empty {
            Err(ParseError::empty_object(&path))
        } else {
            Ok(GetterSelector {
                id: id,
                parent: service_id,
                tags: tags,
                service_tags: service_tags,
                kind: kind,
                private: ()
            })
        }
    }
}
impl GetterSelector {
    /// Create a new selector that accepts all getter channels.
    pub fn new() -> Self {
        Self::default()
    }

    /// Restrict to a channel with a specific id.
    pub fn with_id(self, id: Id<Getter>) -> Self {
        GetterSelector {
            id: self.id.and(Exactly::Exactly(id)),
            .. self
        }
    }

    /// Restrict to a channel with a specific parent.
    pub fn with_parent(self, id: Id<ServiceId>) -> Self {
        GetterSelector {
            parent: self.parent.and(Exactly::Exactly(id)),
            .. self
        }
    }

    /// Restrict to a channel with a specific kind.
    pub fn with_kind(self, kind: ChannelKind) -> Self {
        GetterSelector {
            kind: self.kind.and(Exactly::Exactly(kind)),
            .. self
        }
    }

    ///  Restrict to channels that have all the tags in `tags`.
    pub fn with_tags(self, tags: Vec<Id<TagId>>) -> Self {
        GetterSelector {
            tags: merge(self.tags, tags),
            .. self
        }
    }

    ///  Restrict to channels offered by a service that has all the tags in `tags`.
    pub fn with_service_tags(self, tags: Vec<Id<TagId>>) -> Self {
        GetterSelector {
            service_tags: merge(self.service_tags, tags),
            .. self
        }
    }

    /// Restrict to channels that are accepted by two selector.
    pub fn and(self, other: Self) -> Self {
        GetterSelector {
            id: self.id.and(other.id),
            parent: self.parent.and(other.parent),
            tags: self.tags.union(&other.tags).cloned().collect(),
            service_tags: self.service_tags.union(&other.service_tags).cloned().collect(),
            kind: self.kind.and(other.kind),
            private: (),
        }
    }

    /// Determine if a channel is matched by this selector.
    pub fn matches(&self, service_tags: &HashSet<Id<TagId>>, channel: &Channel<Getter>) -> bool {
        if !self.id.matches(&channel.id) {
            return false;
        }
        if !self.parent.matches(&channel.service) {
            return false;
        }
        if !self.kind.matches(&channel.mechanism.kind) {
            return false;
        }
        if !has_selected_tags(&self.tags, &channel.tags) {
            return false;
        }
        if !has_selected_tags(&self.service_tags, service_tags) {
            return false;
        }
        true
    }
}

/// A selector for one or more setter channels.
///
/// # JSON
///
/// A selector is an object with the following fields:
///
/// - (optional) string `id`: accept only a channel with a given id;
/// - (optional) string `service`: accept only channels of a service with a given id;
/// - (optional) array of string `tags`:  accept only channels with all the tags in the array;
/// - (optional) array of string `service_tags`:  accept only channels of a service with all the
///        tags in the array;
/// - (optional) string|object `kind` (see `ChannelKind`): accept only channels of a given kind.
///
/// While each field is optional, at least one field must be provided.
///
/// ```
/// use foxbox_taxonomy::selector::*;
///
/// // A selector with all fields defined.
/// let json_selector = "{                         \
///   \"id\": \"setter 1\",                        \
///   \"service\": \"service 1\",                  \
///   \"tags\": [\"tag 1\", \"tag 2\"],            \
///   \"service_tags\": [\"tag 3\", \"tag 4\"],    \
///   \"kind\": \"Ready\"                          \
/// }";
///
/// SetterSelector::from_str(json_selector).unwrap();
///
/// // The following will be rejected because no field is provided:
/// let json_empty = "{}";
/// match SetterSelector::from_str(json_empty) {
///   Err(ParseError::EmptyObject {..}) => { /* as expected */ },
///   other => panic!("Unexpected result {:?}", other)
/// }
/// ```
#[derive(Clone, Debug, Deserialize, Default)]
pub struct SetterSelector {
    /// If `Exactly(id)`, return only the channel with the corresponding id.
    pub id: Exactly<Id<Setter>>,

    /// If `Exactly(id)`, return only channels that are immediate children
    /// of service `id`.
    pub parent: Exactly<Id<ServiceId>>,

    ///  Restrict results to channels that have all the tags in `tags`.
    pub tags: HashSet<Id<TagId>>,

    ///  Restrict results to channels offered by a service that has all the tags in `tags`.
    pub service_tags: HashSet<Id<TagId>>,

    /// If `Exactly(k)`, restrict results to channels that accept values
    /// of kind `k`.
    pub kind: Exactly<ChannelKind>,

    /// Make sure that we can't instantiate from another crate.
    private: (),
}

impl Parser<SetterSelector> for SetterSelector {
    fn description() -> String {
        "SetterSelector".to_owned()
    }
    fn parse(path: Path, source: &mut JSON) -> Result<Self, ParseError> {
        let mut is_empty = true;
        let id = try!(match path.push("id", |path| Exactly::take_opt(path, source, "id")) {
            None => Ok(Exactly::Always),
            Some(result) => {
                is_empty = false;
                result
            }
        });
        let service_id = try!(match path.push("service", |path| Exactly::take_opt(path, source, "service")) {
            None => Ok(Exactly::Always),
            Some(result) => {
                is_empty = false;
                result
            }
        });
        let tags : HashSet<_> = match path.push("tags", |path| Id::take_vec_opt(path, source, "tags")) {
            None => HashSet::new(),
            Some(Ok(mut vec)) => {
                is_empty = false;
                vec.drain(..).collect()
            }
            Some(Err(err)) => return Err(err),
        };
        let service_tags : HashSet<_> = match path.push("service_tags", |path| Id::take_vec_opt(path, source, "service_tags")) {
            None => HashSet::new(),
            Some(Ok(mut vec)) => {
                is_empty = false;
                vec.drain(..).collect()
            }
            Some(Err(err)) => return Err(err),
        };
        let kind = try!(match path.push("kind", |path| Exactly::take_opt(path, source, "kind")) {
            None => Ok(Exactly::Always),
            Some(result) => {
                is_empty = false;
                result
            }
        });
        if is_empty {
            Err(ParseError::empty_object(&path))
        } else {
            Ok(SetterSelector {
                id: id,
                parent: service_id,
                tags: tags,
                service_tags: service_tags,
                kind: kind,
                private: ()
            })
        }
    }
}

impl SetterSelector {
    /// Create a new selector that accepts all getter channels.
    pub fn new() -> Self {
        SetterSelector::default()
    }

    /// Selector to a channel with a specific id.
    pub fn with_id(self, id: Id<Setter>) -> Self {
        SetterSelector {
            id: self.id.and(Exactly::Exactly(id)),
            .. self
        }
    }

    /// Selector to channels with a specific parent.
    pub fn with_parent(self, id: Id<ServiceId>) -> Self {
        SetterSelector {
            parent: self.parent.and(Exactly::Exactly(id)),
            .. self
        }
    }

    /// Selector to channels with a specific kind.
    pub fn with_kind(self, kind: ChannelKind) -> Self {
        SetterSelector {
            kind: self.kind.and(Exactly::Exactly(kind)),
            .. self
        }
    }

    ///  Restrict to channels that have all the tags in `tags`.
    pub fn with_tags(self, tags: Vec<Id<TagId>>) -> Self {
        SetterSelector {
            tags: merge(self.tags, tags),
            .. self
        }
    }

    ///  Restrict to channels offered by a service that has all the tags in `tags`.
    pub fn with_service_tags(self, tags: Vec<Id<TagId>>) -> Self {
        SetterSelector {
            service_tags: merge(self.service_tags, tags),
            .. self
        }
    }

    /// Restrict results to channels that are accepted by two selector.
    pub fn and(self, other: Self) -> Self {
        SetterSelector {
            id: self.id.and(other.id),
            parent: self.parent.and(other.parent),
            tags: self.tags.union(&other.tags).cloned().collect(),
            service_tags: self.service_tags.union(&other.service_tags).cloned().collect(),
            kind: self.kind.and(other.kind),
            private: (),
        }
    }

    /// Determine if a channel is matched by this selector.
    pub fn matches(&self, service_tags: &HashSet<Id<TagId>>, channel: &Channel<Setter>) -> bool {
        if !self.id.matches(&channel.id) {
            return false;
        }
        if !self.parent.matches(&channel.service) {
            return false;
        }
        if !self.kind.matches(&channel.mechanism.kind) {
            return false;
        }
        if !has_selected_tags(&self.tags, &channel.tags) {
            return false;
        }
        if !has_selected_tags(&self.service_tags, service_tags) {
            return false;
        }
        true
    }
}

/// An acceptable interval of time.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Period {
    #[serde(default)]
    pub min: Option<Duration>,
    #[serde(default)]
    pub max: Option<Duration>,
}
impl Period {
    pub fn and(self, other: Self) -> Self {
        let min = match (self.min, other.min) {
            (None, x) |
            (x, None) => x,
            (Some(min1), Some(min2)) => Some(cmp::max(min1, min2))
        };
        let max = match (self.max, other.max) {
            (None, x) |
            (x, None) => x,
            (Some(max1), Some(max2)) => Some(cmp::min(max1, max2))
        };
        Period {
            min: min,
            max: max
        }
    }

    pub fn and_option(a: Option<Self>, b: Option<Self>) -> Option<Self> {
        match (a, b) {
            (None, x) |
            (x, None) => x,
            (Some(a), Some(b)) => Some(a.and(b))
        }
    }

    pub fn matches(&self, duration: &Duration) -> bool {
        if let Some(ref min) = self.min {
            if min > duration {
                return false;
            }
        }
        if let Some(ref max) = self.max {
            if max < duration {
                return false;
            }
        }
        true
    }

    pub fn matches_option(period: &Option<Self>, duration: &Option<Duration>) -> bool {
        match (period, duration) {
            (&Some(ref period), &Some(ref duration))
                if !period.matches(duration) => false,
            (&Some(_), &None) => false,
            _ => true,
        }
    }
}


fn has_selected_tags(actual: &HashSet<Id<TagId>>, requested: &HashSet<Id<TagId>>) -> bool {
    for tag in &*actual {
        if !requested.contains(tag) {
            return false;
        }
    }
    true
}
