use std::fmt;

use num_bigint::BigInt;

use builtin_object_derive::BuiltinObject;

use super::cmp::eq_int_float;
use super::float::Float;

/// Built in integer type
#[derive(Debug, PartialEq, BuiltinObject)]
pub struct Int {
    class: Rc<Type>,
    value: BigInt,
}

impl Int {
    pub fn new(class: Rc<Type>, value: BigInt) -> Self {
        Self { class: class.clone(), value }
    }

    pub fn value(&self) -> &BigInt {
        &self.value
    }

    /// Is this Int equal to the specified Float?
    pub fn eq_float(&self, float: &Float) -> bool {
        eq_int_float(self, float)
    }
}

impl fmt::Display for Int {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

// impl From<BigInt> for Int {
//     fn from(value: BigInt) -> Self {
//         Int::new(value)
//     }
// }
//
// macro_rules! int_from {
//     ($($T:ty),+) => { $(
//         impl From<$T> for Int {
//             fn from(value: $T) -> Self {
//                 let value = BigInt::from(value);
//                 Int::new(value)
//             }
//         }
//     )+ };
// }
//
// int_from!(i8, u8, i16, u16, i32, u32, i64, u64, i128, u128);
//
// impl From<f32> for Int {
//     fn from(value: f32) -> Self {
//         let value = BigInt::from_f32(value).unwrap();
//         Int::new(value)
//     }
// }
//
// impl From<f64> for Int {
//     fn from(value: f64) -> Self {
//         let value = BigInt::from_f64(value).unwrap();
//         Int::new(value)
//     }
// }
//
// macro_rules! int_from_string {
//     ($($T:ty),+) => { $(
//         impl From<$T> for Int {
//             fn from(value: $T) -> Self {
//                 let value = BigInt::from_str_radix(value.as_ref(), 10).unwrap();
//                 Int::new(value)
//             }
//         }
//     )+ };
// }
//
// int_from_string!(&str, String, &String);
