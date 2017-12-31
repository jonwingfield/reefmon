// TODO: share this with the thermostat library instead of copy/pasting
pub mod temp {
    use std::marker::PhantomData;
    use std::cmp::Ordering;
    use std::fmt;
    use std::ops::{Sub , Add, Mul};

    pub enum C {}
    pub enum F {}
    pub struct Temperature<Unit>(pub i32, PhantomData<Unit>);

    // Implements ordering on the first field for a tuple struct
    macro_rules! impl_ord_tuple {
        ( $name:ident for $gen_arg:ident ) => {
            impl<$gen_arg> PartialEq for $name<$gen_arg> {
                fn eq(&self, other: &$name<$gen_arg>) -> bool {
                    self.0.eq(&other.0)
                }
            }

            impl<$gen_arg> PartialOrd for $name<$gen_arg> {
                fn partial_cmp(&self, other: &$name<$gen_arg>) -> Option<Ordering> {
                    self.0.partial_cmp(&other.0)
                }
            }
        }
    }

    impl_ord_tuple!(Temperature for Unit);

    impl fmt::Display for Temperature<C> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}C", self.0 as f32 / 100.0)
        }
    }

    impl fmt::Display for Temperature<F> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}F", self.0 as f32 / 100.0)
        }
    }

    impl fmt::Debug for Temperature<C> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}C", self.0 as f32 / 100.0)
        }
    }

    impl fmt::Debug for Temperature<F> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}F", self.0 as f32 / 100.0)
        }
    }

    impl<Unit> Sub for Temperature<Unit> {
        type Output = Self;
        fn sub(self, _rhs: Self) -> Self {
            Temperature(self.0 - _rhs.0, PhantomData)
        }
    }

    impl<Unit> Add for Temperature<Unit> {
        type Output = Self;
        fn add(self, _rhs: Self) -> Self {
            Temperature(self.0 + _rhs.0, PhantomData)
        }
    }

    impl<Unit> Mul for Temperature<Unit> {
        type Output = Self;
        fn mul(self, _rhs: Self) -> Self {
            Temperature(self.0 * _rhs.0, PhantomData)
        }
    }

    impl<Unit> Copy for Temperature<Unit> {}
    impl<Unit> Clone for Temperature<Unit> {
        fn clone(&self) -> Self {
            Temperature(self.0, PhantomData)
        }
    }

    impl Temperature<C> {
        pub fn in_c(degrees: f32) -> Temperature<C> {
            Temperature((degrees * 100.0) as i32, PhantomData)
        }

        pub fn to_f(&self) -> Temperature<F> {
            Temperature::in_f(((self.0 as f32)/100.0 * (9.0/5.0) + 32.0))
        }

        pub fn value(&self) -> f32 {
            return self.0 as f32 / 100.0;
        }
    }

    impl Temperature<F> {
        pub fn in_f(degrees: f32) -> Temperature<F> {
            Temperature((degrees * 100.0) as i32, PhantomData)
        }

        pub fn to_c(&self) -> Temperature<C> {
            Temperature::in_c((self.0 as f32 / 100.0 - 32.0) * (5.0/9.0))
        }

        pub fn value(&self) -> f32 {
            return self.0 as f32 / 100.0;
        }
    }
}
