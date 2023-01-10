use alloc::sync::Arc;
use libxernel::sync::Spinlock;

use super::vnode::VNode;

// FileDescriptor
pub struct FileHandle {
    node: Arc<Spinlock<VNode>>,
}

impl FileHandle {
    pub fn new(node: Arc<Spinlock<VNode>>) -> Self {
        Self { node }
    }

    pub fn get_node(&self) -> Arc<Spinlock<VNode>> {
        self.node.clone()
    }
}
