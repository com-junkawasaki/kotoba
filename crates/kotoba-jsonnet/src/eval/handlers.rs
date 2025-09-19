//! Handler traits for extensible Jsonnet evaluation

use crate::ast::{BinaryOp, UnaryOp, Expr};
use crate::error::Result;
use crate::value::JsonnetValue;
use crate::eval::Context;

/// Handler for binary and unary operations
pub trait OpHandler {
    /// Evaluate a binary operation (e.g., 2 + 3, "hello" + "world")
    fn eval_binary_op(&mut self, context: &mut Context, left: JsonnetValue, op: BinaryOp, right: JsonnetValue) -> Result<JsonnetValue>;

    /// Evaluate a unary operation (e.g., -5, !true)
    fn eval_unary_op(&mut self, context: &mut Context, op: UnaryOp, operand: JsonnetValue) -> Result<JsonnetValue>;
}

/// Handler for function calls (user-defined, builtin, external)
pub trait FuncallHandler {
    /// Call a function with given arguments
    fn call_function(&mut self, context: &mut Context, func: JsonnetValue, args: Vec<JsonnetValue>) -> Result<JsonnetValue>;

    /// Check if a function name refers to a builtin function
    fn is_builtin_function(&self, name: &str) -> bool;

    /// Call a builtin function
    fn call_builtin_function(&mut self, context: &mut Context, name: &str, args: Vec<JsonnetValue>) -> Result<JsonnetValue>;

    /// Check if a function name refers to an external function
    fn is_external_function(&self, name: &str) -> bool;

    /// Call an external function (HTTP, AI API, system commands, etc.)
    fn call_external_function(&mut self, context: &mut Context, name: &str, args: Vec<JsonnetValue>) -> Result<JsonnetValue>;
}

/// Handler for comprehensions (list/dict comprehensions)
pub trait ComprehensionHandler {
    /// Evaluate a list comprehension [expr for var in collection if condition]
    fn eval_list_comprehension(&mut self, context: &mut Context, expr: &Expr, var: &str, collection: &Expr, condition: Option<&Expr>) -> Result<JsonnetValue>;

    /// Evaluate a dict comprehension {key: value for var in collection if condition}
    fn eval_dict_comprehension(&mut self, context: &mut Context, key_expr: &Expr, value_expr: &Expr, var: &str, collection: &Expr, condition: Option<&Expr>) -> Result<JsonnetValue>;
}

/// Default implementations for handlers

/// Default operation handler with standard Jsonnet semantics
pub struct DefaultOpHandler;

impl OpHandler for DefaultOpHandler {
    fn eval_binary_op(&mut self, _context: &mut Context, left: JsonnetValue, op: BinaryOp, right: JsonnetValue) -> Result<JsonnetValue> {
        match op {
            BinaryOp::Add => self.eval_add(left, right),
            BinaryOp::Sub => self.eval_sub(left, right),
            BinaryOp::Mul => self.eval_mul(left, right),
            BinaryOp::Div => self.eval_div(left, right),
            BinaryOp::Mod => self.eval_mod(left, right),
            BinaryOp::Lt => self.eval_lt(left, right),
            BinaryOp::Le => self.eval_le(left, right),
            BinaryOp::Gt => self.eval_gt(left, right),
            BinaryOp::Ge => self.eval_ge(left, right),
            BinaryOp::Eq => self.eval_eq(left, right),
            BinaryOp::Ne => self.eval_ne(left, right),
            BinaryOp::And => self.eval_and(left, right),
            BinaryOp::Or => self.eval_or(left, right),
            BinaryOp::In => self.eval_in(left, right),
            BinaryOp::BitAnd => self.eval_bitwise_and(left, right),
            BinaryOp::BitOr => self.eval_bitwise_or(left, right),
            BinaryOp::BitXor => self.eval_bitwise_xor(left, right),
            BinaryOp::ShiftL => self.eval_lsh(left, right),
            BinaryOp::ShiftR => self.eval_rsh(left, right),
        }
    }

    fn eval_unary_op(&mut self, _context: &mut Context, op: UnaryOp, operand: JsonnetValue) -> Result<JsonnetValue> {
        match op {
            UnaryOp::Not => self.eval_not(operand),
            UnaryOp::Neg => self.eval_neg(operand),
            UnaryOp::Pos => self.eval_pos(operand),
            UnaryOp::BitNot => self.eval_bitwise_not(operand),
        }
    }
}

