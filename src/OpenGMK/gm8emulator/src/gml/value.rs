use crate::{game::external::dll, gml, math::Real};
use serde::{de::Deserializer, ser::Serializer, Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::{self, Display},
    sync::{OnceLock, RwLock},
};

// =========================================================================
// Global Interned String Pool for NaN-Boxing
// =========================================================================

struct StringPool {
    strings: Vec<&'static [u8]>,
    map: HashMap<&'static [u8], u32>,
}

fn get_pool() -> &'static RwLock<StringPool> {
    static POOL: OnceLock<RwLock<StringPool>> = OnceLock::new();
    POOL.get_or_init(|| {
        RwLock::new(StringPool {
            strings: Vec::new(),
            map: HashMap::new(),
        })
    })
}

fn intern_string(bytes: &[u8]) -> u32 {
    {
        let pool = get_pool().read().unwrap();
        if let Some(&id) = pool.map.get(bytes) {
            return id;
        }
    }
    let mut pool = get_pool().write().unwrap();
    if let Some(&id) = pool.map.get(bytes) {
        return id;
    }
    let leaked: &'static [u8] = Box::leak(bytes.to_vec().into_boxed_slice());
    let id = pool.strings.len() as u32;
    pool.strings.push(leaked);
    pool.map.insert(leaked, id);
    id
}

fn get_interned_string(id: usize) -> &'static [u8] {
    let pool = get_pool().read().unwrap();
    if id < pool.strings.len() {
        pool.strings[id]
    } else {
        b""
    }
}

// =========================================================================
// 64-bit NaN-Boxing Value
// =========================================================================

pub const STR_TAG: u64 = 0x7FF8_0001_0000_0000;
pub const STR_TAG_MASK: u64 = 0xFFFF_0000_0000_0000;
pub const PAYLOAD_MASK: u64 = 0x0000_FFFF_FFFF_FFFF;
pub const CMP_EPSILON: f64 = 0.00001;

#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct Value(pub u64);

#[derive(Clone, Debug, PartialEq)]
pub enum ValueView {
    Real(Real),
    Str(gml::String),
}

#[derive(Serialize, Deserialize)]
enum SerdeValue {
    Real(Real),
    Str(gml::String),
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.is_real() {
            SerdeValue::Real(Real::from(f64::from_bits(self.0))).serialize(serializer)
        } else {
            SerdeValue::Str(self.as_string()).serialize(serializer)
        }
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let sv = SerdeValue::deserialize(deserializer)?;
        Ok(match sv {
            SerdeValue::Real(r) => Value::from_real(r),
            SerdeValue::Str(s) => Value::from_string(s.as_ref()),
        })
    }
}

impl PartialEq for Value {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        if self.0 == other.0 {
            return true;
        }
        if self.is_real() && other.is_real() {
            f64::from_bits(self.0) == f64::from_bits(other.0)
        } else if self.is_string() && other.is_string() {
            self.as_bytes_slice() == other.as_bytes_slice()
        } else {
            false
        }
    }
}

impl Eq for Value {}

impl PartialOrd for Value {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.is_real() && other.is_real() {
            f64::from_bits(self.0).partial_cmp(&f64::from_bits(other.0))
        } else if self.is_string() && other.is_string() {
            self.as_bytes_slice().partial_cmp(other.as_bytes_slice())
        } else if self.is_real() {
            Some(std::cmp::Ordering::Less)
        } else {
            Some(std::cmp::Ordering::Greater)
        }
    }
}

impl Ord for Value {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_real() {
            write!(f, "{}", Real::from(self.as_real_raw()))
        } else {
            write!(f, "\"{}\"", self.as_string())
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_real() {
            write!(f, "Real({:?})", Real::from(self.as_real_raw()))
        } else {
            write!(f, "Str({:?})", self.as_string())
        }
    }
}

