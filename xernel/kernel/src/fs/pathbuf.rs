use alloc::{
    string::{String, ToString},
    vec::Vec,
};

#[derive(Debug)]
pub struct PathBuf {
    inner: String,
}

impl PathBuf {
    pub fn new() -> PathBuf {
        PathBuf {
            inner: String::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> PathBuf {
        PathBuf {
            inner: String::with_capacity(capacity),
        }
    }

    pub fn components(&self) -> Vec<String> {
        self.inner
            .split_inclusive('/')
            .map(|s| s.to_string())
            .collect()
    }

    pub fn into_string(self) -> String {
        self.inner
    }

    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    pub fn clear(&mut self) {
        self.inner.clear()
    }

    pub fn starts_with(&self, pat: &PathBuf) -> bool {
        self.inner.starts_with(&pat.as_string())
    }

    pub fn as_string(&self) -> String {
        self.inner.clone()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    // TODO:
    pub fn push(&mut self) {}

    // TODO:
    pub fn pop(&mut self) {}
}

impl From<String> for PathBuf {
    fn from(path: String) -> Self {
        PathBuf { inner: path }
    }
}

impl From<&str> for PathBuf {
    fn from(path: &str) -> Self {
        PathBuf {
            inner: path.to_string(),
        }
    }
}

impl From<&String> for PathBuf {
    fn from(path: &String) -> Self {
        PathBuf {
            inner: path.clone(),
        }
    }
}

impl PartialEq for PathBuf {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl Clone for PathBuf {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}
