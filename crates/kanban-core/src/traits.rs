use serde::{Deserialize, Serialize};

pub trait Editable<T>: Serialize + for<'de> Deserialize<'de> + Sized {
    fn from_entity(entity: &T) -> Self;
    fn apply_to(self, entity: &mut T);
}
