use super::model::MarketplaceEntry;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Node {
    pub git: String,
    pub title: String,
    pub entry: Option<MarketplaceEntry>,
    pub children: Vec<Node>,
}

pub(super) fn forest(entries: &[MarketplaceEntry], installed: bool, query: &str) -> Vec<Node> {
    let by_git: BTreeMap<_, _> = entries.iter().map(|e| (e.git.clone(), e)).collect();
    let query = query.trim().to_lowercase();
    let matches = |e: &MarketplaceEntry| {
        format!(
            "{} {} {} {}",
            e.title,
            e.summary,
            e.tags.join(" "),
            e.parent_title.as_deref().unwrap_or_default()
        )
        .to_lowercase()
        .contains(&query)
    };
    let mut retained = BTreeSet::new();
    for entry in entries.iter().filter(|e| e.installed == installed) {
        let mut path = vec![entry.git.clone()];
        let mut cursor = entry.parent_git.as_ref();
        let mut matched = matches(entry);
        while let Some(parent) = cursor {
            if path.contains(parent) {
                break;
            }
            path.push(parent.clone());
            let ancestor = by_git.get(parent);
            matched |= ancestor.is_some_and(|e| matches(e));
            cursor = ancestor.and_then(|e| e.parent_git.as_ref());
        }
        if matched {
            retained.extend(path);
        }
    }
    let mut children: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut roots = Vec::new();
    for git in &retained {
        let parent = by_git
            .get(git)
            .and_then(|e| e.parent_git.as_ref())
            .filter(|parent| retained.contains(*parent) && *parent != git);
        if let Some(parent) = parent {
            children
                .entry(parent.clone())
                .or_default()
                .push(git.clone());
        } else {
            roots.push(git.clone());
        }
    }
    fn build(
        git: String,
        by_git: &BTreeMap<String, &MarketplaceEntry>,
        children: &BTreeMap<String, Vec<String>>,
        seen: &mut BTreeSet<String>,
    ) -> Option<Node> {
        if !seen.insert(git.clone()) {
            return None;
        }
        let descendants = children
            .get(&git)
            .into_iter()
            .flatten()
            .filter_map(|child| build(child.clone(), by_git, children, seen))
            .collect();
        Some(Node {
            title: by_git
                .get(&git)
                .map(|e| e.title.clone())
                .or_else(|| {
                    by_git.values().find_map(|e| {
                        (e.parent_git.as_ref() == Some(&git))
                            .then(|| e.parent_title.clone())
                            .flatten()
                    })
                })
                .unwrap_or_else(|| "父插件".into()),
            entry: by_git.get(&git).map(|e| (*e).clone()),
            git,
            children: descendants,
        })
    }
    let mut seen = BTreeSet::new();
    let mut nodes = roots
        .into_iter()
        .filter_map(|git| build(git, &by_git, &children, &mut seen))
        .collect::<Vec<_>>();
    for git in retained {
        if let Some(node) = build(git, &by_git, &children, &mut seen) {
            nodes.push(node);
        }
    }
    nodes
}

#[cfg(test)]
mod tests {
    use super::*;
    fn entry(git: &str, parent: Option<&str>, installed: bool) -> MarketplaceEntry {
        serde_json::from_value(serde_json::json!({"git":git,"parent_git":parent,"rev":"abc","title":git,"summary":"","license":"MIT","tags":[],"installed":installed})).unwrap()
    }
    #[test]
    fn optional_child_remains_available_under_installed_parent() {
        let entries = vec![
            entry("Agent", None, true),
            entry("Memory", Some("Agent"), false),
        ];
        let installed = forest(&entries, true, "");
        assert_eq!(installed.len(), 1);
        assert!(installed[0].children.is_empty());
        let available = forest(&entries, false, "");
        assert_eq!(available[0].git, "Agent");
        assert_eq!(available[0].children[0].git, "Memory");
        assert_eq!(forest(&entries, false, "memory"), available);
        assert_eq!(forest(&entries, false, "agent"), available);
    }
    #[test]
    fn missing_parent_and_cycles_remain_bounded() {
        let entries = vec![entry("child", Some("parent"), false)];
        let nodes = forest(&entries, false, "");
        assert!(nodes[0].entry.is_none());
        assert_eq!(nodes[0].title, "父插件");
        assert_eq!(nodes[0].children.len(), 1);
        let cycle = vec![entry("A", Some("B"), false), entry("B", Some("A"), false)];
        assert_eq!(forest(&cycle, false, "").len(), 1);
    }

    #[test]
    fn parent_title_is_searchable_until_the_parent_is_published() {
        let mut child = entry("child", Some("parent"), false);
        child.title = "记忆".into();
        child.parent_title = Some("智能体".into());
        let nodes = forest(&[child.clone()], false, "智能体");
        assert_eq!(nodes[0].title, "智能体");
        let mut parent = entry("parent", None, false);
        parent.title = "正式父插件".into();
        assert_eq!(forest(&[parent, child], false, "")[0].title, "正式父插件");
    }
}
