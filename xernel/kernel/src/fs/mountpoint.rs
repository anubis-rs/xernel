use alloc::string::String;
use alloc::vec::Vec;
use alloc::{rc::Rc, string::ToString};

use super::{FsNode, FsNodeHandler};

pub struct FsMountpoint {
    pub path: String,
    pub node: Rc<FsNode>,
}

impl FsMountpoint {
    pub fn new(source: String, name: String, handler: Rc<dyn FsNodeHandler>) -> Self {
        //let path: Vec<String> = Vec::new();
        //path.push(source.to_string());

        FsMountpoint {
            path: source.to_string(),
            node: Rc::new(FsNode::new(name.clone(), handler.clone())),
        }
    }
}
