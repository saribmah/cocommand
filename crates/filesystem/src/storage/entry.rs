//! Entry enum for slab slots.

/// Internal entry representation for slab slots.
#[derive(Clone)]
pub enum Entry<T> {
    /// Slot is free; stores the index of the next free slot in the freelist.
    Vacant(usize),
    /// Slot is occupied by a value.
    Occupied(T),
}
