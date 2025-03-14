pub type AccountIndex = u64;

pub trait Repository<T> {
    type Err;
    fn get(&self, account: AccountIndex) -> Result<Option<T>, Self::Err>;
    fn revoke(&mut self, account: AccountIndex) -> Result<Option<T>, Self::Err>;
    fn put(&mut self, account: AccountIndex, data: T) -> Result<(), Self::Err>;
    fn keys(&self) -> Result<impl Iterator<Item = AccountIndex>, Self::Err>;
    fn entries(&self) -> Result<impl Iterator<Item = (AccountIndex, T)>, Self::Err>;
}
