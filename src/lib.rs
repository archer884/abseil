pub enum Maybe<T> {
    Yes(T),
    No
}

pub trait AsMaybe<T> {
    fn maybe(self) -> Maybe<T>;
}

impl<T> AsMaybe<T> for Option<T> {
    fn maybe(self) -> Maybe<T> {
        match self {
            Some(object) => Maybe::Yes(object),
            None => Maybe::No
        }
    }
}

impl<T, E> AsMaybe<T> for Result<T, E> {
    fn maybe(self) -> Maybe<T> {
        match self {
            Ok(object) => Maybe::Yes(object),
            Err(_) => Maybe::No
        }
    }
}

pub struct Fallback<T> {
    object: Maybe<T>
}

impl<T> Fallback<T> {
    pub fn from<M: AsMaybe<T>>(object: M) -> Fallback<T> {
        Fallback { object: object.maybe() }
    }

    pub fn to(self, backup: T) -> Box<T> {
        match self.object {
            Maybe::Yes(object) => Box::new(object),
            Maybe::No => Box::new(backup)
        }
    }
}
