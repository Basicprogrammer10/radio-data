pub struct ValueRepeat<I: Iterator> {
    iter: I,
    val: Option<<I as Iterator>::Item>,
    reps: usize,
    i: usize,
}

impl<I: Iterator> ValueRepeat<I> {
    pub fn new(mut reps: usize, iter: I) -> Self {
        reps -= 1;
        Self {
            iter,
            reps,
            val: None,
            i: reps,
        }
    }
}

impl<I> Iterator for ValueRepeat<I>
where
    I: Iterator,
    <I as Iterator>::Item: Clone,
{
    type Item = <I as Iterator>::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.val.is_none() || self.i == 0 {
            self.i = self.reps;
            self.val = Some(self.iter.next()?);
            return self.val.clone();
        }

        self.i -= 1;
        return self.val.clone();
    }
}

trait IntoValueRepeat<I>
where
    I: Iterator,
{
    fn repeat(self, rep: usize) -> ValueRepeat<I>;
}

impl<I> IntoValueRepeat<I> for I where I: Iterator {
    fn repeat(self, reps: usize) -> ValueRepeat<I> {
        ValueRepeat::new(reps, self)
    }
}

#[cfg(test)]
mod test {
    use crate::misc::IntoValueRepeat;

    #[test]
    fn test_value_repeat_iter() {
        let mut iter = [1, 2, 3].iter().repeat(3);
        
        for i in [1, 1, 1, 2, 2, 2, 3, 3, 3] {
            assert_eq!(i, *iter.next().unwrap());
        }
    }
}