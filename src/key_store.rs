use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    Available,
    Reserved,
    Retrieved,
}

#[derive(Debug, Clone)]
pub struct StoredKey {
    pub key_id: String,
    pub key_value: String,
    pub reservable: bool,
    pub state: KeyState,
}

pub struct KeyStore {
    keys: Mutex<Vec<StoredKey>>,
}

impl KeyStore {
    pub fn new() -> Self {
        Self {
            keys: Mutex::new(Vec::new()),
        }
    }

    pub fn add_key(&self, key_id: &str, key_value: &str, reservable: bool) {
        let mut keys = self.keys.lock().unwrap();
        keys.push(StoredKey {
            key_id: key_id.to_string(),
            key_value: key_value.to_string(),
            reservable,
            state: KeyState::Available,
        });
    }

    pub fn reserve_keys(&self, count: usize) -> Vec<(String, String)> {
        let mut keys = self.keys.lock().unwrap();
        let mut result = Vec::with_capacity(count);

        for key in keys.iter_mut() {
            if result.len() >= count {
                break;
            }
            if key.state == KeyState::Available && key.reservable {
                key.state = KeyState::Reserved;
                result.push((key.key_id.clone(), key.key_value.clone()));
            }
        }

        result
    }

    pub fn retrieve_key(&self, key_id: &str) -> Option<(String, String)> {
        let mut keys = self.keys.lock().unwrap();

        for key in keys.iter_mut() {
            if key.key_id == key_id && key.state == KeyState::Available {
                key.state = KeyState::Retrieved;
                return Some((key.key_id.clone(), key.key_value.clone()));
            }
        }

        None
    }

    pub fn available_count(&self) -> usize {
        let keys = self.keys.lock().unwrap();
        keys.iter().filter(|k| k.state == KeyState::Available).count()
    }
}

pub trait KeyGatherer: Send {
    fn run(&self, store: Arc<KeyStore>);
}