impl DefaultOpHandler {
    fn eval_add(&self, left: JsonnetValue, right: JsonnetValue) -> Result<JsonnetValue> {
        match (&left, &right) {
            (JsonnetValue::Number(l), JsonnetValue::Number(r)) => Ok(JsonnetValue::Number(l + r)),
            (JsonnetValue::String(l), JsonnetValue::String(r)) => Ok(JsonnetValue::String(format!("{}{}", l, r))),
            (JsonnetValue::Array(l), JsonnetValue::Array(r)) => {
                let mut result = l.clone();
                result.extend(r.clone());
                Ok(JsonnetValue::Array(result))
            }
            (JsonnetValue::Object(l), JsonnetValue::Object(r)) => {
                let mut result = l.clone();
                for (k, v) in r {
                    result.insert(k, v);
                }
                Ok(JsonnetValue::Object(result))
            }
            _ => Err(crate::error::JsonnetError::runtime_error(format!("Cannot add {} and {}", left.type_name(), right.type_name()))),
        }
    }

    fn eval_sub(&self, left: JsonnetValue, right: JsonnetValue) -> Result<JsonnetValue> {
        match (&left, &right) {
            (JsonnetValue::Number(l), JsonnetValue::Number(r)) => Ok(JsonnetValue::Number(l - r)),
            _ => Err(crate::error::JsonnetError::runtime_error(format!("Cannot subtract {} from {}", right.type_name(), left.type_name()))),
        }
    }

    fn eval_mul(&self, left: JsonnetValue, right: JsonnetValue) -> Result<JsonnetValue> {
        match (&left, &right) {
            (JsonnetValue::Number(l), JsonnetValue::Number(r)) => Ok(JsonnetValue::Number(l * r)),
            _ => Err(crate::error::JsonnetError::runtime_error(format!("Cannot multiply {} and {}", left.type_name(), right.type_name()))),
        }
    }

    fn eval_div(&self, left: JsonnetValue, right: JsonnetValue) -> Result<JsonnetValue> {
        match (&left, &right) {
            (JsonnetValue::Number(l), JsonnetValue::Number(r)) => {
                if *r == 0.0 {
                    return Err(crate::error::JsonnetError::runtime_error("Division by zero"));
                }
                Ok(JsonnetValue::Number(l / r))
            }
            _ => Err(crate::error::JsonnetError::runtime_error(format!("Cannot divide {} by {}", left.type_name(), right.type_name()))),
        }
    }

    fn eval_mod(&self, left: JsonnetValue, right: JsonnetValue) -> Result<JsonnetValue> {
        match (&left, &right) {
            (JsonnetValue::Number(l), JsonnetValue::Number(r)) => {
                if *r == 0.0 {
                    return Err(crate::error::JsonnetError::runtime_error("Modulo by zero"));
                }
                Ok(JsonnetValue::Number(l % r))
            }
            _ => Err(crate::error::JsonnetError::runtime_error(format!("Cannot compute {} mod {}", left.type_name(), right.type_name()))),
        }
    }

    fn eval_lt(&self, left: JsonnetValue, right: JsonnetValue) -> Result<JsonnetValue> {
        self.compare_values(&left, &right, |a, b| match (a, b) {
            (JsonnetValue::Number(l), JsonnetValue::Number(r)) => Some(l < r),
            (JsonnetValue::String(l), JsonnetValue::String(r)) => Some(l < r),
            _ => None,
        }).map(JsonnetValue::Boolean)
    }

    fn eval_le(&self, left: JsonnetValue, right: JsonnetValue) -> Result<JsonnetValue> {
        self.compare_values(&left, &right, |a, b| match (a, b) {
            (JsonnetValue::Number(l), JsonnetValue::Number(r)) => Some(l <= r),
            (JsonnetValue::String(l), JsonnetValue::String(r)) => Some(l <= r),
            _ => None,
        }).map(JsonnetValue::Boolean)
    }

    fn eval_gt(&self, left: JsonnetValue, right: JsonnetValue) -> Result<JsonnetValue> {
        self.compare_values(&left, &right, |a, b| match (a, b) {
            (JsonnetValue::Number(l), JsonnetValue::Number(r)) => Some(l > r),
            (JsonnetValue::String(l), JsonnetValue::String(r)) => Some(l > r),
            _ => None,
        }).map(JsonnetValue::Boolean)
    }

