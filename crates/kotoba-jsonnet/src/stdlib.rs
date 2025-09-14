//! Jsonnet standard library implementation

use crate::error::{JsonnetError, Result};
use crate::value::JsonnetValue;
use std::collections::HashMap;

/// Standard library function implementations
pub struct StdLib;

impl StdLib {
    /// Call a standard library function
    pub fn call_function(name: &str, args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        match name {
            "length" => Self::length(args),
            "type" => Self::type_of(args),
            "makeArray" => Self::make_array(args),
            "filter" => Self::filter(args),
            "map" => Self::map(args),
            "foldl" => Self::foldl(args),
            "foldr" => Self::foldr(args),
            "range" => Self::range(args),
            "join" => Self::join(args),
            "split" => Self::split(args),
            "contains" => Self::contains(args),
            "startsWith" => Self::starts_with(args),
            "endsWith" => Self::ends_with(args),
            "substr" => Self::substr(args),
            "char" => Self::char_fn(args),
            "codepoint" => Self::codepoint(args),
            "toString" => Self::to_string(args),
            "parseInt" => Self::parse_int(args),
            "parseJson" => Self::parse_json(args),
            "encodeUTF8" => Self::encode_utf8(args),
            "decodeUTF8" => Self::decode_utf8(args),
            "md5" => Self::md5(args),
            "base64" => Self::base64(args),
            "base64Decode" => Self::base64_decode(args),
            "manifestJson" => Self::manifest_json(args),
            "manifestJsonEx" => Self::manifest_json_ex(args),
            "manifestYaml" => Self::manifest_yaml(args),
            "escapeStringJson" => Self::escape_string_json(args),
            "escapeStringYaml" => Self::escape_string_yaml(args),
            "escapeStringPython" => Self::escape_string_python(args),
            "escapeStringBash" => Self::escape_string_bash(args),
            "escapeStringDollars" => Self::escape_string_dollars(args),
            "stringChars" => Self::string_chars(args),
            "stringBytes" => Self::string_bytes(args),
            "format" => Self::format(args),
            "isArray" => Self::is_array(args),
            "isBoolean" => Self::is_boolean(args),
            "isFunction" => Self::is_function(args),
            "isNumber" => Self::is_number(args),
            "isObject" => Self::is_object(args),
            "isString" => Self::is_string(args),
            "count" => Self::count(args),
            "find" => Self::find(args),
            "member" => Self::member(args),
            "modulo" => Self::modulo(args),
            "pow" => Self::pow(args),
            "exp" => Self::exp(args),
            "log" => Self::log(args),
            "sqrt" => Self::sqrt(args),
            "sin" => Self::sin(args),
            "cos" => Self::cos(args),
            "tan" => Self::tan(args),
            "asin" => Self::asin(args),
            "acos" => Self::acos(args),
            "atan" => Self::atan(args),
            "floor" => Self::floor(args),
            "ceil" => Self::ceil(args),
            "round" => Self::round(args),
            "abs" => Self::abs(args),
            "max" => Self::max(args),
            "min" => Self::min(args),
            "clamp" => Self::clamp(args),
            "assertEqual" => Self::assert_equal(args),
            "sort" => Self::sort(args),
            "uniq" => Self::uniq(args),
            "reverse" => Self::reverse(args),
            "mergePatch" => Self::merge_patch(args),
            "get" => Self::get(args),
            "objectFields" => Self::object_fields(args),
            "objectFieldsAll" => Self::object_fields_all(args),
            "objectHas" => Self::object_has(args),
            "objectHasAll" => Self::object_has_all(args),
            "objectValues" => Self::object_values(args),
            "objectValuesAll" => Self::object_values_all(args),
            "prune" => Self::prune(args),
            "mapWithKey" => Self::map_with_key(args),
            _ => Err(JsonnetError::runtime_error(format!("Unknown std function: {}", name))),
        }
    }

