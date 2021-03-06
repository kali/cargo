use semver;
use std::hash::Hash;
use std::sync::Arc;
use std::fmt::{mod, Show, Formatter};
use std::hash;
use serialize::{Encodable, Encoder, Decodable, Decoder};

use regex::Regex;

use util::{CargoResult, CargoError, short_hash, ToSemver};
use core::source::SourceId;

/// Identifier for a specific version of a package in a specific source.
#[deriving(Clone)]
pub struct PackageId {
    inner: Arc<PackageIdInner>,
}

#[deriving(PartialEq, PartialOrd, Eq, Ord)]
struct PackageIdInner {
    name: String,
    version: semver::Version,
    source_id: SourceId,
}

impl<E, S: Encoder<E>> Encodable<S, E> for PackageId {
    fn encode(&self, s: &mut S) -> Result<(), E> {
        let source = self.inner.source_id.to_url();
        let encoded = format!("{} {} ({})", self.inner.name, self.inner.version,
                              source);
        encoded.encode(s)
    }
}

impl<E, D: Decoder<E>> Decodable<D, E> for PackageId {
    fn decode(d: &mut D) -> Result<PackageId, E> {
        let string: String = raw_try!(Decodable::decode(d));
        let regex = Regex::new(r"^([^ ]+) ([^ ]+) \(([^\)]+)\)$").unwrap();
        let captures = regex.captures(string.as_slice()).expect("invalid serialized PackageId");

        let name = captures.at(1);
        let version = semver::Version::parse(captures.at(2)).ok().expect("invalid version");
        let source_id = SourceId::from_url(captures.at(3).to_string());

        Ok(PackageId {
            inner: Arc::new(PackageIdInner {
                name: name.to_string(),
                version: version,
                source_id: source_id,
            }),
        })
    }
}

impl<S: hash::Writer> Hash<S> for PackageId {
    fn hash(&self, state: &mut S) {
        self.inner.name.hash(state);
        self.inner.version.to_string().hash(state);
        self.inner.source_id.hash(state);
    }
}

impl PartialEq for PackageId {
    fn eq(&self, other: &PackageId) -> bool {
        self.inner.eq(&*other.inner)
    }
}
impl PartialOrd for PackageId {
    fn partial_cmp(&self, other: &PackageId) -> Option<Ordering> {
        self.inner.partial_cmp(&*other.inner)
    }
}
impl Eq for PackageId {}
impl Ord for PackageId {
    fn cmp(&self, other: &PackageId) -> Ordering {
        self.inner.cmp(&*other.inner)
    }
}

#[deriving(Clone, Show, PartialEq)]
pub enum PackageIdError {
    InvalidVersion(String),
    InvalidNamespace(String)
}

impl CargoError for PackageIdError {
    fn description(&self) -> String {
        match *self {
            InvalidVersion(ref v) => format!("invalid version: {}", *v),
            InvalidNamespace(ref ns) => format!("invalid namespace: {}", *ns),
        }
    }
    fn is_human(&self) -> bool { true }
}

#[deriving(PartialEq, Hash, Clone, Encodable)]
pub struct Metadata {
    pub metadata: String,
    pub extra_filename: String
}

impl PackageId {
    pub fn new<T: ToSemver>(name: &str, version: T,
                             sid: &SourceId) -> CargoResult<PackageId> {
        let v = try!(version.to_semver().map_err(InvalidVersion));
        Ok(PackageId {
            inner: Arc::new(PackageIdInner {
                name: name.to_string(),
                version: v,
                source_id: sid.clone(),
            }),
        })
    }

    pub fn get_name(&self) -> &str {
        self.inner.name.as_slice()
    }

    pub fn get_version(&self) -> &semver::Version {
        &self.inner.version
    }

    pub fn get_source_id(&self) -> &SourceId {
        &self.inner.source_id
    }

    pub fn generate_metadata(&self) -> Metadata {
        let metadata = short_hash(
            &(self.inner.name.as_slice(), self.inner.version.to_string(),
              &self.inner.source_id));
        let extra_filename = format!("-{}", metadata);

        Metadata { metadata: metadata, extra_filename: extra_filename }
    }
}

impl Metadata {
    pub fn mix<T: Hash>(&mut self, t: &T) {
        let new_metadata = short_hash(&(self.metadata.as_slice(), t));
        self.extra_filename = format!("-{}", new_metadata);
        self.metadata = new_metadata;
    }
}

static CENTRAL_REPO: &'static str = "http://rust-lang.org/central-repo";

impl Show for PackageId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        try!(write!(f, "{} v{}", self.inner.name, self.inner.version));

        if self.inner.source_id.to_string().as_slice() != CENTRAL_REPO {
            try!(write!(f, " ({})", self.inner.source_id));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{PackageId, CENTRAL_REPO};
    use core::source::{RegistryKind, SourceId};
    use util::ToUrl;

    #[test]
    fn invalid_version_handled_nicely() {
        let loc = CENTRAL_REPO.to_url().unwrap();
        let repo = SourceId::new(RegistryKind, loc);

        assert!(PackageId::new("foo", "1.0", &repo).is_err());
        assert!(PackageId::new("foo", "1", &repo).is_err());
        assert!(PackageId::new("foo", "bar", &repo).is_err());
        assert!(PackageId::new("foo", "", &repo).is_err());
    }
}