    fn eval_ge(&self, left: JsonnetValue, right: JsonnetValue) -> Result<JsonnetValue> {
        self.compare_values(&left, &right, |a, b| match (a, b) {
            (JsonnetValue::Number(l), JsonnetValue::Number(r)) => Some(l >= r),
            (JsonnetValue::String(l), JsonnetValue::String(r)) => Some(l >= r),
            _ => None,
        }).map(JsonnetValue::Boolean)
    }

    fn eval_eq(&self, left: JsonnetValue, right: JsonnetValue) -> Result<JsonnetValue> {
        Ok(JsonnetValue::Boolean(left == right))
    }

    fn eval_ne(&self, left: JsonnetValue, right: JsonnetValue) -> Result<JsonnetValue> {
        Ok(JsonnetValue::Boolean(left != right))
    }

    fn compare_values<F>(&self, left: &JsonnetValue, right: &JsonnetValue, cmp: F) -> Result<bool>
    where
        F: FnOnce(&JsonnetValue, &JsonnetValue) -> Option<bool>,
    {
        if let Some(result) = cmp(left, right) {
            Ok(result)
        } else {
            Err(crate::error::JsonnetError::runtime_error(format!(
                "Cannot compare {} and {}",
                left.type_name(),
                right.type_name()
            )))
        }
    }

    fn eval_and(&self, left: JsonnetValue, right: JsonnetValue) -> Result<JsonnetValue> {
        Ok(JsonnetValue::Boolean(left.is_truthy() && right.is_truthy()))
    }

    fn eval_or(&self, left: JsonnetValue, right: JsonnetValue) -> Result<JsonnetValue> {
        Ok(JsonnetValue::Boolean(left.is_truthy() || right.is_truthy()))
    }

    fn eval_in(&self, left: JsonnetValue, right: JsonnetValue) -> Result<JsonnetValue> {
        match (&left, &right) {
            (JsonnetValue::String(key), JsonnetValue::Object(obj)) => {
                Ok(JsonnetValue::Boolean(obj.contains_key(key)))
            }
            _ => Ok(JsonnetValue::Boolean(false)),
        }
    }

    fn eval_bitwise_and(&self, left: JsonnetValue, right: JsonnetValue) -> Result<JsonnetValue> {
        match (&left, &right) {
            (JsonnetValue::Number(l), JsonnetValue::Number(r)) => {
                Ok(JsonnetValue::Number(((*l as i64) & (*r as i64)) as f64))
            }
            _ => Err(crate::error::JsonnetError::runtime_error(format!("Cannot compute bitwise AND of {} and {}", left.type_name(), right.type_name()))),
        }
    }

    fn eval_bitwise_or(&self, left: JsonnetValue, right: JsonnetValue) -> Result<JsonnetValue> {
        match (&left, &right) {
            (JsonnetValue::Number(l), JsonnetValue::Number(r)) => {
                Ok(JsonnetValue::Number(((*l as i64) | (*r as i64)) as f64))
            }
            _ => Err(crate::error::JsonnetError::runtime_error(format!("Cannot compute bitwise OR of {} and {}", left.type_name(), right.type_name()))),
        }
    }

    fn eval_bitwise_xor(&self, left: JsonnetValue, right: JsonnetValue) -> Result<JsonnetValue> {
        match (&left, &right) {
            (JsonnetValue::Number(l), JsonnetValue::Number(r)) => {
                Ok(JsonnetValue::Number(((*l as i64) ^ (*r as i64)) as f64))
            }
            _ => Err(crate::error::JsonnetError::runtime_error(format!("Cannot compute bitwise XOR of {} and {}", left.type_name(), right.type_name()))),
        }
    }

    fn eval_lsh(&self, left: JsonnetValue, right: JsonnetValue) -> Result<JsonnetValue> {
        match (&left, &right) {
            (JsonnetValue::Number(l), JsonnetValue::Number(r)) => {
                Ok(JsonnetValue::Number(((*l as i64) << (*r as i64)) as f64))
            }
            _ => Err(crate::error::JsonnetError::runtime_error(format!("Cannot left shift {} by {}", left.type_name(), right.type_name()))),
        }
    }

    fn eval_rsh(&self, left: JsonnetValue, right: JsonnetValue) -> Result<JsonnetValue> {
        match (&left, &right) {
            (JsonnetValue::Number(l), JsonnetValue::Number(r)) => {
                Ok(JsonnetValue::Number(((*l as i64) >> (*r as i64)) as f64))
            }
            _ => Err(crate::error::JsonnetError::runtime_error(format!("Cannot right shift {} by {}", left.type_name(), right.type_name()))),
        }
    }

