//! This module aims to provide iterative computation of the base-converted result, starting at the
//! most significant digit.
//! 
//! # Warning
//! This is optimized for passwordmaker-rs domain specific number ranges. If you want to use this
//! somewhere else, make sure to adapt some maths. For instance you might want to early-out for leading zeros.
//! 
//! The maths is not great, sorry. It's way easier to start at the least significant digit...
//! If you have any great idea how to improve it: Make a merge request!

use std::convert::TryInto;
use std::ops::{Mul, DivAssign};
use std::iter::successors;

pub(crate) struct IterativeBaseConversion<V,B>{
    current_value : V,
    current_base_potency : V,
    remaining_digits : usize,
    base : B,
}

impl<V,B> IterativeBaseConversion<V,B> 
    where V: for<'a> From<&'a B>,                           //could be replaced by num::traits::identities::One.
          for<'a> &'a V : Mul<&'a B, Output = Option<V>> +  //used to get the first current_base_potency.
                          Mul<&'a V, Output = Option<V>>
{
    pub(super) fn new(value : V, base : B) -> Self{
        let PotencyAndExponent{potency : current_base_potency, count : remaining_digits} = Self::find_highest_fitting_potency(&base);
        Self{
            current_value : value,
            current_base_potency,
            remaining_digits,
            base,
        }
    }

    fn find_highest_fitting_potency(base : &B) -> PotencyAndExponent<V> {
        let base_v = base.into();

        let exp_result = successors(Some((base_v, 1)), |(p, e)| {
            Some(((p*p)?, 2*e))
        }).last();


        let result = successors(exp_result, |(potency, count)| (potency * base).map(|v| (v, count +1)))
            .last()
            .expect("Cannot fail, first entry is Some (required V : From<B>) and there's no filtering.");
        PotencyAndExponent{ potency : result.0, count : result.1 + 1 }
    }
}

impl<V,B> std::iter::Iterator for IterativeBaseConversion<V,B>
    where V : for<'a> DivAssign<&'a B> + //used between steps to go to next-lower current_base_potency
              RemAssignWithQuotient+     //used to get the result of each step.
              TryInto<B>                 //used to convert the result of each step. We _know_ this cannot fail, but requiring Into would be wrong.
{
    type Item = B;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining_digits == 0 {
            None
        } else {
            let result = self.current_value.rem_assign_with_quotient(&self.current_base_potency);
            
            self.current_base_potency /=  &self.base;
            self.remaining_digits = self.remaining_digits - 1;
            
            //this cannot ever yield None.
            result.try_into().ok()
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining_digits, Some(self.remaining_digits))
    }
}

impl<V,B> std::iter::ExactSizeIterator for IterativeBaseConversion<V,B>
    where IterativeBaseConversion<V,B> : Iterator
{}

struct PotencyAndExponent<V>{
    potency : V,
    count : usize,
}

pub(crate) trait RemAssignWithQuotient{
    /// Replaces self with remainder of division, and returns quotient.
    fn rem_assign_with_quotient(&mut self, divisor : &Self) -> Self;
}

//tests general behaviour, using primitive types.
#[cfg(test)]
mod iterative_conversion_tests{
    use std::{ops::Mul, convert::{From, TryFrom}};

    use super::*;

    #[derive(Debug,Clone)]
    struct MyU128(u128);
    impl Mul<&u64> for &MyU128 {
        type Output = Option<MyU128>;
        fn mul(self, rhs: &u64) -> Self::Output {
            self.0.checked_mul(*rhs as u128).map(|s| MyU128(s))
        }
    }

    impl Mul<&MyU128> for &MyU128 {
        type Output = Option<MyU128>;
        fn mul(self, rhs: &MyU128) -> Self::Output {
            self.0.checked_mul(rhs.0).map(|s| MyU128(s))
     }
    }

    impl RemAssignWithQuotient for MyU128{
        fn rem_assign_with_quotient(&mut self, divisor : &Self) -> Self {
            let quotient = self.0 / divisor.0;
            self.0 %= divisor.0;
            Self(quotient)
        }
    }
    impl From<&u64> for MyU128{
        fn from(v: &u64) -> Self {
            MyU128(v.clone() as u128)
        }
    }

    impl DivAssign<&u64> for MyU128{
        fn div_assign(&mut self, rhs: &u64) {
            self.0 = self.0 / (*rhs as u128);
        }
    }

    impl TryFrom<MyU128> for u64{
        type Error = std::num::TryFromIntError;

        fn try_from(value: MyU128) -> Result<Self, Self::Error> {
            value.0.try_into()
        }
    }


    #[test]
    fn test_simple_u128_to_hex_conversion(){
        let i = IterativeBaseConversion::new(MyU128(12345678u128), 16u64);
        assert_eq!(i.len(), 32);
        assert_eq!(i.skip_while(|x| *x == 0_u64).collect::<Vec<_>>(), vec![0xB, 0xC, 0x6, 0x1, 0x4, 0xE]);
    }
    #[test]
    fn test_simple_u128_to_base_17_conversion(){
        let i = IterativeBaseConversion::new(MyU128(1234567890123456789u128), 17u64);
        assert_eq!(i.len(), 32);
        assert_eq!(i.skip_while(|x| *x == 0_u64).collect::<Vec<_>>(), vec![7, 5, 0xA, 0x10, 0xC, 0xC, 3, 0xD, 3, 0xA, 3,8,4,8,3]);
    }
    #[test]
    fn test_simple_u128_to_base_39_conversion(){
        let i = IterativeBaseConversion::new(MyU128(1234567890123456789u128), 39u64);
        assert_eq!(i.len(), 25);
        // 3YPRS4FaC1KU
        assert_eq!(i.skip_while(|x| *x == 0_u64).collect::<Vec<_>>(), vec![3, 34, 25, 27, 28, 4, 15, 36, 12, 1, 20, 30]);
    }
}