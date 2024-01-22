#[macro_export]
macro_rules! on_drop {
    ($name:ident, $t:expr) => {{
        OnDrop::new($name, $t)
    }};
    ($name:expr, $t:expr) => {{
        OnDrop::new($name, $t)
    }};
}

#[macro_export]
macro_rules! defer {
    ($t:expr) => {
        let _guard = OnDrop::new((), $t);
    };
    ($t:tt) => {
        let _guard = OnDrop::new((), || $t);
    };
}
