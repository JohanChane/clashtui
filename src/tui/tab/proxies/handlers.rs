use super::super::dev::*;
use crate::tui::business::Business;

use super::content::Proxies;
use super::tree::NodeType;

impl Proxies {
    pub fn fzf_find(&mut self, items: Vec<(String, usize)>, task_set: &mut FutureSet<Self>) {
        self.fzf_find_with_prompt(items, "Find Proxy", task_set);
    }

    pub fn fzf_find_with_prompt(
        &mut self,
        items: Vec<(String, usize)>,
        prompt: impl Into<String> + Send + 'static,
        task_set: &mut FutureSet<Self>,
    ) {
        let names: Vec<String> = items.iter().map(|(name, _)| name.clone()).collect();
        async move {
            let selected = FzfFinder::new(names)
                .with_title(prompt.into())
                .build_and_send_async()
                .await
                .await
                .unwrap_or_default();
            // Map fzf positional index back to tree index
            let target = selected.and_then(|pos| items.get(pos).map(|(_, idx)| *idx));
            wrapper(move |content: &mut Self| {
                content.jump_target.set(target);
            })
        }
        .spawn_at(task_set);
    }
}

impl Proxies {
    fn operation_started(&mut self, message: String) {
        self.error = Some(message);
        self.testing_since = Some(std::time::Instant::now());
    }
    pub fn select_inline(&mut self, group: String, node: String, business: &mut Business) {
        let message = format!("Switching to {node}...");
        if business.select_inline(group, node) {
            self.operation_started(message);
        }
    }
    pub fn test_delay(&mut self, name: String, ntype: NodeType, business: &mut Business) {
        let message = format!("Testing {name}...");
        if business.test_delay(name, ntype == NodeType::Folder) {
            self.operation_started(message);
        }
    }
    pub fn test_all_delay(&mut self, business: &mut Business) {
        let folders: Vec<String> = self
            .tree
            .nodes
            .iter()
            .filter(|n| n.node_type == NodeType::Folder)
            .map(|n| n.name.clone())
            .collect();
        let files: Vec<String> = self
            .tree
            .nodes
            .iter()
            .filter(|n| n.node_type == NodeType::File && n.depth == 0)
            .map(|n| n.name.clone())
            .collect();
        let total = folders.len() + files.len();
        if business.test_all_delay(folders, files) {
            self.operation_started(format!("Testing all ({total} groups/nodes)..."));
        }
    }
}
