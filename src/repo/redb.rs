use std::convert::Into;
use crate::repo::model::{AccountIndex, Repository};
use redb::{
    AccessGuard, Database, DatabaseError, Error, ReadOnlyTable, ReadableTable, TableDefinition,
    TableError, Value,
};
use std::path::Path;
use std::sync::Arc;

type AccountIndexedTable<T> = TableDefinition<'static, AccountIndex, T>;
pub type RedbRepoDefault<T> = RedbRepo<T, T>;

pub struct RedbRepo<From, Into>
where
    From: Value + 'static,
{
    table: AccountIndexedTable<From>,
    db: Arc<Database>,
    transform: Transformer<From, Into>,
}

impl<From, Into> RedbRepo<From, Into> where From: Value + 'static {}

impl<T> RedbRepo<T, T>
where
    T: Value + 'static,
{
    pub fn new(table: AccountIndexedTable<T>, db: Arc<Database>) -> Self {
        Self {
            table,
            db,
            transform: Transformer {
                forward: |e| e,
                backward: |e| e,
            },
        }
    }

    pub fn create(
        table: AccountIndexedTable<T>,
        name: impl AsRef<Path>,
    ) -> Result<Self, DatabaseError> {
        Ok(Self::new(table, Arc::new(Database::create(name)?)))
    }
}

impl<From, Into> RedbRepo<From, Into>
where
    From: Value + 'static,
{
    pub fn new_proxy(
        table: AccountIndexedTable<From>,
        db: Arc<Database>,
        transform: Transformer<From, Into>,
    ) -> Self {
        Self {
            table,
            db,
            transform,
        }
    }

    pub fn create_proxy(
        table: AccountIndexedTable<From>,
        name: impl AsRef<Path>,
        transform: Transformer<From, Into>,
    ) -> Result<Self, DatabaseError> {
        Ok(Self::new_proxy(
            table,
            Arc::new(Database::create(name)?),
            transform,
        ))
    }
}

impl<From, Into> Repository<Into> for RedbRepo<From, Into>
where
    From: Value + Clone + 'static + for<'a> std::borrow::Borrow<<From as Value>::SelfType<'a>>,
{
    type Err = Error;

    fn get(&self, account: AccountIndex) -> Result<Option<Into>, Error> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(self.table)?;
        Ok(table
            .get(account)?
            .map(|s| (self.transform.forward)(s.clone())))
    }

    fn revoke(&mut self, account: AccountIndex) -> Result<Option<Into>, Error> {
        let write_txn = self.db.begin_write()?;
        let mut table = write_txn.open_table(self.table)?;
        let option = table.remove(account)?;
        Ok(option.map(|s| (self.transform.forward)(s.clone())))
    }

    fn put(&mut self, account: AccountIndex, data: Into) -> Result<(), Error> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(self.table)?;
            table.insert(account, (self.transform.backward)(data))?;
        }
        write_txn.commit()?;
        Ok(())
    }

    fn keys(&self) -> Result<impl Iterator<Item = AccountIndex>, Error> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(self.table)?;
        let vec: Vec<AccountIndex> = table.iter()?.map(|e| e.unwrap().0.value()).collect();
        Ok(KeyIterator::new(vec))
    }

    fn entries(&self) -> Result<impl Iterator<Item = (AccountIndex, Into)>, Self::Err> {
        let read_txn = self.db.begin_read()?;
        match read_txn.open_table(self.table) {
            Ok(table) => {
                let vec: Vec<AccountIndex> = table.iter()?.map(|e| e.unwrap().0.value()).collect();
                let key_iter = KeyIterator::new(vec);

                Ok(OptionalIterator {
                    inner: Some(EntryIterator {
                        table,
                        key_iter,
                        forward: self.transform.forward,
                    }),
                })
            }
            Err(TableError::TableDoesNotExist(_)) => {
                Ok(OptionalIterator { inner: None })
            }
            Err(e) => {
                Err(e.into())
            },
        }
    }
}

struct KeyIterator {
    vec: Vec<AccountIndex>,
    idx: usize,
}

impl KeyIterator {
    fn new(vec: Vec<AccountIndex>) -> Self {
        Self { vec, idx: 0 }
    }
}

impl Iterator for KeyIterator {
    type Item = AccountIndex;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.vec.len() {
            None
        } else {
            let v = self.vec[self.idx];
            self.idx += 1;
            Some(v)
        }
    }
}

struct EntryIterator<Into, From>
where
    From: Value + 'static,
{
    table: ReadOnlyTable<AccountIndex, From>,
    key_iter: KeyIterator,
    forward: fn(From) -> Into,
}

impl<From, Into> Iterator for EntryIterator<Into, From>
where
    From: Value + Clone + 'static,
{
    type Item = (AccountIndex, Into);

    fn next(&mut self) -> Option<Self::Item> {
        let key = self.key_iter.next()?;
        self.table
            .get(key)
            .map_or(None, |e| Some((key, (self.forward)(e.unwrap().clone()))))
    }
}

struct OptionalIterator<T: Iterator> {
    inner: Option<T>,
}

impl<T: Iterator> Iterator for OptionalIterator<T> {
    type Item = <T as Iterator>::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.as_mut()?.next()
    }
}

trait CloneAccess<T> {
    fn clone(&self) -> T;
}

impl<'a, T> CloneAccess<T> for AccessGuard<'a, T>
where
    T: Value + Clone,
{
    fn clone(&self) -> T {
        unsafe {
            let v = self.value();
            std::mem::transmute_copy::<_, T>(&v).clone()
        }
    }
}

pub struct Transformer<From, Into> {
    pub forward: fn(From) -> Into,
    pub backward: fn(Into) -> From,
}
