use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use crate::object::Identity;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TagType {
    Lightweight,
    Annotated,
}

/// A tag object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    pub target_hash: String,
    pub tag_type: TagType,
    pub tagger: Option<Identity>,
    pub message: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Tag {
    pub fn lightweight(name: String, target_hash: String) -> Self {
        Self {
            name, target_hash, tag_type: TagType::Lightweight,
            tagger: None, message: None, created_at: Utc::now(),
        }
    }

    pub fn annotated(name: String, target_hash: String, tagger: Identity, message: String) -> Self {
        Self {
            name, target_hash, tag_type: TagType::Annotated,
            tagger: Some(tagger), message: Some(message), created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lightweight_tag() {
        let tag = Tag::lightweight("v1.0".to_string(), "abc123".to_string());
        assert_eq!(tag.tag_type, TagType::Lightweight);
        assert!(tag.message.is_none());
    }

    #[test]
    fn test_annotated_tag() {
        let tagger = Identity::new("Neo".to_string(), "neo@apexforge.dev".to_string());
        let tag = Tag::annotated("v2.0".to_string(), "def456".to_string(), tagger, "Release 2.0".to_string());
        assert_eq!(tag.tag_type, TagType::Annotated);
        assert_eq!(tag.message.as_deref(), Some("Release 2.0"));
    }
}