    /// std.length(x) - returns length of array, string, or object
    pub fn length(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "length")?;
        match &args[0] {
            JsonnetValue::Array(arr) => Ok(JsonnetValue::number(arr.len() as f64)),
            JsonnetValue::String(s) => Ok(JsonnetValue::number(s.len() as f64)),
            JsonnetValue::Object(obj) => Ok(JsonnetValue::number(obj.len() as f64)),
            _ => Err(JsonnetError::type_error("length() requires array, string, or object")),
        }
    }

    /// std.type(x) - returns type of value as string
    fn type_of(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "type")?;
        let type_str = args[0].type_name();
        Ok(JsonnetValue::string(type_str))
    }

    /// std.makeArray(n, func) - creates array by calling func n times
    fn make_array(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 2, "makeArray")?;
        let _n = args[0].as_number()? as usize;
        let _func = &args[1];

        // TODO: Function call implementation needed
        // For now, return empty array
        Ok(JsonnetValue::array(vec![]))
    }

    /// std.filter(func, arr) - filters array using predicate function
    fn filter(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 2, "filter")?;
        // TODO: Implement filtering
        Ok(args[1].clone())
    }

    /// std.map(func, arr) - maps function over array
    fn map(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 2, "map")?;
        // TODO: Implement mapping
        Ok(args[1].clone())
    }

    /// std.foldl(func, arr, init) - left fold
    fn foldl(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 3, "foldl")?;
        // TODO: Implement folding
        Ok(args[2].clone())
    }

    /// std.foldr(func, arr, init) - right fold
    fn foldr(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 3, "foldr")?;
        // TODO: Implement folding
        Ok(args[2].clone())
    }

    /// std.range(n) - creates array [0, 1, ..., n-1]
    fn range(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "range")?;
        let n = args[0].as_number()? as usize;
        let arr: Vec<JsonnetValue> = (0..n).map(|i| JsonnetValue::number(i as f64)).collect();
        Ok(JsonnetValue::array(arr))
    }

    /// std.join(sep, arr) - joins array elements with separator
    fn join(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 2, "join")?;
        let sep = args[0].as_string()?;
        let arr = args[1].as_array()?;

        let mut result = String::new();
        for (i, item) in arr.iter().enumerate() {
            if i > 0 {
                result.push_str(sep);
            }
            result.push_str(&item.to_string());
        }

        Ok(JsonnetValue::string(result))
    }

    /// std.split(str, sep) - splits string by separator
    fn split(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 2, "split")?;
        let s = args[0].as_string()?;
        let sep = args[1].as_string()?;

        let parts: Vec<JsonnetValue> = s.split(sep).map(JsonnetValue::string).collect();
        Ok(JsonnetValue::array(parts))
    }

    /// std.contains(arr, elem) - checks if array contains element
    fn contains(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 2, "contains")?;
        let arr = args[0].as_array()?;
        let contains = arr.iter().any(|item| item.equals(&args[1]));
        Ok(JsonnetValue::boolean(contains))
    }

    /// std.startsWith(str, prefix) - checks if string starts with prefix
    fn starts_with(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 2, "startsWith")?;
        let s = args[0].as_string()?;
        let prefix = args[1].as_string()?;
        Ok(JsonnetValue::boolean(s.starts_with(prefix)))
    }

    /// std.endsWith(str, suffix) - checks if string ends with suffix
    fn ends_with(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 2, "endsWith")?;
        let s = args[0].as_string()?;
        let suffix = args[1].as_string()?;
        Ok(JsonnetValue::boolean(s.ends_with(suffix)))
    }

    /// std.substr(str, from, len) - extracts substring
    fn substr(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 3, "substr")?;
        let s = args[0].as_string()?;
        let from = args[1].as_number()? as usize;
        let len = args[2].as_number()? as usize;

        let substr = if from >= s.len() {
            ""
        } else {
            let end = (from + len).min(s.len());
            &s[from..end]
        };

        Ok(JsonnetValue::string(substr))
    }

    /// std.char(n) - returns character for codepoint
    fn char_fn(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "char")?;
        let n = args[0].as_number()? as u32;
        match char::from_u32(n) {
            Some(c) => Ok(JsonnetValue::string(c.to_string())),
            None => Err(JsonnetError::runtime_error("Invalid codepoint")),
        }
    }

    /// std.codepoint(str) - returns codepoint of first character
    fn codepoint(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "codepoint")?;
        let s = args[0].as_string()?;
        match s.chars().next() {
            Some(c) => Ok(JsonnetValue::number(c as u32 as f64)),
            None => Err(JsonnetError::runtime_error("Empty string")),
        }
    }

    /// std.toString(x) - converts value to string
    fn to_string(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "toString")?;
        Ok(JsonnetValue::string(args[0].to_string()))
    }

    /// std.parseInt(str) - parses string as integer
    fn parse_int(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "parseInt")?;
        let s = args[0].as_string()?;
        match s.parse::<f64>() {
            Ok(n) => Ok(JsonnetValue::number(n)),
            Err(_) => Err(JsonnetError::runtime_error("Invalid number format")),
        }
    }

    /// std.parseJson(str) - parses JSON string
    fn parse_json(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "parseJson")?;
        let s = args[0].as_string()?;
        match serde_json::from_str::<serde_json::Value>(s) {
            Ok(value) => Ok(JsonnetValue::from_json_value(value)),
            Err(_) => Err(JsonnetError::runtime_error("Invalid JSON")),
        }
    }

    /// std.encodeUTF8(str) - encodes string as UTF-8 bytes
    fn encode_utf8(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "encodeUTF8")?;
        let s = args[0].as_string()?;
        let bytes: Vec<JsonnetValue> = s.as_bytes().iter().map(|&b| JsonnetValue::number(b as f64)).collect();
        Ok(JsonnetValue::array(bytes))
    }

    /// std.decodeUTF8(arr) - decodes UTF-8 bytes to string
    fn decode_utf8(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "decodeUTF8")?;
        let arr = args[0].as_array()?;
        let mut bytes = Vec::new();
        for item in arr {
            let b = item.as_number()? as u8;
            bytes.push(b);
        }
        match String::from_utf8(bytes) {
            Ok(s) => Ok(JsonnetValue::string(s)),
            Err(_) => Err(JsonnetError::runtime_error("Invalid UTF-8 sequence")),
        }
    }

    /// std.md5(str) - computes MD5 hash
    fn md5(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "md5")?;
        let s = args[0].as_string()?;
        use md5::{Md5, Digest};
        let mut hasher = Md5::new();
        hasher.update(s.as_bytes());
        let result = hasher.finalize();
        Ok(JsonnetValue::string(format!("{:x}", result)))
    }

    /// std.base64(str) - base64 encodes string
    fn base64(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "base64")?;
        let s = args[0].as_string()?;
        use base64::{Engine as _, engine::general_purpose};
        let encoded = general_purpose::STANDARD.encode(s.as_bytes());
        Ok(JsonnetValue::string(encoded))
    }

    /// std.base64Decode(str) - base64 decodes string
    fn base64_decode(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "base64Decode")?;
        let s = args[0].as_string()?;
        use base64::{Engine as _, engine::general_purpose};
        match general_purpose::STANDARD.decode(s.as_bytes()) {
            Ok(bytes) => match String::from_utf8(bytes) {
                Ok(decoded) => Ok(JsonnetValue::string(decoded)),
                Err(_) => Err(JsonnetError::runtime_error("Invalid UTF-8 in decoded data")),
            },
            Err(_) => Err(JsonnetError::runtime_error("Invalid base64")),
        }
    }

    /// std.manifestJson(x) - pretty prints value as JSON
    fn manifest_json(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "manifestJson")?;
        let json = serde_json::to_string_pretty(&args[0].to_json_value())?;
        Ok(JsonnetValue::string(json))
    }

    /// std.manifestJsonEx(x, indent) - pretty prints value as JSON with custom indent
    fn manifest_json_ex(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 2, "manifestJsonEx")?;
        // TODO: Implement custom indentation
        Self::manifest_json(vec![args[0].clone()])
    }

    /// std.manifestYaml(x) - pretty prints value as YAML
    #[cfg(feature = "yaml")]
    fn manifest_yaml(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "manifestYaml")?;
        let yaml = serde_yaml::to_string(&args[0].to_json_value())?;
        Ok(JsonnetValue::string(yaml))
    }

    /// std.manifestYaml(x) - pretty prints value as YAML (fallback when yaml feature disabled)
    #[cfg(not(feature = "yaml"))]
    fn manifest_yaml(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "manifestYaml")?;
        // Fallback to JSON when YAML feature is disabled
        Self::manifest_json(args)
    }

    // String escaping functions
    fn escape_string_json(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "escapeStringJson")?;
        let s = args[0].as_string()?;
        let escaped = serde_json::to_string(s)?;
        Ok(JsonnetValue::string(escaped))
    }

    #[cfg(feature = "yaml")]
    fn escape_string_yaml(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "escapeStringYaml")?;
        // TODO: Implement proper YAML escaping
        Self::escape_string_json(args)
    }

    #[cfg(not(feature = "yaml"))]
    fn escape_string_yaml(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "escapeStringYaml")?;
        // Fallback to JSON escaping when YAML feature is disabled
        Self::escape_string_json(args)
    }

    fn escape_string_python(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "escapeStringPython")?;
        let s = args[0].as_string()?;
        let escaped = s.escape_default().to_string();
        Ok(JsonnetValue::string(format!("'{}'", escaped)))
    }

    fn escape_string_bash(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "escapeStringBash")?;
        let s = args[0].as_string()?;
        let escaped = s.replace("'", "'\"'\"'").replace("\\", "\\\\");
        Ok(JsonnetValue::string(format!("'{}'", escaped)))
    }

    fn escape_string_dollars(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "escapeStringDollars")?;
        let s = args[0].as_string()?;
        let escaped = s.replace("$$", "$").replace("$", "$$");
        Ok(JsonnetValue::string(escaped))
    }

    // Additional string functions
    fn string_chars(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "stringChars")?;
        let s = args[0].as_string()?;
        let chars: Vec<JsonnetValue> = s.chars().map(|c| JsonnetValue::string(c.to_string())).collect();
        Ok(JsonnetValue::array(chars))
    }

    fn string_bytes(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "stringBytes")?;
        let s = args[0].as_string()?;
        let bytes: Vec<JsonnetValue> = s.as_bytes().iter().map(|&b| JsonnetValue::number(b as f64)).collect();
        Ok(JsonnetValue::array(bytes))
    }

    fn format(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        // TODO: Implement format function
        Self::check_args(&args, 2, "format")?;
        Ok(JsonnetValue::string("<formatted>".to_string()))
    }

    // Type checking functions
    fn is_array(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "isArray")?;
        Ok(JsonnetValue::boolean(matches!(args[0], JsonnetValue::Array(_))))
    }

    fn is_boolean(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "isBoolean")?;
        Ok(JsonnetValue::boolean(matches!(args[0], JsonnetValue::Boolean(_))))
    }

    fn is_function(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "isFunction")?;
        Ok(JsonnetValue::boolean(matches!(args[0], JsonnetValue::Function(_))))
    }

    fn is_number(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "isNumber")?;
        Ok(JsonnetValue::boolean(matches!(args[0], JsonnetValue::Number(_))))
    }

    fn is_object(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "isObject")?;
        Ok(JsonnetValue::boolean(matches!(args[0], JsonnetValue::Object(_))))
    }

    fn is_string(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "isString")?;
        Ok(JsonnetValue::boolean(matches!(args[0], JsonnetValue::String(_))))
    }

    // Array functions
    fn count(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 2, "count")?;
        let arr = args[0].as_array()?;
        let elem = &args[1];
        let count = arr.iter().filter(|item| item.equals(elem)).count() as f64;
        Ok(JsonnetValue::number(count))
    }

    fn find(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        // TODO: Implement find function
        Self::check_args(&args, 2, "find")?;
        Ok(JsonnetValue::array(vec![]))
    }

    fn member(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::contains(args)
    }

    // Math functions
    fn modulo(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 2, "modulo")?;
        let a = args[0].as_number()?;
        let b = args[1].as_number()?;
        if b == 0.0 {
            return Err(JsonnetError::DivisionByZero);
        }
        Ok(JsonnetValue::number(a % b))
    }

    fn pow(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 2, "pow")?;
        let a = args[0].as_number()?;
        let b = args[1].as_number()?;
        Ok(JsonnetValue::number(a.powf(b)))
    }

    fn exp(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "exp")?;
        let x = args[0].as_number()?;
        Ok(JsonnetValue::number(x.exp()))
    }

    fn log(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "log")?;
        let x = args[0].as_number()?;
        if x <= 0.0 {
            return Err(JsonnetError::runtime_error("log of non-positive number"));
        }
        Ok(JsonnetValue::number(x.ln()))
    }

    fn sqrt(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "sqrt")?;
        let x = args[0].as_number()?;
        if x < 0.0 {
            return Err(JsonnetError::runtime_error("sqrt of negative number"));
        }
        Ok(JsonnetValue::number(x.sqrt()))
    }

    fn sin(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "sin")?;
        let x = args[0].as_number()?;
        Ok(JsonnetValue::number(x.sin()))
    }

    fn cos(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "cos")?;
        let x = args[0].as_number()?;
        Ok(JsonnetValue::number(x.cos()))
    }

    fn tan(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "tan")?;
        let x = args[0].as_number()?;
        Ok(JsonnetValue::number(x.tan()))
    }

    fn asin(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "asin")?;
        let x = args[0].as_number()?;
        Ok(JsonnetValue::number(x.asin()))
    }

    fn acos(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "acos")?;
        let x = args[0].as_number()?;
        Ok(JsonnetValue::number(x.acos()))
    }

    fn atan(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "atan")?;
        let x = args[0].as_number()?;
        Ok(JsonnetValue::number(x.atan()))
    }

    fn floor(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "floor")?;
        let x = args[0].as_number()?;
        Ok(JsonnetValue::number(x.floor()))
    }

    fn ceil(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "ceil")?;
        let x = args[0].as_number()?;
        Ok(JsonnetValue::number(x.ceil()))
    }

    fn round(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "round")?;
        let x = args[0].as_number()?;
        Ok(JsonnetValue::number(x.round()))
    }

    fn abs(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "abs")?;
        let x = args[0].as_number()?;
        Ok(JsonnetValue::number(x.abs()))
    }

    fn max(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "max")?;
        let arr = args[0].as_array()?;
        if arr.is_empty() {
            return Err(JsonnetError::runtime_error("max() called on empty array"));
        }
        let mut max_val = f64::NEG_INFINITY;
        for item in arr {
            let val = item.as_number()?;
            if val > max_val {
                max_val = val;
            }
        }
        Ok(JsonnetValue::number(max_val))
    }

    fn min(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "min")?;
        let arr = args[0].as_array()?;
        if arr.is_empty() {
            return Err(JsonnetError::runtime_error("min() called on empty array"));
        }
        let mut min_val = f64::INFINITY;
        for item in arr {
            let val = item.as_number()?;
            if val < min_val {
                min_val = val;
            }
        }
        Ok(JsonnetValue::number(min_val))
    }

    fn clamp(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 3, "clamp")?;
        let x = args[0].as_number()?;
        let min = args[1].as_number()?;
        let max = args[2].as_number()?;
        let clamped = x.max(min).min(max);
        Ok(JsonnetValue::number(clamped))
    }

    fn assert_equal(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 2, "assertEqual")?;
        if !args[0].equals(&args[1]) {
            return Err(JsonnetError::assertion_failed(format!(
                "Assertion failed: {} != {}",
                args[0], args[1]
            )));
        }
        Ok(JsonnetValue::boolean(true))
    }

    // Array manipulation functions
    fn sort(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "sort")?;
        // TODO: Implement sorting
        Ok(args[0].clone())
    }

    fn uniq(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "uniq")?;
        // TODO: Implement uniqueness filtering
        Ok(args[0].clone())
    }

    fn reverse(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "reverse")?;
        let arr = args[0].as_array()?;
        let reversed: Vec<JsonnetValue> = arr.iter().rev().cloned().collect();
        Ok(JsonnetValue::array(reversed))
    }

    // Object functions
    fn merge_patch(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 2, "mergePatch")?;
        // TODO: Implement merge patch
        Ok(args[0].clone())
    }

    fn get(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 3, "get")?;
        let obj = args[0].as_object()?;
        let key = args[1].as_string()?;
        let default = &args[2];
        Ok(obj.get(key).unwrap_or(default).clone())
    }

    fn object_fields(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "objectFields")?;
        let obj = args[0].as_object()?;
        let fields: Vec<JsonnetValue> = obj.keys()
            .filter(|&k| !k.starts_with('_')) // Filter out hidden fields
            .map(|k| JsonnetValue::string(k.clone()))
            .collect();
        Ok(JsonnetValue::array(fields))
    }

    fn object_fields_all(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "objectFieldsAll")?;
        let obj = args[0].as_object()?;
        let fields: Vec<JsonnetValue> = obj.keys()
            .map(|k| JsonnetValue::string(k.clone()))
            .collect();
        Ok(JsonnetValue::array(fields))
    }

    fn object_has(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 2, "objectHas")?;
        let obj = args[0].as_object()?;
        let key = args[1].as_string()?;
        Ok(JsonnetValue::boolean(obj.contains_key(key) && !key.starts_with('_')))
    }

    fn object_has_all(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 2, "objectHasAll")?;
        let obj = args[0].as_object()?;
        let key = args[1].as_string()?;
        Ok(JsonnetValue::boolean(obj.contains_key(key)))
    }

    fn object_values(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "objectValues")?;
        let obj = args[0].as_object()?;
        let values: Vec<JsonnetValue> = obj.iter()
            .filter(|(k, _)| !k.starts_with('_'))
            .map(|(_, v)| v.clone())
            .collect();
        Ok(JsonnetValue::array(values))
    }

    fn object_values_all(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "objectValuesAll")?;
        let obj = args[0].as_object()?;
        let values: Vec<JsonnetValue> = obj.values().cloned().collect();
        Ok(JsonnetValue::array(values))
    }

    fn prune(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "prune")?;
        // TODO: Implement pruning (remove null values)
        Ok(args[0].clone())
    }

    fn map_with_key(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 2, "mapWithKey")?;
        // TODO: Implement mapWithKey
        Ok(args[1].clone())
    }

    /// Helper function to check argument count
    fn check_args(args: &[JsonnetValue], expected: usize, func_name: &str) -> Result<()> {
        if args.len() != expected {
            return Err(JsonnetError::invalid_function_call(format!(
                "{}() expects {} arguments, got {}",
                func_name, expected, args.len()
            )));
        }
        Ok(())
    }
}

impl JsonnetValue {
    /// Convert from serde_json::Value to JsonnetValue
    pub fn from_json_value(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => JsonnetValue::Null,
            serde_json::Value::Bool(b) => JsonnetValue::boolean(b),
            serde_json::Value::Number(n) => JsonnetValue::number(n.as_f64().unwrap_or(0.0)),
            serde_json::Value::String(s) => JsonnetValue::string(s),
            serde_json::Value::Array(arr) => {
                let jsonnet_arr: Vec<JsonnetValue> = arr.into_iter()
                    .map(JsonnetValue::from_json_value)
                    .collect();
                JsonnetValue::array(jsonnet_arr)
            }
            serde_json::Value::Object(obj) => {
                let mut jsonnet_obj = HashMap::new();
                for (k, v) in obj {
                    jsonnet_obj.insert(k, JsonnetValue::from_json_value(v));
                }
                JsonnetValue::object(jsonnet_obj)
            }
        }
    }
}
