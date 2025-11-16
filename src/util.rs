use std::marker::PhantomData;

pub trait WhatItertoolsIsMissing {
    /// Return an iterator which gives the current iteration count as well as
    /// the next value but only when the element is `Result::Ok`.
    ///
    /// It maps `Ok(val)` to `Ok((idx, val))` while leaving the error case
    /// alone.
    ///
    /// ```
    /// use util::WhatItertoolsIsMissing;
    ///
    /// let input = vec![Ok(41), Err(false), Ok(11)];
    /// let it = input.into_iter().enumerate_ok()
    /// itertools::assert_equal(it, vec![Ok((0, 42)), Err(false), Ok((1,12))]);
    /// ```
    fn enumerate_ok<T, E>(self) -> EnumerateOk<Self, T, E>
    where
        Self: Iterator<Item = Result<T, E>> + Sized,
    {
        EnumerateOk {
            iter: self,
            last: 0,
            phantom_t: PhantomData,
            phantom_e: PhantomData,
        }
    }
}

impl<I: Iterator> WhatItertoolsIsMissing for I {}

pub struct EnumerateOk<I, T, E> {
    iter: I,
    last: usize,
    phantom_t: PhantomData<T>,
    phantom_e: PhantomData<E>,
}

impl<I, T, E> Iterator for EnumerateOk<I, T, E>
where
    I: Iterator<Item = Result<T, E>>,
{
    type Item = Result<(usize, T), E>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.iter.next();
        match next {
            Some(Ok(val)) => {
                let index = self.last;
                self.last += 1;
                Some(Ok((index, val)))
            }
            Some(Err(e)) => Some(Err(e)),
            None => None,
        }
    }
}
