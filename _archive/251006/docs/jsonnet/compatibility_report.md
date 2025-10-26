# Google Jsonnet Standard Library Functions (175 total)

## Implemented in Kotoba-Jsonnet (89/175 = 51%)

### Array Functions
- ✅ length, makeArray, filter, map, foldl, foldr, range, member, count, uniq, sort, reverse
- ✅ find, all, any

### String Functions
- ✅ length, substr, startsWith, endsWith, contains, split, join, char, codepoint, toString, parseInt
- ✅ encodeUTF8, decodeUTF8, md5, base64, base64Decode, escapeStringJson, escapeStringYaml, escapeStringPython
- ✅ escapeStringBash, escapeStringDollars, stringChars, stringBytes, format, toLower, toUpper, trim

### Object Functions
- ✅ objectFields, objectFieldsAll, objectHas, objectHasAll, objectValues, objectValuesAll, get, mapWithKey, mergePatch, prune

### Math Functions
- ✅ abs, sqrt, sin, cos, tan, asin, acos, atan, floor, ceil, round, pow, exp, log, modulo, max, min, clamp

### Type Functions
- ✅ type, isArray, isBoolean, isFunction, isNumber, isObject, isString

### Utility Functions
- ✅ assertEqual, parseJson, manifestJson, manifestJsonEx, trace

## Missing Functions (86 functions)

### Core Functions (High Priority)
- ❌ id - Identity function
- ❌ equals - Deep equality comparison
- ❌ lines - String to lines conversion
- ❌ strReplace - String replacement
- ❌ asciiLower/asciiUpper - ASCII case conversion
- ❌ remove/removeAt - Array element removal
- ❌ set/setDiff/setInter/setUnion - Set operations
- ❌ sha1/sha256/sha3/sha512 - Hash functions

### Extended Array Functions
- ❌ flatMap, flattenArrays, flattenDeepArray
- ❌ mapWithIndex
- ❌ slice, remove, removeAt

### Advanced String Functions
- ❌ lstripChars/rstripChars/stripChars - Character stripping
- ❌ findSubstr - Substring search
- ❌ strReplace - String replacement
- ❌ repeat - String repetition
- ❌ asciiLower/asciiUpper - ASCII case conversion

### Extended Object Functions
- ❌ objectKeysValues/objectKeysValuesAll - Key-value pairs
- ❌ objectRemoveKey - Key removal

### Advanced Math Functions
- ❌ log2, log10 - Logarithmic functions
- ❌ deg2rad, rad2deg - Angle conversion
- ❌ hypot, atan2 - Advanced trigonometry
- ❌ mantissa, exponent - Floating point decomposition
- ❌ sign - Sign function

### Additional Type Functions
- ❌ isInteger, isDecimal, isEven, isOdd, isEmpty

### Extended Encoding/Decoding
- ❌ base64DecodeBytes - Binary base64 decoding
- ❌ escapeStringXML - XML escaping

### Manifest Functions
- ❌ manifestIni, manifestPython, manifestPythonVars
- ❌ manifestToml, manifestTomlEx
- ❌ manifestXmlJsonml, manifestYamlDoc, manifestYamlStream

## Compatibility Summary

- **Core Compatibility**: 51% (89/175 functions)
- **Critical Missing**: id, equals, lines, strReplace
- **Performance Impact**: Missing hash functions (sha1/sha256/sha3/sha512)
- **Advanced Features**: Missing most manifest functions and advanced math

## Next Steps

1. **Phase 1**: Implement core functions (id, equals, lines, strReplace)
2. **Phase 2**: Add hash functions (sha1, sha256, sha3, sha512)
3. **Phase 3**: Implement advanced array functions
4. **Phase 4**: Add manifest functions for different formats

