use super::{DocumentSemanticBreadcrumb, DocumentSemanticTreeEntry};
#[path = "document_semantic_routes.rs"]
mod routes;
use crate::{
    Error, Result,
    document_inventory::{DocumentPairing, IndexedDocument},
};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
#[derive(Clone)]
struct Node {
    label: String,
    path: Option<String>,
    source: Option<String>,
    edge: Option<DocumentPairing>,
    group: Vec<Node>,
    group_key: Option<String>,
    boundary: Option<String>,
}
pub(super) struct Projection<'a> {
    pub files: HashMap<&'a str, &'a IndexedDocument>,
    pub eligible: BTreeSet<String>,
    pub references: BTreeMap<String, BTreeSet<String>>,
    pub issues: Vec<String>,
    roots: Vec<Node>,
    edges: HashMap<&'a str, Vec<Node>>,
}
impl<'a> Projection<'a> {
    pub fn new(files: &'a [IndexedDocument], view: &str) -> Self {
        let map: HashMap<_, _> = files.iter().map(|f| (f.path.as_str(), f)).collect();
        let product = view == "product";
        let root = if product {
            "doc/product/README.md"
        } else {
            "doc/README.md"
        };
        let mut issues = vec![];
        let mut edges = HashMap::new();
        for f in files {
            let relevant = if product {
                f.path.starts_with("doc/product/")
            } else {
                !f.path.starts_with("doc/product/")
            };
            if !relevant {
                continue;
            }
            if let Some(nav) = &f.navigation {
                if let Some(issue) = &nav.relationships.issue {
                    issues.push(format!("{}: {}", f.path, issue));
                }
                edges.insert(
                    f.path.as_str(),
                    nav.relationships
                        .targets
                        .iter()
                        .enumerate()
                        .map(|(i, edge)| Node {
                            label: nav
                                .labels
                                .get(i)
                                .cloned()
                                .unwrap_or_else(|| edge.destination.clone()),
                            path: edge
                                .target
                                .as_ref()
                                .map(|p| p.split('#').next().unwrap_or(p).to_owned()),
                            source: Some(f.path.clone()),
                            edge: Some(edge.clone()),
                            group: vec![],
                            group_key: None,
                            boundary: edge.target.as_deref().and_then(|target| {
                                let target = target.split('#').next().unwrap_or(target);
                                let domain = |p: &str| {
                                    if p.starts_with("doc/product/") {
                                        p.split('/').take(3).collect::<Vec<_>>().join("/")
                                    } else {
                                        p.split('/').take(2).collect::<Vec<_>>().join("/")
                                    }
                                };
                                (f.path != root && domain(&f.path) != domain(target))
                                    .then(|| "cross_domain".into())
                            }),
                        })
                        .collect::<Vec<_>>(),
                );
            } else if f.state == "read" {
                issues.push(format!(
                    "{}: navigation not parsed; refresh local index",
                    f.path
                ));
            }
        }
        let initial = edges.get(root).cloned().unwrap_or_default();
        if !map.contains_key(root) {
            issues.push(format!(
                "navigation root {root} is outside candidate manifest"
            ));
        } else if map[root].state != "read" {
            issues.push(format!("navigation root {root} is unread"));
        }
        let roots = if product {
            initial
        } else {
            let mut groups: Vec<Node> = vec![];
            for node in initial {
                if let Some(group) = groups.iter_mut().find(|g| g.label == node.label) {
                    group.group.push(node);
                } else {
                    groups.push(Node {
                        label: node.label.clone(),
                        path: None,
                        source: None,
                        edge: None,
                        boundary: None,
                        group_key: Some(format!(
                            "module:{}",
                            node.path.as_deref().unwrap_or("unresolved")
                        )),
                        group: vec![node],
                    });
                }
            }
            groups
        };
        let mut projection = Self {
            files: map,
            eligible: BTreeSet::new(),
            references: BTreeMap::new(),
            issues,
            roots,
            edges,
        };
        let reached = projection.reachable(&projection.roots);
        projection.eligible = files
            .iter()
            .filter(|f| {
                let domain = if product {
                    f.path.starts_with("doc/product/")
                } else {
                    f.path.starts_with("doc/") && !f.path.starts_with("doc/product/")
                };
                f.path != root
                    && ((domain && ["prd", "design", "entry"].contains(&f.kind.as_str()))
                        || reached.contains(&f.path))
            })
            .map(|f| f.path.clone())
            .collect();
        let unassigned = projection
            .eligible
            .difference(&reached)
            .filter_map(|path| projection.files.get(path.as_str()))
            .map(|f| Node {
                label: f.title.clone(),
                path: Some(f.path.clone()),
                source: None,
                edge: None,
                boundary: None,
                group: vec![],
                group_key: None,
            })
            .collect::<Vec<_>>();
        if !unassigned.is_empty() {
            projection.roots.push(Node {
                label: "未归属文档（未建立显式导航）".into(),
                path: None,
                source: None,
                edge: None,
                boundary: None,
                group: unassigned,
                group_key: Some("unassigned".into()),
            });
        }
        let refs = crate::document_directory_references::historical_references(
            &files.iter().collect::<Vec<_>>(),
        );
        projection.references.extend(refs);
        for (source, nodes) in &projection.edges {
            if projection.files.get(source).is_some_and(|f| {
                f.state == "read" && !crate::document_directory_references::is_history(f)
            }) {
                for node in nodes {
                    if let Some(path) = &node.path
                        && let Some(target) = projection.files.get(path.as_str())
                        && target.state == "read"
                        && crate::document_directory_references::is_history(target)
                        && target.revision.is_some()
                        && node.edge.as_ref().is_some_and(|edge| {
                            edge.resolution == "resolved" && edge.target_revision == target.revision
                        })
                    {
                        projection
                            .references
                            .entry(path.clone())
                            .or_default()
                            .insert((*source).to_owned());
                    }
                }
            }
        }
        if projection.issues.len() > 20 {
            let count = projection.issues.len();
            projection.issues.truncate(20);
            projection.issues.push(format!(
                "{count} navigation issues in total; first 20 shown"
            ));
        }
        projection
    }
    fn next(&self, node: &Node) -> Vec<Node> {
        if !node.group.is_empty() {
            return node.group.clone();
        }
        if node.boundary.is_some() {
            return vec![];
        }
        if node
            .edge
            .as_ref()
            .is_some_and(|e| e.resolution != "resolved")
        {
            return vec![];
        }
        node.path
            .as_ref()
            .and_then(|p| self.edges.get(p.as_str()))
            .cloned()
            .unwrap_or_default()
    }
    fn reachable(&self, nodes: &[Node]) -> BTreeSet<String> {
        let mut seen = BTreeSet::new();
        let mut expanded = BTreeSet::new();
        let mut queue = nodes.to_vec();
        while let Some(node) = queue.pop() {
            if let Some(path) = &node.path {
                if node.boundary.is_none() {
                    seen.insert(path.clone());
                }
                if node.boundary.is_some()
                    || node
                        .edge
                        .as_ref()
                        .is_some_and(|e| e.resolution != "resolved")
                    || !expanded.insert(path.clone())
                {
                    continue;
                }
            }
            queue.extend(self.next(&node));
        }
        seen
    }
    pub fn children(
        &self,
        parent: &str,
        matching: &BTreeSet<String>,
        snapshot: &str,
        view: &str,
        current: bool,
    ) -> Result<(
        Vec<DocumentSemanticTreeEntry>,
        Vec<DocumentSemanticBreadcrumb>,
    )> {
        if parent.len() > 4096 {
            return Err(Error::Invalid("invalid semantic occurrence".into()));
        }
        let original_parent = parent;
        let parent = if parent.is_empty() {
            ""
        } else {
            let (generation, route) = parent
                .split_once(':')
                .ok_or_else(|| Error::Invalid("invalid semantic occurrence snapshot".into()))?;
            if generation != format!("{snapshot}-{view}") || route.is_empty() {
                return Err(Error::Invalid(
                    "semantic occurrence expired; reload tree root".into(),
                ));
            }
            route
        };
        let mut nodes = self.roots.clone();
        let mut crumbs = vec![];
        let mut ancestors = HashSet::new();
        let mut route = String::new();
        let mut stable_route = vec![];
        let steps: Vec<_> = if parent.is_empty() {
            vec![]
        } else {
            parent.split('/').collect()
        };
        if steps.len() >= 64 {
            return Err(Error::Invalid(
                "semantic occurrence depth limit reached".into(),
            ));
        }
        for step in &steps {
            let index = step
                .parse::<usize>()
                .map_err(|_| Error::Invalid("invalid semantic occurrence".into()))?;
            if index.to_string() != *step {
                return Err(Error::Invalid("invalid semantic occurrence".into()));
            }
            let stable_steps = routes::steps(&nodes);
            let node = nodes
                .get(index)
                .ok_or_else(|| Error::Invalid("semantic occurrence is outside snapshot".into()))?;
            if !route.is_empty() {
                route.push('/');
            }
            route.push_str(step);
            stable_route.push(stable_steps[index].clone());
            crumbs.push(DocumentSemanticBreadcrumb {
                stable_route: stable_route.clone(),
                occurrence_id: format!("{snapshot}-{view}:{route}"),
                label: node.label.clone(),
            });
            if let Some(path) = &node.path
                && !ancestors.insert(path.clone())
            {
                return Err(Error::Invalid(
                    "semantic occurrence is a cycle marker".into(),
                ));
            }
            nodes = self.next(node);
        }
        let child_routes = routes::steps(&nodes);
        let entries = nodes
            .iter()
            .enumerate()
            .filter_map(|(i, node)| {
                let reachable = self.reachable(std::slice::from_ref(node));
                let count = reachable.intersection(matching).count();
                let diagnostic = node.boundary.is_some()
                    || node
                        .edge
                        .as_ref()
                        .is_some_and(|e| e.resolution != "resolved");
                if count == 0 && !diagnostic {
                    return None;
                }
                let cycle = node.path.as_ref().is_some_and(|p| ancestors.contains(p));
                let depth_limited = steps.len() + 1 >= 64;
                let file = node
                    .path
                    .as_ref()
                    .and_then(|p| self.files.get(p.as_str()))
                    .map(|f| (*f).clone());
                let historical = if current {
                    node.path
                        .as_ref()
                        .and_then(|path| self.references.get(path))
                } else {
                    None
                };
                Some(DocumentSemanticTreeEntry {
                    occurrence_id: if original_parent.is_empty() {
                        format!("{snapshot}-{view}:{i}")
                    } else {
                        format!("{original_parent}/{i}")
                    },
                    stable_route: stable_route
                        .iter()
                        .cloned()
                        .chain(std::iter::once(child_routes[i].clone()))
                        .collect(),
                    label: node.label.clone(),
                    path: node.path.clone(),
                    kind: file.as_ref().map_or("group".into(), |f| f.kind.clone()),
                    matching_files: count,
                    historical_reference_files: historical.map_or(0, BTreeSet::len),
                    historical_reference_paths: historical
                        .map(|paths| paths.iter().take(20).cloned().collect())
                        .unwrap_or_default(),
                    cycle,
                    depth_limited,
                    boundary: node.boundary.clone(),
                    source_path: node.source.clone(),
                    source_line: node.edge.as_ref().map(|e| e.declaration_line),
                    relationship: node.edge.clone(),
                    file,
                    has_children: !cycle && !depth_limited && !self.next(node).is_empty(),
                })
            })
            .collect();
        Ok((entries, crumbs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn old_navigation_cache_is_unparsed_instead_of_declared_empty() {
        let file = IndexedDocument::new("project", "doc/README.md", false);
        let mut old = serde_json::to_value(&file).unwrap();
        old.as_object_mut().unwrap().remove("navigation");
        let old: IndexedDocument = serde_json::from_value(old).unwrap();
        let files = [old];
        let projection = Projection::new(&files, "professional");
        assert!(
            projection
                .issues
                .iter()
                .any(|issue| issue.contains("navigation not parsed"))
        );
        let mut parsed = file;
        parsed.navigation = Some(crate::document_inventory::DocumentNavigation {
            relationships: crate::document_inventory::DocumentPairings {
                targets: vec![],
                issue: None,
            },
            labels: vec![],
        });
        let files = [parsed];
        assert!(Projection::new(&files, "professional").issues.is_empty());
    }
}
