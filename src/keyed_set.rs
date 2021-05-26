use std::{
    collections::{
        HashMap,
        hash_map,
    },
    fmt::Display
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Key(usize);

pub struct KeyedSet<T> {
    map: HashMap<Key, T>,
    next: Key, 
}

impl Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("#{}", self.0))
    }
}

impl<T> KeyedSet<T> {
    pub fn new() -> Self {
        Self { map: HashMap::new(), next: Key(0) }
    }

    fn generate_key(&mut self) -> Key {
        let key = self.next;
        self.next.0 += 1;
        key
    }
    
    pub fn insert(&mut self, value: T) -> Key {
        let key = self.generate_key();
        self.map.insert(key, value);
        key
    }

    pub fn get(&self, key: Key) -> Option<&T> {
        self.map.get(&key)
    }
    
    pub fn get_mut(&mut self, key: Key) -> Option<&mut T> {
        self.map.get_mut(&key)
    }

    pub fn remove(&mut self, key: Key) -> Option<T> {
        self.map.remove(&key)
    }

    pub fn iter(&self) -> <&Self as IntoIterator>::IntoIter {
        self.into_iter()
    }

    pub fn iter_mut(&mut self) -> <&mut Self as IntoIterator>::IntoIter {
        self.into_iter()
    }

    pub fn len(&self) -> usize { self.map.len() }
}

impl<T> IntoIterator for KeyedSet<T> {
    type Item = (Key, T);
    type IntoIter = hash_map::IntoIter<Key, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a KeyedSet<T> {
    type Item = (&'a Key, &'a T);
    type IntoIter = hash_map::Iter<'a, Key, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut KeyedSet<T> {
    type Item = (&'a Key, &'a mut T);
    type IntoIter = hash_map::IterMut<'a, Key, T>;

    fn into_iter(self) -> Self::IntoIter {
        (&mut self.map).into_iter()
    }
}

pub mod prelude {
    pub use super::{Key, KeyedSet};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_general_behavior() {
        let mut a = KeyedSet::new();
        let hello = a.insert("Hello!");
        let bye = a.insert("Bye!");

        assert_eq!(a.get(hello), Some(&"Hello!"));
        assert_eq!(a.get_mut(bye), Some(&mut "Bye!"));
        
        a.remove(hello);
        assert_eq!(a.get(hello), None);
        assert_eq!(a.get(bye), Some(&"Bye!"));
    }
}
