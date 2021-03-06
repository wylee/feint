//! Integer type
use std::any::Any;
use std::fmt;

use num_bigint::BigInt;
use num_traits::{FromPrimitive, ToPrimitive, Zero};

use crate::vm::{RuntimeBoolResult, RuntimeContext, RuntimeErr, RuntimeObjResult};

use super::builtin_types::BUILTIN_TYPES;
use super::class::TypeRef;
use super::float::Float;
use super::object::{Object, ObjectExt};
use super::util::{eq_int_float, gt_int_float, lt_int_float};

pub struct Int {
    value: BigInt,
}

impl Int {
    pub fn new(value: BigInt) -> Self {
        Self { value }
    }

    pub fn value(&self) -> &BigInt {
        &self.value
    }

    // Cast both LHS and RHS to f64 and divide them
    fn div_f64(&self, rhs: &dyn Object) -> Result<f64, RuntimeErr> {
        let lhs_val = self.value().to_f64().unwrap();
        let rhs_val = if let Some(rhs) = rhs.as_any().downcast_ref::<Self>() {
            rhs.value().to_f64().unwrap()
        } else if let Some(rhs) = rhs.as_any().downcast_ref::<Float>() {
            *rhs.value()
        } else {
            return Err(RuntimeErr::new_type_err(format!(
                "Could not divide {} into Int",
                rhs.type_name()
            )));
        };
        Ok(lhs_val / rhs_val)
    }
}

macro_rules! make_op {
    ( $meth:ident, $op:tt, $message:literal ) => {
        fn $meth(&self, rhs: &dyn Object, ctx: &RuntimeContext) -> RuntimeObjResult {
            if let Some(rhs) = rhs.as_any().downcast_ref::<Self>() {
                // XXX: Return Int
                let value = self.value() $op rhs.value();
                let value = ctx.builtins.new_int(value);
                Ok(value)
            } else if let Some(rhs) = rhs.as_any().downcast_ref::<Float>() {
                // XXX: Return Float
                let value = self.value().to_f64().unwrap() $op rhs.value();
                let value = ctx.builtins.new_float(value);
                Ok(value)
            } else {
                Err(RuntimeErr::new_type_err(format!($message, rhs.type_name())))
            }
        }
    };
}

impl Object for Int {
    fn class(&self) -> &TypeRef {
        BUILTIN_TYPES.get("Int").unwrap()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn negate(&self, ctx: &RuntimeContext) -> RuntimeObjResult {
        Ok(ctx.builtins.new_int(-self.value()))
    }

    fn as_bool(&self, _ctx: &RuntimeContext) -> RuntimeBoolResult {
        Ok(self.value().is_zero())
    }

    fn is_equal(&self, rhs: &dyn Object, _ctx: &RuntimeContext) -> bool {
        if let Some(rhs) = rhs.as_any().downcast_ref::<Self>() {
            self.is(rhs) || self.value() == rhs.value()
        } else if let Some(rhs) = rhs.as_any().downcast_ref::<Float>() {
            eq_int_float(self, rhs)
        } else {
            false
        }
    }

    fn less_than(&self, rhs: &dyn Object, _ctx: &RuntimeContext) -> RuntimeBoolResult {
        if let Some(rhs) = rhs.as_any().downcast_ref::<Self>() {
            Ok(self.value() < rhs.value())
        } else if let Some(rhs) = rhs.as_any().downcast_ref::<Float>() {
            Ok(lt_int_float(self, rhs))
        } else {
            Err(RuntimeErr::new_type_err(format!(
                "Could not compare {} to {}: >",
                rhs.class(),
                self.class(),
            )))
        }
    }

    fn greater_than(
        &self,
        rhs: &dyn Object,
        _ctx: &RuntimeContext,
    ) -> RuntimeBoolResult {
        if let Some(rhs) = rhs.as_any().downcast_ref::<Self>() {
            Ok(self.value() > rhs.value())
        } else if let Some(rhs) = rhs.as_any().downcast_ref::<Float>() {
            Ok(gt_int_float(self, rhs))
        } else {
            Err(RuntimeErr::new_type_err(format!(
                "Could not compare {} to {}: >",
                self.class(),
                rhs.class()
            )))
        }
    }

    fn pow(&self, rhs: &dyn Object, ctx: &RuntimeContext) -> RuntimeObjResult {
        if let Some(rhs) = rhs.as_any().downcast_ref::<Self>() {
            // XXX: Return Int
            let base = self.value();
            let exp = rhs.value().to_u32().unwrap();
            let value = base.pow(exp);
            let value = ctx.builtins.new_int(value);
            Ok(value)
        } else if let Some(rhs) = rhs.as_any().downcast_ref::<Float>() {
            // XXX: Return Float
            let base = self.value().to_f64().unwrap();
            let exp = *rhs.value();
            let value = base.powf(exp);
            let value = ctx.builtins.new_float(value);
            Ok(value)
        } else {
            Err(RuntimeErr::new_type_err(format!(
                "Could not raise {} by {}",
                self.class(),
                rhs.class()
            )))
        }
    }

    make_op!(modulo, %, "Could not divide {} with Int");
    make_op!(mul, *, "Could not multiply {} with Int");
    make_op!(add, +, "Could not add {} to Int");
    make_op!(sub, -, "Could not subtract {} from Int");

    // Int division *always* returns a Float
    fn div(&self, rhs: &dyn Object, ctx: &RuntimeContext) -> RuntimeObjResult {
        let value = self.div_f64(rhs)?;
        let value = ctx.builtins.new_float(value);
        Ok(value)
    }

    // Int *floor* division *always* returns an Int
    fn floor_div(&self, rhs: &dyn Object, ctx: &RuntimeContext) -> RuntimeObjResult {
        let value = self.div_f64(rhs)?;
        let value = BigInt::from_f64(value).unwrap();
        let value = ctx.builtins.new_int(value);
        Ok(value)
    }
}

// Display -------------------------------------------------------------

impl fmt::Display for Int {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value())
    }
}

impl fmt::Debug for Int {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}
