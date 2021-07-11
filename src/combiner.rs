// Copyright Christian Daley 2021
// Copyright Frank Mori Hess 2007-2008.
// Distributed under the Boost Software License, Version 1.0. 
// See http://www.boost.org/LICENSE_1_0.txt

use std::iter::Sum;

/// Types that can be used as a combiner for a signal. 
pub trait Combiner<R> {
    /// The return type of the signal. May be different than the return type of
    /// the individual slots.
    type Output;

    /// Combines the results of executing the signal's slots into a single output.
    /// Note that `iter` lazily executes the signal's slots. The first slot will not be
    /// executed until `iter.next()` is called for the first time. The second slot will not
    /// be executed until `iter.next()` is called again, etc. Consuming the `iter` is what
    /// causes slots to be executed. If a custom combiner is created that never uses `iter`,
    /// no slots will ever be executed when the signal is emitted.
    fn combine(&self, iter: impl Iterator<Item=R>) -> Self::Output;
}

#[derive(Default)]
/// The default combiner for signals. Will return an `Option<R>` representing the returned value
/// from the last slot that was executed. If no slots were executed, returns `None`.
pub struct DefaultCombiner {}

impl<R> Combiner<R> for DefaultCombiner {
    type Output = Option<R>;

    fn combine(&self, iter: impl Iterator<Item=R>) -> Option<R> {
        iter.last()
    }
}

#[derive(Default)]
/// A combiner that collects all of the slot's return values into a vector.
pub struct VecCombiner {}

impl<R> Combiner<R> for VecCombiner {
    type Output = Vec<R>;

    fn combine(&self, iter: impl Iterator<Item=R>) -> Vec<R> {
        iter.collect()
    }
}

#[derive(Default)]
/// A combiner that sums all of the slot's return values.
pub struct SumCombiner {}

impl<R> Combiner<R> for SumCombiner 
where
    R: Sum
{
    type Output = R;

    fn combine(&self, iter: impl Iterator<Item=R>) -> R {
        iter.sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_combiner_test() {
        let combiner = DefaultCombiner::default();
        let values1 = vec!(5, 1, 9);
        let values2: Vec<i32> = Vec::new();
        assert_eq!(combiner.combine(values1.into_iter()), Some(9));
        assert_eq!(combiner.combine(values2.into_iter()), None);
    }

    #[test]
    fn vec_combiner_test() {
        let combiner = VecCombiner::default();
        let values1 = vec!(5, 1, 9);
        let values2: Vec<i32> = Vec::new();
        assert_eq!(combiner.combine(values1.iter().cloned()), values1);
        assert_eq!(combiner.combine(values2.iter().cloned()), values2);
    }

    #[test]
    fn sum_combiner_test() {
        let combiner = SumCombiner::default();
        let values1 = vec!(5, 1, 9);
        let values2: Vec<i32> = Vec::new();
        assert_eq!(combiner.combine(values1.iter().cloned()), 15);
        assert_eq!(combiner.combine(values2.iter().cloned()), 0);
    }
}