macro_rules! invalid_op {
    ($op: ident, $value: expr) => {
        Err(gml::Error::InvalidOperandsUnary(gml_parser::token::Operator::$op, $value))
    };
    ($op: ident, $left: expr, $right: expr) => {
        Err(gml::Error::InvalidOperandsBinary(gml_parser::token::Operator::$op, $left, $right))
    };
}

impl Value {
    #[inline(always)]
    pub fn is_real(&self) -> bool {
        (self.0 & STR_TAG_MASK) != STR_TAG
    }

    #[inline(always)]
    pub fn is_string(&self) -> bool {
        (self.0 & STR_TAG_MASK) == STR_TAG
    }

    #[inline(always)]
    pub fn from_real(val: impl Into<f64>) -> Self {
        let f: f64 = val.into();
        if f.is_nan() {
            Value(0x7FF8_0000_0000_0000)
        } else {
            Value(f.to_bits())
        }
    }

    #[inline(always)]
    pub fn from_string(bytes: impl AsRef<[u8]>) -> Self {
        let id = intern_string(bytes.as_ref());
        Value(STR_TAG | (id as u64 & PAYLOAD_MASK))
    }

    #[inline(always)]
    pub fn Real(r: Real) -> Self {
        Self::from_real(r)
    }

    #[inline(always)]
    pub fn Str(s: gml::String) -> Self {
        Self::from_string(s.as_ref())
    }

    #[inline(always)]
    pub fn view(&self) -> ValueView {
        if self.is_real() {
            ValueView::Real(Real::from(f64::from_bits(self.0)))
        } else {
            ValueView::Str(self.as_string())
        }
    }

    #[inline(always)]
    pub fn as_real_raw(&self) -> f64 {
        if self.is_real() {
            f64::from_bits(self.0)
        } else {
            0.0
        }
    }

    #[inline(always)]
    pub fn as_real(&self) -> Option<Real> {
        if self.is_real() {
            Some(Real::from(f64::from_bits(self.0)))
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn as_bytes_slice(&self) -> &'static [u8] {
        if self.is_string() {
            get_interned_string((self.0 & PAYLOAD_MASK) as usize)
        } else {
            b""
        }
    }

    #[inline(always)]
    pub fn as_string(&self) -> gml::String {
        gml::String::from(self.as_bytes_slice())
    }

    pub fn ty_str(&self) -> &'static str {
        if self.is_real() {
            "real"
        } else {
            "string"
        }
    }

