use super::node::Node;
use std::ops::{Bound, RangeBounds};

pub struct TreeIterator<'a, R>
where
    R: RangeBounds<Vec<u8>>,
{
    stack: Vec<&'a Node>,
    bounds: R,
}

impl<R> TreeIterator<'_, R>
where
    R: RangeBounds<Vec<u8>>,
{
    pub fn new(root: Option<&Node>, bounds: R) -> TreeIterator<'_, R> {
        if let Some(root) = root {
            TreeIterator {
                stack: vec![root],
                bounds,
            }
        } else {
            TreeIterator {
                stack: Vec::new(),
                bounds,
            }
        }
    }
}

impl<'a, R> Iterator for TreeIterator<'a, R>
where
    R: RangeBounds<Vec<u8>>,
{
    type Item = (&'a [u8], &'a [u8]);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(node) = self.stack.pop() {
            if node.is_leaf() {
                if start_bound_contains(self.bounds.start_bound(), &node.key)
                    && end_bound_contains(self.bounds.end_bound(), &node.key)
                {
                    return Some((&node.key, &node.value));
                }
            } else {
                if end_bound_contains(self.bounds.end_bound(), &node.key) {
                    self.stack.push(node.right.as_ref().unwrap());
                }
                if start_bound_contains_exclusive(self.bounds.start_bound(), &node.key) {
                    self.stack.push(node.left.as_ref().unwrap());
                }
            }
        }
        None
    }
}

impl<R> DoubleEndedIterator for TreeIterator<'_, R>
where
    R: RangeBounds<Vec<u8>>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        while let Some(node) = self.stack.pop() {
            if node.is_leaf() {
                if start_bound_contains(self.bounds.start_bound(), &node.key)
                    && end_bound_contains(self.bounds.end_bound(), &node.key)
                {
                    return Some((&node.key, &node.value));
                }
            } else {
                if start_bound_contains_exclusive(self.bounds.start_bound(), &node.key) {
                    self.stack.push(node.left.as_ref().unwrap());
                }
                if end_bound_contains(self.bounds.end_bound(), &node.key) {
                    self.stack.push(node.right.as_ref().unwrap());
                }
            }
        }
        None
    }
}

fn start_bound_contains<T: Ord>(bound: Bound<T>, key: T) -> bool {
    match bound {
        Bound::Included(b) => key >= b,
        Bound::Excluded(b) => key > b,
        Bound::Unbounded => true,
    }
}

fn start_bound_contains_exclusive<T: Ord>(bound: Bound<T>, key: T) -> bool {
    match bound {
        Bound::Included(b) | Bound::Excluded(b) => key > b,
        Bound::Unbounded => true,
    }
}
fn end_bound_contains<T: Ord>(bound: Bound<T>, key: T) -> bool {
    match bound {
        Bound::Included(b) => key <= b,
        Bound::Excluded(b) => key < b,
        Bound::Unbounded => true,
    }
}
