#[derive(Debug)]
pub struct DoubleBuffered<T> {
    pub odd: T,
    pub even: T,
    target_is_odd: bool,
}

impl<T> DoubleBuffered<T> {
    pub fn new(odd: T, even: T) -> Self {
        DoubleBuffered {
            odd,
            even,
            target_is_odd: false,
        }
    }
    pub fn source(&self) -> &T {
        if self.target_is_odd {
            &self.even
        } else {
            &self.odd
        }
    }

    pub fn source_mut(&mut self) -> &mut T {
        if self.target_is_odd {
            &mut self.even
        } else {
            &mut self.odd
        }
    }

    pub fn target(&self) -> &T {
        if self.target_is_odd {
            &self.odd
        } else {
            &self.even
        }
    }

    pub fn target_mut(&mut self) -> &mut T {
        if self.target_is_odd {
            &mut self.odd
        } else {
            &mut self.even
        }
    }

    pub fn source_and_target_mut(&mut self) -> (&T, &mut T) {
        if self.target_is_odd {
            (&self.even, &mut self.odd)
        } else {
            (&self.odd, &mut self.even)
        }
    }

    pub fn swap(&mut self) {
        self.target_is_odd = !self.target_is_odd;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn double_buffered_swap() {
        let mut buffer = DoubleBuffered::new(1, 2);

        assert!(*buffer.source() == 1);
        assert!(*buffer.target() == 2);

        buffer.swap();

        assert!(*buffer.source() == 2);
        assert!(*buffer.target() == 1);
    }

    #[test]
    fn double_buffered_source_and_target_mut() {
        let mut buffer = DoubleBuffered::new(1, 2);

        buffer.swap();

        let (source, target) = buffer.source_and_target_mut();
        assert!(*source == 2);
        assert!(*target == 1);
    }
}