    pub fn almost_equals(&self, other: &Self) -> bool {
        if self.is_real() && other.is_real() {
            let diff = f64::from_bits(self.0) - f64::from_bits(other.0);
            diff.abs() < CMP_EPSILON
        } else if self.is_string() && other.is_string() {
            self.0 == other.0 || self.as_bytes_slice() == other.as_bytes_slice()
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn round(&self) -> i32 {
        if self.is_real() {
            f64::from_bits(self.0).round() as i32
        } else {
            0
        }
    }

    pub fn is_truthy(&self) -> bool {
        self.is_real() && f64::from_bits(self.0) >= 0.5
    }

    #[inline]
    pub fn as_bool(&self) -> bool {
        self.is_truthy()
    }

    pub fn complement(self) -> gml::Result<Self> {
        if self.is_real() {
            Ok((!(self.round() as u64)).into())
        } else {
            invalid_op!(Complement, self)
        }
    }

    pub fn intdiv(self, rhs: Self) -> gml::Result<Self> {
        if self.is_real() && rhs.is_real() {
            let r = f64::from_bits(rhs.0);
            if r == 0.0 {
                return Ok(Self::from_real(0.0));
            }
            Ok(Self::from_real((f64::from_bits(self.0) / r).floor()))
        } else {
            invalid_op!(IntDivide, self, rhs)
        }
    }

    pub fn bool_and(self, rhs: Self) -> gml::Result<Self> {
        Ok((self.is_truthy() && rhs.is_truthy()).into())
    }

    pub fn bool_or(self, rhs: Self) -> gml::Result<Self> {
        Ok((self.is_truthy() || rhs.is_truthy()).into())
    }

    pub fn bool_xor(self, rhs: Self) -> gml::Result<Self> {
        Ok((self.is_truthy() != rhs.is_truthy()).into())
    }

    pub fn add(self, rhs: Self) -> gml::Result<Self> {
        if self.is_real() && rhs.is_real() {
            Ok(Self::from_real(f64::from_bits(self.0) + f64::from_bits(rhs.0)))
        } else if self.is_string() && rhs.is_string() {
            let l = self.as_bytes_slice();
            let r = rhs.as_bytes_slice();
            let mut buf = Vec::with_capacity(l.len() + r.len());
            buf.extend_from_slice(l);
            buf.extend_from_slice(r);
            Ok(Self::from_string(&buf))
        } else {
            invalid_op!(Add, self, rhs)
        }
    }

    pub fn add_assign(&mut self, rhs: Self) -> gml::Result<()> {
        *self = self.add(rhs)?;
        Ok(())
    }

    pub fn bitand(self, rhs: Self) -> gml::Result<Self> {
        if self.is_real() && rhs.is_real() {
            Ok(((self.round() as u64) & (rhs.round() as u64)).into())
        } else {
            invalid_op!(BitwiseAnd, self, rhs)
        }
    }

    pub fn bitand_assign(&mut self, rhs: Self) -> gml::Result<()> {
        *self = self.bitand(rhs)?;
        Ok(())
    }

    pub fn bitor(self, rhs: Self) -> gml::Result<Self> {
        if self.is_real() && rhs.is_real() {
            Ok(((self.round() as u64) | (rhs.round() as u64)).into())
        } else {
            invalid_op!(BitwiseOr, self, rhs)
        }
    }

    pub fn bitor_assign(&mut self, rhs: Self) -> gml::Result<()> {
        *self = self.bitor(rhs)?;
        Ok(())
    }

    pub fn bitxor(self, rhs: Self) -> gml::Result<Self> {
        if self.is_real() && rhs.is_real() {
            Ok(((self.round() as u64) ^ (rhs.round() as u64)).into())
        } else {
            invalid_op!(BitwiseXor, self, rhs)
        }
    }

    pub fn bitxor_assign(&mut self, rhs: Self) -> gml::Result<()> {
        *self = self.bitxor(rhs)?;
        Ok(())
    }

    pub fn div(self, rhs: Self) -> gml::Result<Self> {
        if self.is_real() && rhs.is_real() {
            let r = f64::from_bits(rhs.0);
            if r == 0.0 {
                return Ok(Self::from_real(0.0));
            }
            Ok(Self::from_real(f64::from_bits(self.0) / r))
        } else {
            invalid_op!(Divide, self, rhs)
        }
    }

    pub fn div_assign(&mut self, rhs: Self) -> gml::Result<()> {
        *self = self.div(rhs)?;
        Ok(())
    }

    pub fn modulo(self, rhs: Self) -> gml::Result<Self> {
        if self.is_real() && rhs.is_real() {
            let l = f64::from_bits(self.0);
            let r = f64::from_bits(rhs.0);
            if r == 0.0 {
                return Ok(Self::from_real(0.0));
            }
            Ok(Self::from_real(l - (l / r).trunc() * r))
        } else {
            invalid_op!(Modulo, self, rhs)
        }
    }

    pub fn mul(self, rhs: Self) -> gml::Result<Self> {
        if self.is_real() && rhs.is_real() {
            Ok(Self::from_real(f64::from_bits(self.0) * f64::from_bits(rhs.0)))
        } else if self.is_real() && rhs.is_string() {
            let repeat = self.round();
            if repeat > 0 {
                let slice = rhs.as_bytes_slice();
                Ok(Self::from_string(&slice.repeat(repeat as usize)))
            } else {
                Ok(Self::from_string(b""))
            }
        } else {
            invalid_op!(Multiply, self, rhs)
        }
    }

    pub fn mul_assign(&mut self, rhs: Self) -> gml::Result<()> {
        *self = self.mul(rhs)?;
        Ok(())
    }

    pub fn neg(self) -> gml::Result<Self> {
        if self.is_real() {
            Ok(Self::from_real(-f64::from_bits(self.0)))
        } else {
            invalid_op!(Subtract, self)
        }
    }

    pub fn not(self) -> gml::Result<Self> {
        if self.is_real() {
            Ok((!self.is_truthy()).into())
        } else {
            invalid_op!(Not, self)
        }
    }

    pub fn shl(self, rhs: Self) -> gml::Result<Self> {
        if self.is_real() && rhs.is_real() {
            let shift = rhs.round() as i8;
            Ok(((self.round() as u64).wrapping_shl(shift as u32)).into())
        } else {
            invalid_op!(BinaryShiftLeft, self, rhs)
        }
    }

    pub fn shr(self, rhs: Self) -> gml::Result<Self> {
        if self.is_real() && rhs.is_real() {
            let shift = rhs.round() as i8;
            Ok(((self.round() as u64).wrapping_shr(shift as u32)).into())
        } else {
            invalid_op!(BinaryShiftRight, self, rhs)
        }
    }

    pub fn sub(self, rhs: Self) -> gml::Result<Self> {
        if self.is_real() && rhs.is_real() {
            Ok(Self::from_real(f64::from_bits(self.0) - f64::from_bits(rhs.0)))
        } else {
            invalid_op!(Subtract, self, rhs)
        }
    }

    pub fn sub_assign(&mut self, rhs: Self) -> gml::Result<()> {
        *self = self.sub(rhs)?;
        Ok(())
    }

    // Comparison operators (GML returns Value, not bool)
    pub fn gml_eq(self, rhs: Self) -> gml::Result<Self> {
        if self.is_real() && rhs.is_real() {
            let diff = f64::from_bits(self.0) - f64::from_bits(rhs.0);
            Ok(if diff.abs() < CMP_EPSILON { super::TRUE } else { super::FALSE }.into())
        } else if self.is_string() && rhs.is_string() {
            let eq = (self.0 == rhs.0) || (self.as_bytes_slice() == rhs.as_bytes_slice());
            Ok(if eq { super::TRUE } else { super::FALSE }.into())
        } else {
            invalid_op!(Equal, self, rhs)
        }
    }

    pub fn gml_ne(self, rhs: Self) -> gml::Result<Self> {
        if self.is_real() && rhs.is_real() {
            let diff = f64::from_bits(self.0) - f64::from_bits(rhs.0);
            Ok(if diff.abs() >= CMP_EPSILON { super::TRUE } else { super::FALSE }.into())
        } else if self.is_string() && rhs.is_string() {
            let ne = (self.0 != rhs.0) && (self.as_bytes_slice() != rhs.as_bytes_slice());
            Ok(if ne { super::TRUE } else { super::FALSE }.into())
        } else {
            invalid_op!(NotEqual, self, rhs)
        }
    }

    pub fn gml_lt(self, rhs: Self) -> gml::Result<Self> {
        if self.is_real() && rhs.is_real() {
            let diff = f64::from_bits(self.0) - f64::from_bits(rhs.0);
            Ok(if diff <= -CMP_EPSILON { super::TRUE } else { super::FALSE }.into())
        } else if self.is_string() && rhs.is_string() {
            Ok(if self.as_bytes_slice() < rhs.as_bytes_slice() { super::TRUE } else { super::FALSE }.into())
        } else {
            invalid_op!(LessThan, self, rhs)
        }
    }

    pub fn gml_lte(self, rhs: Self) -> gml::Result<Self> {
        if self.is_real() && rhs.is_real() {
            let diff = f64::from_bits(self.0) - f64::from_bits(rhs.0);
            Ok(if diff < CMP_EPSILON { super::TRUE } else { super::FALSE }.into())
        } else if self.is_string() && rhs.is_string() {
            Ok(if self.as_bytes_slice() <= rhs.as_bytes_slice() { super::TRUE } else { super::FALSE }.into())
        } else {
            invalid_op!(LessThanOrEqual, self, rhs)
        }
    }

    pub fn gml_gt(self, rhs: Self) -> gml::Result<Self> {
        if self.is_real() && rhs.is_real() {
            let diff = f64::from_bits(self.0) - f64::from_bits(rhs.0);
            Ok(if diff >= CMP_EPSILON { super::TRUE } else { super::FALSE }.into())
        } else if self.is_string() && rhs.is_string() {
            Ok(if self.as_bytes_slice() > rhs.as_bytes_slice() { super::TRUE } else { super::FALSE }.into())
        } else {
            invalid_op!(GreaterThan, self, rhs)
        }
    }

    pub fn gml_gte(self, rhs: Self) -> gml::Result<Self> {
        if self.is_real() && rhs.is_real() {
            let diff = f64::from_bits(self.0) - f64::from_bits(rhs.0);
            Ok(if diff > -CMP_EPSILON { super::TRUE } else { super::FALSE }.into())
        } else if self.is_string() && rhs.is_string() {
            Ok(if self.as_bytes_slice() >= rhs.as_bytes_slice() { super::TRUE } else { super::FALSE }.into())
        } else {
            invalid_op!(GreaterThanOrEqual, self, rhs)
        }
    }

    pub fn max(self, other: Self) -> Self {
        if self.is_real() && other.is_real() {
            if f64::from_bits(self.0) > f64::from_bits(other.0) { self } else { other }
        } else if self.is_real() && other.is_string() {
            if f64::from_bits(self.0) < 0.0 { other } else { self }
        } else if self.is_string() && other.is_real() {
            self
        } else if self.is_string() && other.is_string() {
            if self.as_bytes_slice() > other.as_bytes_slice() { self } else { other }
        } else {
            self
        }
    }

    pub fn min(self, other: Self) -> Self {
        if self.is_real() && other.is_real() {
            if f64::from_bits(self.0) > f64::from_bits(other.0) { other } else { self }
        } else if self.is_real() && other.is_string() {
            if f64::from_bits(self.0) > 0.0 { other } else { self }
        } else if self.is_string() && other.is_real() {
            other
        } else if self.is_string() && other.is_string() {
            if self.as_bytes_slice() > other.as_bytes_slice() { other } else { self }
        } else {
            other
        }
    }

    pub fn repr(&self) -> gml::String {
        if self.is_real() {
            let r = Real::from(self.as_real_raw());
            if r.fract().into_inner() == 0.0 {
                format!("{:.0}", r).into()
            } else {
                format!("{:.2}", r).into()
            }
        } else {
            self.as_string()
        }
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(16);
        if self.is_real() {
            bytes.resize(4, 0);
            bytes.extend_from_slice(&self.as_real_raw().to_le_bytes());
            bytes.resize(16, 0);
        } else {
            let slice = self.as_bytes_slice();
            bytes.push(1);
            bytes.resize(12, 0);
            bytes.extend_from_slice(&(slice.len() as u32).to_le_bytes());
            bytes.extend_from_slice(slice);
        }
        bytes
    }

    pub fn from_reader(reader: &mut dyn std::io::Read) -> Option<Self> {
        let mut block = [0u8; 16];
        reader.read_exact(&mut block).ok()?;
        if block.len() == 16 {
            match u32::from_le_bytes(block[0..4].try_into().unwrap()) {
                0 => Some(Self::from_real(f64::from_le_bytes(block[4..12].try_into().unwrap()))),
                1 => {
                    let len = u32::from_le_bytes(block[12..16].try_into().unwrap());
                    let mut buf = vec![0; len as usize];
                    reader.read_exact(&mut buf).ok()?;
                    Some(Self::from_string(&buf))
                },
                _ => None,
            }
        } else {
            None
        }
    }
}

// Conversions
impl From<f64> for Value {
    #[inline(always)]
    fn from(value: f64) -> Self {
        Self::from_real(value)
    }
}

impl From<Real> for Value {
    #[inline(always)]
    fn from(value: Real) -> Self {
        Self::from_real(value)
    }
}

impl From<i32> for Value {
    #[inline(always)]
    fn from(value: i32) -> Self {
        Self::from_real(value as f64)
    }
}

impl From<u32> for Value {
    #[inline(always)]
    fn from(value: u32) -> Self {
        Self::from_real(value as f64)
    }
}

impl From<u64> for Value {
    #[inline(always)]
    fn from(value: u64) -> Self {
        Self::from_real(value as f64)
    }
}

impl From<usize> for Value {
    #[inline(always)]
    fn from(value: usize) -> Self {
        Self::from_real(value as f64)
    }
}

impl From<u8> for Value {
    #[inline(always)]
    fn from(value: u8) -> Self {
        Self::from_real(value as f64)
    }
}

impl From<bool> for Value {
    #[inline(always)]
    fn from(value: bool) -> Self {
        Self::from_real(if value { gml::TRUE } else { gml::FALSE } as f64)
    }
}

impl From<gml::String> for Value {
    #[inline(always)]
    fn from(value: gml::String) -> Self {
        Self::from_string(value.as_ref())
    }
}

impl From<String> for Value {
    #[inline(always)]
    fn from(value: String) -> Self {
        Self::from_string(value.as_bytes())
    }
}

impl From<&str> for Value {
    #[inline(always)]
    fn from(value: &str) -> Self {
        Self::from_string(value.as_bytes())
    }
}

impl From<Vec<u8>> for Value {
    #[inline(always)]
    fn from(value: Vec<u8>) -> Self {
        Self::from_string(&value)
    }
}

impl From<&[u8]> for Value {
    #[inline(always)]
    fn from(value: &[u8]) -> Self {
        Self::from_string(value)
    }
}

impl From<Value> for i32 {
    #[inline(always)]
    fn from(value: Value) -> Self {
        value.round()
    }
}

impl From<Value> for u32 {
    #[inline(always)]
    fn from(value: Value) -> Self {
        if value.is_real() {
            f64::from_bits(value.0).round() as u32
        } else {
            0
        }
    }
}

impl From<Value> for f64 {
    #[inline(always)]
    fn from(value: Value) -> Self {
        value.as_real_raw()
    }
}

impl From<Value> for Real {
    #[inline(always)]
    fn from(value: Value) -> Self {
        Real::from(value.as_real_raw())
    }
}

impl From<Value> for gml::String {
    #[inline(always)]
    fn from(value: Value) -> Self {
        value.as_string()
    }
}

impl<'a> From<&'a Value> for &'a [u8] {
    #[inline(always)]
    fn from(value: &'a Value) -> Self {
        value.as_bytes_slice()
    }
}

impl Default for Value {
    #[inline(always)]
    fn default() -> Self {
        Self::from_real(0.0)
    }
}

impl From<Value> for dll::Value {
    fn from(v: Value) -> Self {
        if v.is_string() {
            dll::Value::Str(dll::PascalString::new(v.as_bytes_slice()))
        } else {
            dll::Value::Real(v.as_real_raw())
        }
    }
}

impl From<dll::Value> for Value {
    fn from(v: dll::Value) -> Self {
        match v {
            dll::Value::Real(r) => Value::from_real(r),
            dll::Value::Str(s) => Value::from_string(s.as_slice()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn op_add() {
        let a = Value::Real(Real::from(0.1));
        let b = Value::Real(Real::from(0.2));
        assert!((a.add(b).unwrap()).almost_equals(&Value::Real(Real::from(0.3))));

        let c = Value::Str("Hello, ".to_string().into());
        let d = Value::Str("world!".to_string().into());
        assert!((c.add(d).unwrap()).almost_equals(&Value::Str("Hello, world!".to_string().into())));
    }

    #[test]
    #[should_panic]
    fn op_add_invalid() {
        let a = Value::Real(Real::from(0.1));
        let b = Value::Str("owo".to_string().into());
        let _ = a.add(b).unwrap();
    }
}
