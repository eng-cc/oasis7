//! Immutable compressed radix index. The signed checkpoint root authenticates
//! both membership and absence; unreachable staged nodes have no authority.
use super::*;
use std::collections::BTreeMap;
use std::path::Path;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Binding {
    pub request: LocalRequestIdentity,
    pub position: u64,
    pub proposal: Option<String>,
    pub frozen: Option<String>,
    pub prepared: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum Node {
    Leaf {
        key: String,
        binding: Binding,
    },
    Branch {
        prefix: String,
        children: BTreeMap<char, String>,
    },
}
fn read(path: &Path, hash: &str) -> Result<Node, ProtocolError> {
    super::archive::object_read(path, "index", hash)
}
fn write(path: &Path, node: &Node) -> Result<String, ProtocolError> {
    super::archive::object_write(path, "index", node)
}
pub(super) fn key(request: &LocalRequestIdentity) -> Result<String, ProtocolError> {
    crypto::hash("oasis7.endpoint.request.index.v2", request)
}
pub(super) fn get(
    path: &Path,
    root: Option<&str>,
    key: &str,
) -> Result<Option<Binding>, ProtocolError> {
    let Some(root) = root else {
        return Ok(None);
    };
    let mut hash = root.to_owned();
    let mut depth = 0;
    for _ in 0..=64 {
        match read(path, &hash)? {
            Node::Leaf {
                key: stored,
                binding,
            } => {
                crypto::hash_hex(&stored)?;
                if self::key(&binding.request)? != stored {
                    return Err(invalid("index leaf identity"));
                }
                return Ok((stored == key).then_some(binding));
            }
            Node::Branch { prefix, children } => {
                if prefix.len() < depth
                    || prefix.len() >= 64
                    || children.len() < 2
                    || children.len() > 16
                    || !prefix
                        .bytes()
                        .all(|v| v.is_ascii_hexdigit() && !v.is_ascii_uppercase())
                    || children
                        .keys()
                        .any(|v| !v.is_ascii_hexdigit() || v.is_ascii_uppercase())
                {
                    return Err(invalid("index radix structure"));
                }
                if !key.starts_with(&prefix) {
                    return Ok(None);
                }
                let selector = key.as_bytes()[prefix.len()] as char;
                let Some(child) = children.get(&selector) else {
                    return Ok(None);
                };
                hash = child.clone();
                depth = prefix.len() + 1;
            }
        }
    }
    Err(invalid("index depth"))
}
pub(super) fn insert(
    path: &Path,
    root: Option<&str>,
    binding: Binding,
) -> Result<String, ProtocolError> {
    let key = self::key(&binding.request)?;
    insert_at(path, root, &key, binding, 0)
}
fn insert_at(
    path: &Path,
    root: Option<&str>,
    key: &str,
    binding: Binding,
    depth: usize,
) -> Result<String, ProtocolError> {
    if depth > 64 {
        return Err(invalid("index depth"));
    }
    let Some(root) = root else {
        return write(
            path,
            &Node::Leaf {
                key: key.into(),
                binding,
            },
        );
    };
    let node = read(path, root)?;
    let prefix = match &node {
        Node::Leaf { key, .. } => key,
        Node::Branch { prefix, .. } => prefix,
    };
    let common = key
        .bytes()
        .zip(prefix.bytes())
        .take_while(|(a, b)| a == b)
        .count();
    if common < prefix.len() {
        if common < depth || common >= 64 {
            return Err(invalid("index prefix"));
        }
        let leaf = write(
            path,
            &Node::Leaf {
                key: key.into(),
                binding,
            },
        )?;
        let children = BTreeMap::from([
            (prefix.as_bytes()[common] as char, root.into()),
            (key.as_bytes()[common] as char, leaf),
        ]);
        return write(
            path,
            &Node::Branch {
                prefix: key[..common].into(),
                children,
            },
        );
    }
    match node {
        Node::Leaf {
            key: old,
            binding: previous,
        } => {
            if old == key
                && previous.request == binding.request
                && previous.position == binding.position
                && previous.frozen == binding.frozen
                && previous.proposal.is_none()
                && binding.proposal.is_some()
            {
                return write(path, &Node::Leaf { key: old, binding });
            }
            if old == key
                && previous.request == binding.request
                && previous.position == binding.position
                && previous.frozen == binding.frozen
                && previous.proposal == binding.proposal
                && previous.prepared.is_some()
            {
                return write(path, &Node::Leaf { key: old, binding });
            }
            if old != key || previous != binding {
                return Err(invalid("immutable request identity collision"));
            }
            Ok(root.into())
        }
        Node::Branch {
            prefix,
            mut children,
        } => {
            if prefix.len() >= 64 || prefix.len() < depth {
                return Err(invalid("index prefix"));
            }
            let selector = key.as_bytes()[prefix.len()] as char;
            let child = insert_at(
                path,
                children.get(&selector).map(String::as_str),
                key,
                binding,
                prefix.len() + 1,
            )?;
            children.insert(selector, child);
            write(path, &Node::Branch { prefix, children })
        }
    }
}
