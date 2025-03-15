use crate::repo::model::{AccountIndex, Repository};
use crate::tencent::Client;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct ClientCollection {
    inner: HashMap<AccountIndex, Arc<Mutex<Client>>>,
}

impl ClientCollection {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn from_token_repo<R>(repo: &R) -> Self
    where
        R: Repository<String>,
        <R as Repository<String>>::Err: Debug,
    {
        let clients = HashMap::from_iter(
            repo.entries()
                .expect("Failed to list registered user id")
                .map(|(id, token)| (id, Arc::new(Mutex::new(Client::with_token(&token))))),
        );
        Self { inner: clients }
    }
    
    pub async fn get(&self, acc: AccountIndex) -> Option<&Mutex<Client>> {
        self.inner.get(&acc).map(|c| c.as_ref())
    }
    
    pub async fn insert(&mut self, acc: AccountIndex, client: Client) {
        self.inner.insert(acc, Arc::new(Mutex::new(client)));
    }
    
    pub async fn remove(&mut self, acc: AccountIndex) {
        self.inner.remove(&acc);
    }
}
