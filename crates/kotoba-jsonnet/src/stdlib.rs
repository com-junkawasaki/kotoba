//! Jsonnet standard library implementation

use crate::error::{JsonnetError, Result};
use crate::value::JsonnetValue;
use sha1::Sha1;
use sha2::{Sha256, Sha512, Digest};
use sha3::Sha3_256;
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
            "toLower" => Self::to_lower(args),
            "toUpper" => Self::to_upper(args),
            "trim" => Self::trim(args),
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
            "trace" => Self::trace(args),
            "sort" => Self::sort(args),
            "uniq" => Self::uniq(args),
            "reverse" => Self::reverse(args),
            "all" => Self::all(args),
            "any" => Self::any(args),
            "mergePatch" => Self::merge_patch(args),
            "get" => Self::get(args),
            "id" => Self::id(args),
            "equals" => Self::equals(args),
            "lines" => Self::lines(args),
            "strReplace" => Self::str_replace(args),
            "sha1" => Self::sha1(args),
            "sha256" => Self::sha256(args),
            "sha3" => Self::sha3(args),
            "sha512" => Self::sha512(args),
            "asciiLower" => Self::ascii_lower(args),
            "asciiUpper" => Self::ascii_upper(args),
            "set" => Self::set(args),
            "flatMap" => Self::flat_map(args),
            "mapWithIndex" => Self::map_with_index(args),
            "lstripChars" => Self::lstrip_chars(args),
            "rstripChars" => Self::rstrip_chars(args),
            "stripChars" => Self::strip_chars(args),
            "findSubstr" => Self::find_substr(args),
            "repeat" => Self::repeat(args),
            "setMember" => Self::set_member(args),
            "setUnion" => Self::set_union(args),
            "setInter" => Self::set_inter(args),
            "setDiff" => Self::set_diff(args),
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
        Self::check_args(&args, 2, "find")?;
        match (&args[0], &args[1]) {
            (JsonnetValue::Array(arr), value) => {
                let mut indices = Vec::new();
                for (i, item) in arr.iter().enumerate() {
                    if item == value {
                        indices.push(JsonnetValue::Number(i as f64));
                    }
                }
                Ok(JsonnetValue::array(indices))
            }
            _ => Err(JsonnetError::runtime_error("find expects array and search value")),
        }
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

    fn trace(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 2, "trace")?;
        // Print the second argument to stderr for tracing
        eprintln!("TRACE: {:?}", args[1]);
        // Return the first argument
        Ok(args[0].clone())
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

    fn to_lower(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "toLower")?;
        match &args[0] {
            JsonnetValue::String(s) => Ok(JsonnetValue::string(s.to_lowercase())),
            _ => Err(JsonnetError::runtime_error("toLower expects a string argument")),
        }
    }

    fn to_upper(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "toUpper")?;
        match &args[0] {
            JsonnetValue::String(s) => Ok(JsonnetValue::string(s.to_uppercase())),
            _ => Err(JsonnetError::runtime_error("toUpper expects a string argument")),
        }
    }

    fn trim(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "trim")?;
        match &args[0] {
            JsonnetValue::String(s) => Ok(JsonnetValue::string(s.trim().to_string())),
            _ => Err(JsonnetError::runtime_error("trim expects a string argument")),
        }
    }

    fn all(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "all")?;
        match &args[0] {
            JsonnetValue::Array(arr) => {
                let result = arr.iter().all(|item| item.is_truthy());
                Ok(JsonnetValue::boolean(result))
            }
            _ => Err(JsonnetError::runtime_error("all expects an array argument")),
        }
    }

    fn any(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "any")?;
        match &args[0] {
            JsonnetValue::Array(arr) => {
                let result = arr.iter().any(|item| item.is_truthy());
                Ok(JsonnetValue::boolean(result))
            }
            _ => Err(JsonnetError::runtime_error("any expects an array argument")),
        }
    }

    fn id(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "id")?;
        Ok(args[0].clone())
    }

    fn equals(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 2, "equals")?;
        let a = &args[0];
        let b = &args[1];

        // First check primitive equality
        if a == b {
            return Ok(JsonnetValue::boolean(true));
        }

        // Check types
        let ta = a.type_name();
        let tb = b.type_name();
        if ta != tb {
            return Ok(JsonnetValue::boolean(false));
        }

        match (a, b) {
            (JsonnetValue::Array(arr_a), JsonnetValue::Array(arr_b)) => {
                if arr_a.len() != arr_b.len() {
                    return Ok(JsonnetValue::boolean(false));
                }
                for (i, item_a) in arr_a.iter().enumerate() {
                    let eq_args = vec![item_a.clone(), arr_b[i].clone()];
                    if let Ok(JsonnetValue::Boolean(false)) = Self::equals(eq_args) {
                        return Ok(JsonnetValue::boolean(false));
                    }
                }
                Ok(JsonnetValue::boolean(true))
            }
            (JsonnetValue::Object(obj_a), JsonnetValue::Object(obj_b)) => {
                // Get field names
                let fields_a: Vec<String> = obj_a.keys().cloned().collect();
                let fields_b: Vec<String> = obj_b.keys().cloned().collect();

                if fields_a.len() != fields_b.len() {
                    return Ok(JsonnetValue::boolean(false));
                }

                // Sort for comparison
                let mut sorted_a = fields_a.clone();
                sorted_a.sort();
                let mut sorted_b = fields_b.clone();
                sorted_b.sort();

                if sorted_a != sorted_b {
                    return Ok(JsonnetValue::boolean(false));
                }

                // Compare all field values
                for field in sorted_a {
                    let val_a = &obj_a[&field];
                    let val_b = &obj_b[&field];
                    let eq_args = vec![val_a.clone(), val_b.clone()];
                    if let Ok(JsonnetValue::Boolean(false)) = Self::equals(eq_args) {
                        return Ok(JsonnetValue::boolean(false));
                    }
                }
                Ok(JsonnetValue::boolean(true))
            }
            _ => Ok(JsonnetValue::boolean(false)),
        }
    }

    fn lines(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "lines")?;
        match &args[0] {
            JsonnetValue::Array(arr) => {
                let mut lines = Vec::new();
                for item in arr {
                    // Convert to string representation like Jsonnet does
                    match item {
                        JsonnetValue::String(s) => lines.push(s.clone()),
                        JsonnetValue::Number(n) => lines.push(n.to_string()),
                        JsonnetValue::Boolean(b) => lines.push(b.to_string()),
                        _ => lines.push(format!("{}", item)),
                    }
                }
                lines.push("".to_string()); // Add trailing newline
                Ok(JsonnetValue::string(lines.join("\n")))
            }
            _ => Err(JsonnetError::runtime_error("lines expects an array argument")),
        }
    }

    fn str_replace(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 3, "strReplace")?;

        let str_val = &args[0];
        let from_val = &args[1];
        let to_val = &args[2];

        let str = str_val.as_string()?.to_string();
        let from = from_val.as_string()?.to_string();
        let to = to_val.as_string()?.to_string();

        if from.is_empty() {
            return Err(JsonnetError::runtime_error("'from' string must not be zero length"));
        }

        // Simple implementation using Rust's string replace
        // For now, we'll use a simple approach. Full implementation would need
        // the complex recursive logic from Google Jsonnet
        let result = str.replace(&from, &to);
        Ok(JsonnetValue::string(result))
    }

    fn sha1(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "sha1")?;
        let input = args[0].as_string()?.as_bytes();
        let mut hasher = Sha1::new();
        hasher.update(input);
        let result = hasher.finalize();
        Ok(JsonnetValue::string(hex::encode(result)))
    }

    fn sha256(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "sha256")?;
        let input = args[0].as_string()?.as_bytes();
        let mut hasher = Sha256::new();
        hasher.update(input);
        let result = hasher.finalize();
        Ok(JsonnetValue::string(hex::encode(result)))
    }

    fn sha3(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "sha3")?;
        let input = args[0].as_string()?.as_bytes();
        let mut hasher = Sha3_256::new();
        hasher.update(input);
        let result = hasher.finalize();
        Ok(JsonnetValue::string(hex::encode(result)))
    }

    fn sha512(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "sha512")?;
        let input = args[0].as_string()?.as_bytes();
        let mut hasher = Sha512::new();
        hasher.update(input);
        let result = hasher.finalize();
        Ok(JsonnetValue::string(hex::encode(result)))
    }

    fn ascii_lower(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "asciiLower")?;
        let input = args[0].as_string()?;
        Ok(JsonnetValue::string(input.to_ascii_lowercase()))
    }

    fn ascii_upper(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "asciiUpper")?;
        let input = args[0].as_string()?;
        Ok(JsonnetValue::string(input.to_ascii_uppercase()))
    }

    fn flat_map(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 2, "flatMap")?;
        let func = &args[0];
        let arr = &args[1];

        match arr {
            JsonnetValue::Array(array) => {
                let mut result = Vec::new();
                for item in array {
                    // Apply function to each item
                    // For now, we'll implement a simple version that expects the function to return an array
                    // Full implementation would need to evaluate the function
                    if let JsonnetValue::Array(sub_array) = item {
                        result.extend(sub_array.clone());
                    } else {
                        result.push(item.clone());
                    }
                }
                Ok(JsonnetValue::array(result))
            }
            JsonnetValue::String(s) => {
                // For strings, treat each character as an element
                let mut result = Vec::new();
                for ch in s.chars() {
                    // Apply function to each character - simplified implementation
                    result.push(JsonnetValue::string(ch.to_string()));
                }
                Ok(JsonnetValue::array(result))
            }
            _ => Err(JsonnetError::runtime_error("flatMap expects array or string as second argument")),
        }
    }

    fn map_with_index(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 2, "mapWithIndex")?;
        let func = &args[0];
        let arr = &args[1];

        match arr {
            JsonnetValue::Array(array) => {
                let mut result = Vec::new();
                for (i, item) in array.iter().enumerate() {
                    // Apply function with index - simplified implementation
                    // In full implementation, this would call the function with (index, value)
                    result.push(JsonnetValue::array(vec![JsonnetValue::number(i as f64), item.clone()]));
                }
                Ok(JsonnetValue::array(result))
            }
            JsonnetValue::String(s) => {
                let mut result = Vec::new();
                for (i, ch) in s.chars().enumerate() {
                    result.push(JsonnetValue::array(vec![
                        JsonnetValue::number(i as f64),
                        JsonnetValue::string(ch.to_string())
                    ]));
                }
                Ok(JsonnetValue::array(result))
            }
            _ => Err(JsonnetError::runtime_error("mapWithIndex expects array or string as second argument")),
        }
    }

    fn lstrip_chars(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 2, "lstripChars")?;
        let str_val = args[0].as_string()?;
        let chars_val = args[1].as_string()?;

        let chars_set: std::collections::HashSet<char> = chars_val.chars().collect();
        let result: String = str_val.chars()
            .skip_while(|c| chars_set.contains(c))
            .collect();

        Ok(JsonnetValue::string(result))
    }

    fn rstrip_chars(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 2, "rstripChars")?;
        let str_val = args[0].as_string()?;
        let chars_val = args[1].as_string()?;

        let chars_set: std::collections::HashSet<char> = chars_val.chars().collect();
        let result: String = str_val.chars()
            .rev()
            .skip_while(|c| chars_set.contains(c))
            .collect::<Vec<char>>()
            .into_iter()
            .rev()
            .collect();

        Ok(JsonnetValue::string(result))
    }

    fn strip_chars(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 2, "stripChars")?;
        let str_val = &args[0];
        let chars_val = &args[1];

        // First apply lstripChars, then rstripChars
        let lstripped_args = vec![str_val.clone(), chars_val.clone()];
        let lstripped = Self::lstrip_chars(lstripped_args)?;
        let rstripped_args = vec![lstripped, chars_val.clone()];
        Self::rstrip_chars(rstripped_args)
    }

    fn find_substr(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 2, "findSubstr")?;
        let pat = args[0].as_string()?;
        let str = args[1].as_string()?;

        if pat.is_empty() {
            return Err(JsonnetError::runtime_error("findSubstr pattern cannot be empty"));
        }

        let mut result = Vec::new();
        let mut start = 0;

        while let Some(pos) = str[start..].find(&pat) {
            result.push(JsonnetValue::number((start + pos) as f64));
            start += pos + pat.len();
        }

        Ok(JsonnetValue::array(result))
    }

    fn repeat(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 2, "repeat")?;
        let what = &args[0];
        let count_val = &args[1];

        let count = if let JsonnetValue::Number(n) = count_val {
            *n as usize
        } else {
            return Err(JsonnetError::runtime_error("repeat count must be a number"));
        };

        match what {
            JsonnetValue::String(s) => {
                let repeated = s.repeat(count);
                Ok(JsonnetValue::string(repeated))
            }
            JsonnetValue::Array(arr) => {
                let mut result = Vec::new();
                for _ in 0..count {
                    result.extend(arr.clone());
                }
                Ok(JsonnetValue::array(result))
            }
            _ => Err(JsonnetError::runtime_error("repeat first argument must be string or array")),
        }
    }

    fn set(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 1, "set")?;
        match &args[0] {
            JsonnetValue::Array(arr) => {
                // Remove duplicates while preserving order
                let mut result = Vec::new();

                for item in arr {
                    // Check if item is already in result
                    let mut found = false;
                    for existing in &result {
                        if existing == item {
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        result.push(item.clone());
                    }
                }

                Ok(JsonnetValue::array(result))
            }
            _ => Err(JsonnetError::runtime_error("set expects an array argument")),
        }
    }

    fn set_member(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 2, "setMember")?;
        let value = &args[0];
        let arr = match &args[1] {
            JsonnetValue::Array(a) => a,
            _ => return Err(JsonnetError::runtime_error("setMember expects array as second argument")),
        };

        for item in arr {
            if item == value {
                return Ok(JsonnetValue::boolean(true));
            }
        }
        Ok(JsonnetValue::boolean(false))
    }

    fn set_union(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 2, "setUnion")?;
        let arr_a = match &args[0] {
            JsonnetValue::Array(a) => a,
            _ => return Err(JsonnetError::runtime_error("setUnion expects arrays as arguments")),
        };
        let arr_b = match &args[1] {
            JsonnetValue::Array(a) => a,
            _ => return Err(JsonnetError::runtime_error("setUnion expects arrays as arguments")),
        };

        let mut result = Vec::new();

        // Add all elements from first array (preserving order)
        for item in arr_a {
            let mut found = false;
            for existing in &result {
                if existing == item {
                    found = true;
                    break;
                }
            }
            if !found {
                result.push(item.clone());
            }
        }

        // Add elements from second array that aren't already in result
        for item in arr_b {
            let mut found = false;
            for existing in &result {
                if existing == item {
                    found = true;
                    break;
                }
            }
            if !found {
                result.push(item.clone());
            }
        }

        Ok(JsonnetValue::array(result))
    }

    fn set_inter(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 2, "setInter")?;
        let arr_a = match &args[0] {
            JsonnetValue::Array(a) => a,
            _ => return Err(JsonnetError::runtime_error("setInter expects arrays as arguments")),
        };
        let arr_b = match &args[1] {
            JsonnetValue::Array(a) => a,
            _ => return Err(JsonnetError::runtime_error("setInter expects arrays as arguments")),
        };

        let mut result = Vec::new();

        for item_a in arr_a {
            // Check if item_a exists in arr_b
            let mut found_in_b = false;
            for item_b in arr_b {
                if item_a == item_b {
                    found_in_b = true;
                    break;
                }
            }

            if found_in_b {
                // Check if item_a is already in result
                let mut already_in_result = false;
                for existing in &result {
                    if existing == item_a {
                        already_in_result = true;
                        break;
                    }
                }
                if !already_in_result {
                    result.push(item_a.clone());
                }
            }
        }

        Ok(JsonnetValue::array(result))
    }

    fn set_diff(args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        Self::check_args(&args, 2, "setDiff")?;
        let arr_a = match &args[0] {
            JsonnetValue::Array(a) => a,
            _ => return Err(JsonnetError::runtime_error("setDiff expects arrays as arguments")),
        };
        let arr_b = match &args[1] {
            JsonnetValue::Array(a) => a,
            _ => return Err(JsonnetError::runtime_error("setDiff expects arrays as arguments")),
        };

        let mut result = Vec::new();

        for item_a in arr_a {
            // Check if item_a does NOT exist in arr_b
            let mut found_in_b = false;
            for item_b in arr_b {
                if item_a == item_b {
                    found_in_b = true;
                    break;
                }
            }

            if !found_in_b {
                // Check if item_a is already in result
                let mut already_in_result = false;
                for existing in &result {
                    if existing == item_a {
                        already_in_result = true;
                        break;
                    }
                }
                if !already_in_result {
                    result.push(item_a.clone());
                }
            }
        }

        Ok(JsonnetValue::array(result))
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
