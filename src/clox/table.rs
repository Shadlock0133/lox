use std::fmt;

use super::value::ObjString;

const MAX_LOAD_FACTOR: (usize, usize) = (3, 4); // 3/4 or 75%

pub struct Table<V> {
    len: usize,
    entries: Box<[Entry<V>]>,
}

impl<V> Default for Table<V> {
    fn default() -> Self {
        Self {
            len: 0,
            entries: Box::new([]),
        }
    }
}

impl<V: fmt::Debug> fmt::Debug for Table<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Table {{")?;
        if !self.entries.is_empty() {
            writeln!(f)?;
        }
        for (i, entry) in self.entries.iter().enumerate() {
            write!(f, "    bucket {:4x}: ", i)?;
            match entry {
                Entry::Empty => write!(f, "  <empty>")?,
                Entry::Tombstone => write!(f, "| <tombstone>")?,
                Entry::Occupied(o) => {
                    if o.key.1 as usize % self.capacity() != i {
                        write!(f, "| ")?;
                    } else {
                        write!(f, "  ")?;
                    }
                    write!(f, "{:?} [{:x}] => {:?}", o.key.0, o.key.1, o.value)?
                }
            }
            writeln!(f)?;
        }
        writeln!(f, "}}")?;
        Ok(())
    }
}

impl<V> Table<V> {
    // TODO: maybe change return type to Option<Value>
    pub fn insert(&mut self, key: ObjString, value: V) -> bool {
        self.insert_entry(OccupiedEntry {
            key: Box::new(key),
            value,
        })
    }

    fn insert_entry(&mut self, entry: OccupiedEntry<V>) -> bool {
        if self.len + 1 > self.available_capacity() {
            self.adjust_capacity();
        }

        let index = self.find_entry(&entry.key);
        let new_entry = Entry::Occupied(entry);
        let old_entry = std::mem::replace(&mut self.entries[index], new_entry);
        match old_entry {
            Entry::Empty => {
                self.len += 1;
                true
            }
            Entry::Tombstone => true,
            Entry::Occupied(_) => false,
        }
    }

    pub fn get(&self, key: &ObjString) -> Option<&V> {
        if self.len == 0 {
            return None;
        }

        let index = self.find_entry(key);
        match &self.entries[index] {
            Entry::Empty | Entry::Tombstone => None,
            Entry::Occupied(OccupiedEntry { value, .. }) => Some(value),
        }
    }

    pub fn get_mut(&mut self, key: &ObjString) -> Option<&mut V> {
        if self.len == 0 {
            return None;
        }

        let index = self.find_entry(key);
        match &mut self.entries[index] {
            Entry::Empty | Entry::Tombstone => None,
            Entry::Occupied(OccupiedEntry { value, .. }) => Some(value),
        }
    }

    pub fn remove(&mut self, key: &ObjString) -> Option<V> {
        if self.len == 0 {
            return None;
        }

        let index = self.find_entry(key);
        match &mut self.entries[index] {
            Entry::Empty | Entry::Tombstone => None,
            entry @ Entry::Occupied(_) => {
                let entry = std::mem::replace(entry, Entry::Tombstone);
                // TODO: Remove this unnecessary match
                match entry {
                    Entry::Empty | Entry::Tombstone => unreachable!(),
                    Entry::Occupied(o) => Some(o.value),
                }
            }
        }
    }

    fn capacity(&self) -> usize {
        self.entries.len()
    }

    fn available_capacity(&self) -> usize {
        self.capacity() * MAX_LOAD_FACTOR.0 / MAX_LOAD_FACTOR.1
    }

    fn adjust_capacity(&mut self) {
        let cap = self.capacity();
        let new_cap = if cap < 8 { 8 } else { cap * 2 };
        let entries = std::iter::repeat_with(|| Entry::Empty)
            .take(new_cap)
            .collect();

        let old_entries = std::mem::replace(&mut self.entries, entries);
        self.len = 0;

        for entry in old_entries.into_vec().into_iter() {
            match entry {
                Entry::Occupied(o) => {
                    self.insert_entry(o);
                }
                Entry::Empty | Entry::Tombstone => {}
            }
        }
    }

    fn find_entry(&self, key: &ObjString) -> usize {
        let cap = self.capacity();
        let mut index = key.1 as usize % cap;
        let mut tombstone = None;
        loop {
            let entry = &self.entries[index];
            match entry {
                Entry::Occupied(o) if *o.key == *key => return index,
                Entry::Occupied(_) => {}
                Entry::Empty => return tombstone.unwrap_or(index),
                Entry::Tombstone => {
                    tombstone.get_or_insert(index);
                }
            }
            index = (index + 1) % cap;
        }
    }
}

#[derive(Clone)]
enum Entry<V> {
    Empty,
    Tombstone,
    Occupied(OccupiedEntry<V>),
}

impl<V> Default for Entry<V> {
    fn default() -> Self {
        Self::Empty
    }
}

#[derive(Clone)]
struct OccupiedEntry<V> {
    key: Box<ObjString>,
    value: V,
}

#[cfg(test)]
mod tests;
