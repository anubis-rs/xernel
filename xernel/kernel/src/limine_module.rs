use limine::{LimineFile, LimineModuleRequest};

static MODULE_REQUEST: LimineModuleRequest = LimineModuleRequest::new(0);

pub fn get_limine_module(name: &str) -> Option<&LimineFile> {
    let modules = MODULE_REQUEST.get_response().get().unwrap().modules();

    for m in modules {
        // NOTE: the cmdline is wrapped in quotes, so we need to remove them
        let cmdline = m.cmdline.to_str().unwrap().to_str().unwrap();
        let mut cmd_chars = cmdline.chars();
        cmd_chars.next();
        cmd_chars.next_back();

        let cmdline_name = cmd_chars.as_str();

        if cmdline_name == name {
            return Some(unsafe { &*m.as_ptr() });
        }
    }

    None
}