    fn eval_not(&self, operand: JsonnetValue) -> Result<JsonnetValue> {
        Ok(JsonnetValue::Boolean(!operand.is_truthy()))
    }

    fn eval_neg(&self, operand: JsonnetValue) -> Result<JsonnetValue> {
        match operand {
            JsonnetValue::Number(n) => Ok(JsonnetValue::Number(-n)),
            _ => Err(crate::error::JsonnetError::runtime_error(format!("Cannot negate {}", operand.type_name()))),
        }
    }

    fn eval_pos(&self, operand: JsonnetValue) -> Result<JsonnetValue> {
        match operand {
            JsonnetValue::Number(n) => Ok(JsonnetValue::Number(n)),
            _ => Err(crate::error::JsonnetError::runtime_error(format!("Cannot apply unary plus to {}", operand.type_name()))),
        }
    }

    fn eval_bitwise_not(&self, operand: JsonnetValue) -> Result<JsonnetValue> {
        match operand {
            JsonnetValue::Number(n) => Ok(JsonnetValue::Number((!(n as i64)) as f64)),
            _ => Err(crate::error::JsonnetError::runtime_error(format!("Cannot apply bitwise NOT to {}", operand.type_name()))),
        }
    }
}

/// Default function call handler
pub struct DefaultFuncallHandler;

impl FuncallHandler for DefaultFuncallHandler {
    fn call_function(&mut self, context: &mut Context, func: JsonnetValue, args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        match func {
            JsonnetValue::Function(_) => {
                // This should be handled by the evaluator's FunctionCallback implementation
                Err(crate::error::JsonnetError::runtime_error("Function call should be handled by evaluator"))
            }
            JsonnetValue::Builtin(builtin) => {
                // Use the evaluator as the callback for stdlib functions
                builtin.call_with_callback(context as &mut dyn crate::stdlib::FunctionCallback, args)
            }
            _ => Err(crate::error::JsonnetError::runtime_error(format!("Cannot call {}", func.type_name()))),
        }
    }

    fn is_builtin_function(&self, name: &str) -> bool {
        // Check if it's a std.* function or other builtins
        name.starts_with("std.") || name == "length"
    }

    fn call_builtin_function(&mut self, context: &mut Context, name: &str, args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        if name == "length" {
            crate::stdlib::StdLib::length(args)
        } else if name.starts_with("std.") {
            let func_name = &name[4..]; // Remove "std." prefix
            let mut stdlib = crate::stdlib::StdLibWithCallback::new(context as &mut dyn crate::stdlib::FunctionCallback);
            stdlib.call_function(func_name, args)
        } else {
            Err(crate::error::JsonnetError::runtime_error(format!("Unknown builtin function: {}", name)))
        }
    }

    fn is_external_function(&self, name: &str) -> bool {
        // Check for external functions like ai.*, tool.*, memory.*, agent.*, chain.*, and import/importstr
        name.starts_with("ai.") ||
        name.starts_with("tool.") ||
        name.starts_with("memory.") ||
        name.starts_with("agent.") ||
        name.starts_with("chain.") ||
        name == "import" ||
        name == "importstr"
    }

    fn call_external_function(&mut self, context: &mut Context, name: &str, args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        // This should be handled by the evaluator's FunctionCallback implementation for import/importstr
        Err(crate::error::JsonnetError::runtime_error(format!("External function {} should be handled by evaluator", name)))
    }
}

/// Default comprehension handler
pub struct DefaultComprehensionHandler;

impl ComprehensionHandler for DefaultComprehensionHandler {
    fn eval_list_comprehension(&mut self, context: &mut Context, expr: &Expr, var: &str, collection: &Expr, condition: Option<&Expr>) -> Result<JsonnetValue> {
        // This is a simplified implementation - in a real implementation, we'd need access to the evaluator
        // For now, we'll return an error indicating this needs to be handled differently
        Err(crate::error::JsonnetError::runtime_error("List comprehension evaluation requires evaluator access"))
    }

    fn eval_dict_comprehension(&mut self, context: &mut Context, key_expr: &Expr, value_expr: &Expr, var: &str, collection: &Expr, condition: Option<&Expr>) -> Result<JsonnetValue> {
        // This is a simplified implementation - in a real implementation, we'd need access to the evaluator
        Err(crate::error::JsonnetError::runtime_error("Dict comprehension evaluation requires evaluator access"))
    }
}
