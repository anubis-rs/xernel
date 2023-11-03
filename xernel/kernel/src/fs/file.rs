use alloc::sync::Arc;
use libxernel::sync::Spinlock;

use super::vnode::VNode;

pub struct File {
    node: Arc<Spinlock<VNode>>,
    offset: usize
}

impl File {
    pub fn new(node: Arc<Spinlock<VNode>>) -> Self {
        Self { 
            node: node,
            offset: 0
        }
    }

    pub fn get_node(&self) -> Arc<Spinlock<VNode>> {
        self.node.clone()
    }
}
