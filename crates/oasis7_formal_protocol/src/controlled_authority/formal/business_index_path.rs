//! Bounded immutable path transport; no root or outcome qualification is cached.
use super::*;
use std::cell::RefCell;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BusinessIndexPathV1 {
    #[serde(deserialize_with = "codec::decode_path_nodes")]
    pub nodes: Vec<BusinessIndexNodeWriteV1>,
}

impl BusinessIndexRootV1 {
    /// Collect exactly the nodes read by the canonical presence/absence walk.
    pub fn path_nodes(
        &self,
        key: &str,
        reader: &impl BusinessIndexNodeReader,
    ) -> Result<BusinessIndexPathV1, String> {
        struct Recording<'a, R> {
            reader: &'a R,
            nodes: RefCell<Vec<BusinessIndexNodeWriteV1>>,
        }
        impl<R: BusinessIndexNodeReader> BusinessIndexNodeReader for Recording<'_, R> {
            fn read_node(&self, digest: &str) -> Result<Option<Vec<u8>>, String> {
                let bytes = self.reader.read_node(digest)?;
                if let Some(bytes) = &bytes {
                    self.nodes.borrow_mut().push(BusinessIndexNodeWriteV1 {
                        digest: digest.to_owned(),
                        bytes: bytes.clone(),
                    });
                }
                Ok(bytes)
            }
        }
        let recording = Recording {
            reader,
            nodes: RefCell::new(Vec::new()),
        };
        self.get(key, &recording)?;
        let path = BusinessIndexPathV1 {
            nodes: recording.nodes.into_inner(),
        };
        path.verify(self, key)?;
        Ok(path)
    }
}

impl BusinessIndexPathV1 {
    /// Verify the exact requested root/key with the original canonical walker.
    /// Empty paths only prove absence for the canonical empty root.
    pub fn verify(
        &self,
        root: &BusinessIndexRootV1,
        key: &str,
    ) -> Result<Option<BusinessIndexLeafV1>, String> {
        if self.nodes.len() > BUSINESS_INDEX_DEPTH + 1 {
            return Err("business index path node bound".into());
        }
        let mut total = 0usize;
        for node in &self.nodes {
            total = total
                .checked_add(node.bytes.len())
                .ok_or("business index path byte overflow")?;
            if node.bytes.len() > BUSINESS_INDEX_MAX_LEAF_BYTES + 128
                || total > BUSINESS_INDEX_MAX_DELTA_BYTES
            {
                return Err("business index path byte bound".into());
            }
        }
        struct Ordered<'a> {
            nodes: &'a [BusinessIndexNodeWriteV1],
            next: RefCell<usize>,
        }
        impl BusinessIndexNodeReader for Ordered<'_> {
            fn read_node(&self, digest: &str) -> Result<Option<Vec<u8>>, String> {
                let mut next = self.next.borrow_mut();
                let node = self
                    .nodes
                    .get(*next)
                    .ok_or("business index path missing node")?;
                if node.digest != digest {
                    return Err("business index path order or digest mismatch".into());
                }
                *next += 1;
                Ok(Some(node.bytes.clone()))
            }
        }
        let ordered = Ordered {
            nodes: &self.nodes,
            next: RefCell::new(0),
        };
        let leaf = root.get(key, &ordered)?;
        if ordered.next.into_inner() != self.nodes.len() {
            return Err("business index path has unreferenced nodes".into());
        }
        Ok(leaf)
    }
}

#[cfg(test)]
#[path = "business_index_path_tests.rs"]
mod tests;
