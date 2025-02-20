use std::cmp::Ordering;
use std::iter::Peekable;

pub struct MergeIter<I1, I2, K, V>
where
    K: Ord,
    I1: Iterator<Item = (K, Option<V>)>,
    I2: Iterator<Item = (K, V)>,
{
    i1: Peekable<I1>,
    i2: Peekable<I2>,
    reversed: bool,
}

impl<I1, I2, K, V> MergeIter<I1, I2, K, V>
where
    K: Ord,
    I1: Iterator<Item = (K, Option<V>)>,
    I2: Iterator<Item = (K, V)>,
{
    pub fn new(i1: I1, i2: I2, reversed: bool) -> Self {
        MergeIter {
            i1: i1.peekable(),
            i2: i2.peekable(),
            reversed,
        }
    }
}

impl<I1, I2, K, V> Iterator for MergeIter<I1, I2, K, V>
where
    K: Ord,
    I1: Iterator<Item = (K, Option<V>)>,
    I2: Iterator<Item = (K, V)>,
{
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        match (self.i1.peek(), self.i2.peek()) {
            (Some((ref k1, _)), Some((ref k2, _))) => {
                let mut ordering = k1.cmp(k2);
                if self.reversed {
                    ordering = ordering.reverse();
                }
                match ordering {
                    Ordering::Less => match self.i1.next() {
                        Some((_, None)) => self.next(),
                        Some((k, Some(v))) => Some((k, v)),
                        None => None,
                    },
                    Ordering::Equal => {
                        self.i2.next();
                        match self.i1.next() {
                            Some((_, None)) => self.next(),
                            Some((k, Some(v))) => Some((k, v)),
                            None => None,
                        }
                    }
                    Ordering::Greater => self.i2.next(),
                }
            }
            (Some(_), None) => match self.i1.next() {
                Some((_, None)) => self.next(),
                Some((k, Some(v))) => Some((k, v)),
                None => None,
            },
            (None, Some(_)) => self.i2.next(),
            (None, None) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_iter() {
        let i1 = [(1, Some("a")), (2, None), (3, Some("c"))];
        let i2 = [(1, "A"), (2, "B"), (4, "D")];

        assert_eq!(
            MergeIter::new(i1.iter().cloned(), i2.iter().cloned(), false).collect::<Vec<_>>(),
            vec![(1, "a"), (3, "c"), (4, "D")]
        );

        assert_eq!(
            MergeIter::new(i1.iter().rev().cloned(), i2.iter().rev().cloned(), true)
                .collect::<Vec<_>>(),
            vec![(4, "D"), (3, "c"), (1, "a")]
        );
    }
}
