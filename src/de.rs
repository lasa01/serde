// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::collections::{HashMap, HashSet, TreeMap, TreeSet};
use std::gc::{GC, Gc};
use std::hash::Hash;
use std::num;
use std::rc::Rc;
use std::sync::Arc;

#[deriving(Clone, PartialEq, Show)]
pub enum Token {
    Null,
    Bool(bool),
    Int(int),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    Uint(uint),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    F32(f32),
    F64(f64),
    Char(char),
    Str(&'static str),
    String(String),
    Option(bool),

    TupleStart(uint),
    StructStart(&'static str, uint),
    EnumStart(&'static str, &'static str, uint),
    SeqStart(uint),
    MapStart(uint),

    End,
}

macro_rules! to_result {
    ($expr:expr, $err:expr) => {
        match $expr {
            Some(value) => Ok(value),
            None => $err,
        }
    }
}

pub trait Deserializer<E>: Iterator<Result<Token, E>> {
    fn end_of_stream_error<T>(&self) -> Result<T, E>;

    fn syntax_error<T>(&self, token: Token) -> Result<T, E>;

    fn missing_field_error<T>(&self, field: &'static str) -> Result<T, E>;

    #[inline]
    fn expect_token(&mut self) -> Result<Token, E> {
        match self.next() {
            Some(Ok(token)) => Ok(token),
            Some(Err(err)) => Err(err),
            None => self.end_of_stream_error(),
        }
    }

    #[inline]
    fn expect_null(&mut self, token: Token) -> Result<(), E> {
        match token {
            Null => Ok(()),
            TupleStart(_) => {
                match try!(self.expect_token()) {
                    End => Ok(()),
                    token => self.syntax_error(token),
                }
            }
            token => self.syntax_error(token),
        }
    }

    #[inline]
    fn expect_bool(&mut self, token: Token) -> Result<bool, E> {
        match token {
            Bool(value) => Ok(value),
            token => self.syntax_error(token),
        }
    }

    #[inline]
    fn expect_num<T: NumCast>(&mut self, token: Token) -> Result<T, E> {
        match token {
            Int(x) => to_result!(num::cast(x), self.syntax_error(Int(x))),
            I8(x) => to_result!(num::cast(x), self.syntax_error(I8(x))),
            I16(x) => to_result!(num::cast(x), self.syntax_error(I16(x))),
            I32(x) => to_result!(num::cast(x), self.syntax_error(I32(x))),
            I64(x) => to_result!(num::cast(x), self.syntax_error(I64(x))),
            Uint(x) => to_result!(num::cast(x), self.syntax_error(Uint(x))),
            U8(x) => to_result!(num::cast(x), self.syntax_error(U8(x))),
            U16(x) => to_result!(num::cast(x), self.syntax_error(U16(x))),
            U32(x) => to_result!(num::cast(x), self.syntax_error(U32(x))),
            U64(x) => to_result!(num::cast(x), self.syntax_error(U64(x))),
            F32(x) => to_result!(num::cast(x), self.syntax_error(F32(x))),
            F64(x) => to_result!(num::cast(x), self.syntax_error(F64(x))),
            token => self.syntax_error(token),
        }
    }

    #[inline]
    fn expect_from_primitive<T: FromPrimitive>(&mut self, token: Token) -> Result<T, E> {
        match token {
            Int(x) => to_result!(num::from_int(x), self.syntax_error(Int(x))),
            I8(x) => to_result!(num::from_i8(x), self.syntax_error(I8(x))),
            I16(x) => to_result!(num::from_i16(x), self.syntax_error(I16(x))),
            I32(x) => to_result!(num::from_i32(x), self.syntax_error(I32(x))),
            I64(x) => to_result!(num::from_i64(x), self.syntax_error(I64(x))),
            Uint(x) => to_result!(num::from_uint(x), self.syntax_error(Uint(x))),
            U8(x) => to_result!(num::from_u8(x), self.syntax_error(U8(x))),
            U16(x) => to_result!(num::from_u16(x), self.syntax_error(U16(x))),
            U32(x) => to_result!(num::from_u32(x), self.syntax_error(U32(x))),
            U64(x) => to_result!(num::from_u64(x), self.syntax_error(U64(x))),
            F32(x) => to_result!(num::from_f32(x), self.syntax_error(F32(x))),
            F64(x) => to_result!(num::from_f64(x), self.syntax_error(F64(x))),
            token => self.syntax_error(token),
        }
    }

    #[inline]
    fn expect_char(&mut self, token: Token) -> Result<char, E> {
        match token {
            Char(value) => Ok(value),
            token => self.syntax_error(token),
        }
    }

    #[inline]
    fn expect_str(&mut self, token: Token) -> Result<&'static str, E> {
        match token {
            Str(value) => Ok(value),
            token => self.syntax_error(token),
        }
    }

    #[inline]
    fn expect_string(&mut self, token: Token) -> Result<String, E> {
        match token {
            Str(value) => Ok(value.to_string()),
            String(value) => Ok(value),
            token => self.syntax_error(token),
        }
    }

    #[inline]
    fn expect_option<
        T: Deserializable
    >(&mut self, token: Token) -> Result<Option<T>, E> {
        match token {
            Option(false) => Ok(None),
            Option(true) => {
                let value: T = try!(Deserializable::deserialize(self));
                Ok(Some(value))
            }
            token => self.syntax_error(token),
        }
    }

    #[inline]
    fn expect_tuple_start(&mut self, token: Token) -> Result<uint, E> {
        match token {
            TupleStart(len) => Ok(len),
            token => self.syntax_error(token),
        }
    }

    #[inline]
    fn expect_tuple_elt<
        T: Deserializable
    >(&mut self) -> Result<T, E> {
        Deserializable::deserialize(self)
    }

    #[inline]
    fn expect_tuple_end(&mut self) -> Result<(), E> {
        match try!(self.expect_token()) {
            End => Ok(()),
            token => self.syntax_error(token),
        }
    }

    #[inline]
    fn expect_struct_start(&mut self, token: Token, name: &str) -> Result<(), E> {
        match token {
            StructStart(n, _) => {
                if name == n {
                    Ok(())
                } else {
                    self.syntax_error(token)
                }
            }
            _ => self.syntax_error(token),
        }
    }

    #[inline]
    fn expect_struct_field<
        T: Deserializable
    >(&mut self, name: &str) -> Result<T, E> {
        match try!(self.expect_token()) {
            Str(n) => {
                if name != n {
                    return self.syntax_error(Str(n));
                }
            }
            String(n) => {
                if name != n.as_slice() {
                    return self.syntax_error(String(n));
                }
            }
            token => { return self.syntax_error(token); }
        }

        Deserializable::deserialize(self)
    }

    #[inline]
    fn expect_struct_end(&mut self) -> Result<(), E> {
        match try!(self.expect_token()) {
            End => Ok(()),
            token => self.syntax_error(token),
        }
    }

    #[inline]
    fn expect_enum_start(&mut self, token: Token, name: &str, variants: &[&str]) -> Result<uint, E> {
        match token {
            EnumStart(n, v, _) => {
                if name == n {
                    match variants.iter().position(|variant| *variant == v) {
                        Some(position) => Ok(position),
                        None => self.syntax_error(token),
                    }
                } else {
                    self.syntax_error(token)
                }
            }
            _ => self.syntax_error(token),
        }
    }

    #[inline]
    fn expect_enum_elt<
        T: Deserializable
    >(&mut self) -> Result<T, E> {
        Deserializable::deserialize(self)
    }

    #[inline]
    fn expect_enum_end(&mut self) -> Result<(), E> {
        match try!(self.expect_token()) {
            End => Ok(()),
            token => self.syntax_error(token),
        }
    }

    #[inline]
    fn expect_seq_start(&mut self, token: Token) -> Result<uint, E> {
        match token {
            TupleStart(len) => Ok(len),
            SeqStart(len) => Ok(len),
            token => self.syntax_error(token),
        }
    }

    #[inline]
    fn expect_seq_elt_or_end<
        T: Deserializable
    >(&mut self) -> Result<Option<T>, E> {
        match try!(self.expect_token()) {
            End => Ok(None),
            token => {
                let value = try!(Deserializable::deserialize_token(self, token));
                Ok(Some(value))
            }
        }
    }

    #[inline]
    fn expect_seq<
        'a,
        T: Deserializable,
        C: FromIterator<T>
    >(&'a mut self, token: Token) -> Result<C, E> {
        let len = try!(self.expect_seq_start(token));

        let mut d: SeqDeserializer<'a, Self, E> = SeqDeserializer {
            d: self,
            len: len,
            err: None,
        };

        let collection: C = d.collect();

        match d.err {
            Some(err) => Err(err),
            None => Ok(collection),
        }
    }

    #[inline]
    fn expect_map_start(&mut self, token: Token) -> Result<uint, E> {
        match token {
            MapStart(len) => Ok(len),
            _ => self.syntax_error(token),
        }
    }

    #[inline]
    fn expect_map_elt_or_end<
        K: Deserializable,
        V: Deserializable
    >(&mut self) -> Result<Option<(K, V)>, E> {
        match try!(self.expect_token()) {
            End => Ok(None),
            token => {
                let key = try!(Deserializable::deserialize_token(self, token));
                let value = try!(Deserializable::deserialize(self));
                Ok(Some((key, value)))
            }
        }
    }

    #[inline]
    fn expect_map<
        'a,
        K: Deserializable,
        V: Deserializable,
        C: FromIterator<(K, V)>
    >(&'a mut self, token: Token) -> Result<C, E> {
        let len = try!(self.expect_map_start(token));

        let mut d: MapDeserializer<'a, Self, E> = MapDeserializer {
            d: self,
            len: len,
            err: None,
        };

        let collection: C = d.collect();

        match d.err {
            Some(err) => Err(err),
            None => Ok(collection),
        }
    }
}

//////////////////////////////////////////////////////////////////////////////

struct SeqDeserializer<'a, D, E> {
    d: &'a mut D,
    len: uint,
    err: Option<E>,
}

impl<
    'a,
    T: Deserializable,
    D: Deserializer<E>,
    E
> Iterator<T> for SeqDeserializer<'a, D, E> {
    #[inline]
    fn next(&mut self) -> Option<T> {
        match self.d.expect_seq_elt_or_end() {
            Ok(next) => next,
            Err(err) => {
                self.err = Some(err);
                None
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (uint, Option<uint>) {
        (self.len, Some(self.len))
    }
}

//////////////////////////////////////////////////////////////////////////////

struct MapDeserializer<'a, D, E> {
    d: &'a mut D,
    len: uint,
    err: Option<E>,
}

impl<
    'a,
    K: Deserializable,
    V: Deserializable,
    D: Deserializer<E>,
    E
> Iterator<(K, V)> for MapDeserializer<'a, D, E> {
    #[inline]
    fn next(&mut self) -> Option<(K, V)> {
        match self.d.expect_map_elt_or_end() {
            Ok(next) => next,
            Err(err) => {
                self.err = Some(err);
                None
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (uint, Option<uint>) {
        (self.len, Some(self.len))
    }
}

//////////////////////////////////////////////////////////////////////////////

pub trait Deserializable {
    #[inline]
    fn deserialize<
        D: Deserializer<E>,
        E
    >(d: &mut D) -> Result<Self, E> {
        let token = try!(d.expect_token());
        Deserializable::deserialize_token(d, token)
    }

    fn deserialize_token<
        D: Deserializer<E>,
        E
    >(d: &mut D, token: Token) -> Result<Self, E>;
}

//////////////////////////////////////////////////////////////////////////////

macro_rules! impl_deserializable {
    ($ty:ty, $method:ident) => {
        impl Deserializable for $ty {
            #[inline]
            fn deserialize_token<
                D: Deserializer<E>,
                E
            >(d: &mut D, token: Token) -> Result<$ty, E> {
                d.$method(token)
            }
        }
    }
}

impl_deserializable!(bool, expect_bool)
impl_deserializable!(int, expect_num)
impl_deserializable!(i8, expect_num)
impl_deserializable!(i16, expect_num)
impl_deserializable!(i32, expect_num)
impl_deserializable!(i64, expect_num)
impl_deserializable!(uint, expect_num)
impl_deserializable!(u8, expect_num)
impl_deserializable!(u16, expect_num)
impl_deserializable!(u32, expect_num)
impl_deserializable!(u64, expect_num)
impl_deserializable!(f32, expect_num)
impl_deserializable!(f64, expect_num)
impl_deserializable!(char, expect_char)
impl_deserializable!(&'static str, expect_str)
impl_deserializable!(String, expect_string)

//////////////////////////////////////////////////////////////////////////////

impl<T: Deserializable> Deserializable for Box<T> {
    #[inline]
    fn deserialize_token<
        D: Deserializer<E>,
        E
    >(d: &mut D, token: Token) -> Result<Box<T>, E> {
        Ok(box try!(Deserializable::deserialize_token(d, token)))
    }
}

impl<T: Deserializable + 'static> Deserializable for Gc<T> {
    #[inline]
    fn deserialize_token<
        D: Deserializer<E>,
        E
    >(d: &mut D, token: Token) -> Result<Gc<T>, E> {
        Ok(box (GC) try!(Deserializable::deserialize_token(d, token)))
    }
}

impl<T: Deserializable> Deserializable for Rc<T> {
    #[inline]
    fn deserialize_token<
        D: Deserializer<E>,
        E
    >(d: &mut D, token: Token) -> Result<Rc<T>, E> {
        Ok(Rc::new(try!(Deserializable::deserialize_token(d, token))))
    }
}

impl<T: Deserializable + Send + Share> Deserializable for Arc<T> {
    #[inline]
    fn deserialize_token<
        D: Deserializer<E>,
        E
    >(d: &mut D, token: Token) -> Result<Arc<T>, E> {
        Ok(Arc::new(try!(Deserializable::deserialize_token(d, token))))
    }
}

//////////////////////////////////////////////////////////////////////////////

impl<T: Deserializable> Deserializable for Option<T> {
    #[inline]
    fn deserialize_token<
        D: Deserializer<E>,
        E
    >(d: &mut D, token: Token) -> Result<Option<T>, E> {
        d.expect_option(token)
    }
}

//////////////////////////////////////////////////////////////////////////////

macro_rules! deserialize_seq {
    ($seq:expr) => {
        {
            loop {
                match d.next() {
                    Some(Ok(End)) => { break; }
                    Some(Ok(token)) => {
                        let v = try!(Deserializable::deserialize_token(d, token));
                        $seq.push(v)
                    }
                    Some(Err(err)) => { return Err(err); }
                    None => { return d.end_of_stream_error(); }
                }
            }

            Ok($seq)
        }
    }
}

impl<T: Deserializable> Deserializable for Vec<T> {
    #[inline]
    fn deserialize_token<
        D: Deserializer<E>,
        E
    >(d: &mut D, token: Token) -> Result<Vec<T>, E> {
        d.expect_seq(token)
    }
}

//////////////////////////////////////////////////////////////////////////////

impl<
    K: Deserializable + Eq + Hash,
    V: Deserializable
> Deserializable for HashMap<K, V> {
    #[inline]
    fn deserialize_token<
        D: Deserializer<E>,
        E
    >(d: &mut D, token: Token) -> Result<HashMap<K, V>, E> {
        d.expect_map(token)
    }
}

impl<
    K: Deserializable + Ord,
    V: Deserializable
> Deserializable for TreeMap<K, V> {
    #[inline]
    fn deserialize_token<
        D: Deserializer<E>,
        E
    >(d: &mut D, token: Token) -> Result<TreeMap<K, V>, E> {
        d.expect_map(token)
    }
}

//////////////////////////////////////////////////////////////////////////////

impl<
    T: Deserializable + Eq + Hash
> Deserializable for HashSet<T> {
    #[inline]
    fn deserialize_token<
        D: Deserializer<E>,
        E
    >(d: &mut D, token: Token) -> Result<HashSet<T>, E> {
        d.expect_seq(token)
    }
}

impl<
    T: Deserializable + Ord
> Deserializable for TreeSet<T> {
    #[inline]
    fn deserialize_token<
        D: Deserializer<E>,
        E
    >(d: &mut D, token: Token) -> Result<TreeSet<T>, E> {
        d.expect_seq(token)
    }
}

//////////////////////////////////////////////////////////////////////////////

macro_rules! peel {
    ($name:ident, $($other:ident,)*) => (impl_deserialize_tuple!($($other,)*))
}

macro_rules! impl_deserialize_tuple {
    () => {
        impl Deserializable for () {
            #[inline]
            fn deserialize_token<
                D: Deserializer<E>,
                E
            >(d: &mut D, token: Token) -> Result<(), E> {
                d.expect_null(token)
            }
        }
    };
    ( $($name:ident,)+ ) => {
        impl<
            $($name: Deserializable),*
        > Deserializable for ($($name,)*) {
            #[inline]
            #[allow(uppercase_variables)]
            fn deserialize_token<
                D: Deserializer<E>,
                E
            >(d: &mut D, token: Token) -> Result<($($name,)*), E> {
                try!(d.expect_tuple_start(token));

                let result = ($({
                    let $name = try!(d.expect_tuple_elt());
                    $name
                },)*);

                try!(d.expect_tuple_end());

                Ok(result)
            }
        }
        peel!($($name,)*)
    }
}

impl_deserialize_tuple! { T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, }

//////////////////////////////////////////////////////////////////////////////

/// Helper struct that will ignore tokens while taking in consideration
/// recursive structures.
pub struct IgnoreTokens;

impl Deserializable for IgnoreTokens {
    #[inline]
    fn deserialize_token<
        D: Deserializer<E>,
        E
    >(d: &mut D, token: Token) -> Result<IgnoreTokens, E> {
        match token {
            Option(true) => {
                Deserializable::deserialize(d)
            }

            EnumStart(_, _, _) => {
                loop {
                    match try!(d.expect_token()) {
                        End => { return Ok(IgnoreTokens); }
                        token => {
                            let _: IgnoreTokens = try!(Deserializable::deserialize_token(d, token));
                        }
                    }
                }
            }

            StructStart(_, _) => {
                loop {
                    match try!(d.expect_token()) {
                        End => { return Ok(IgnoreTokens); }
                        Str(_) | String(_) => {
                            let _: IgnoreTokens = try!(Deserializable::deserialize(d));
                        }
                        _token => { return d.syntax_error(token); }
                    }
                }
            }

            TupleStart(_) => {
                loop {
                    match try!(d.expect_token()) {
                        End => { return Ok(IgnoreTokens); }
                        token => {
                            let _: IgnoreTokens = try!(Deserializable::deserialize_token(d, token));
                        }
                    }
                }
            }

            SeqStart(_) => {
                loop {
                    match try!(d.expect_token()) {
                        End => { return Ok(IgnoreTokens); }
                        token => {
                            let _: IgnoreTokens = try!(Deserializable::deserialize_token(d, token));
                        }
                    }
                }
            }

            MapStart(_) => {
                loop {
                    match try!(d.expect_token()) {
                        End => { return Ok(IgnoreTokens); }
                        token => {
                            let _: IgnoreTokens = try!(Deserializable::deserialize_token(d, token));
                            let _: IgnoreTokens = try!(Deserializable::deserialize(d));
                        }
                    }
                }
            }

            End => d.syntax_error(token),

            _ => Ok(IgnoreTokens),
        }
    }
}

//////////////////////////////////////////////////////////////////////////////

/// Helper struct that will gather tokens while taking in consideration
/// recursive structures.
pub struct GatherTokens {
    tokens: Vec<Token>,
}

impl GatherTokens {
    #[inline]
    pub fn unwrap(self) -> Vec<Token> {
        self.tokens
    }

    #[inline]
    fn gather<D: Deserializer<E>, E>(&mut self, d: &mut D) -> Result<(), E> {
        let token = try!(d.expect_token());
        self.gather_token(d, token)
    }

    #[inline]
    fn gather_token<D: Deserializer<E>, E>(&mut self, d: &mut D, token: Token) -> Result<(), E> {
        match token {
            token @ Option(true) => {
                self.tokens.push(token);
                self.gather(d)
            }
            EnumStart(name, variant, len) => {
                self.tokens.reserve_additional(len + 1);
                self.tokens.push(EnumStart(name, variant, len));
                self.gather_seq(d)
            }
            StructStart(name, len) => {
                self.tokens.reserve_additional(len + 1);
                self.tokens.push(StructStart(name, len));
                self.gather_struct(d)
            }
            TupleStart(len) => {
                self.tokens.reserve_additional(len + 1);
                self.tokens.push(TupleStart(len));
                self.gather_seq(d)
            }
            SeqStart(len) => {
                self.tokens.reserve_additional(len + 1);
                self.tokens.push(SeqStart(len));
                self.gather_seq(d)
            }
            MapStart(len) => {
                self.tokens.reserve_additional(len + 1);
                self.tokens.push(MapStart(len));
                self.gather_map(d)
            }
            End => {
                d.syntax_error(token)
            }
            token => {
                self.tokens.push(token);
                Ok(())
            }
        }
    }

    #[inline]
    fn gather_seq<D: Deserializer<E>, E>(&mut self, d: &mut D) -> Result<(), E> {
        loop {
            match try!(d.expect_token()) {
                token @ End => {
                    self.tokens.push(token);
                    return Ok(());
                }
                token => {
                    try!(self.gather_token(d, token));
                }
            }
        }
    }

    #[inline]
    fn gather_struct<D: Deserializer<E>, E>(&mut self, d: &mut D) -> Result<(), E> {
        loop {
            match try!(d.expect_token()) {
                token @ End => {
                    self.tokens.push(token);
                    return Ok(());
                }
                token @ Str(_) | token @ String(_) => {
                    self.tokens.push(token);
                    try!(self.gather(d))
                }
                token => { return d.syntax_error(token); }
            }
        }
    }

    #[inline]
    fn gather_map<D: Deserializer<E>, E>(&mut self, d: &mut D) -> Result<(), E> {
        loop {
            match try!(d.expect_token()) {
                End => {
                    self.tokens.push(End);
                    return Ok(());
                }
                token => {
                    try!(self.gather_token(d, token));
                    try!(self.gather(d))
                }
            }
        }
    }
}

impl Deserializable for GatherTokens {
    #[inline]
    fn deserialize_token<
        D: Deserializer<E>,
        E
    >(d: &mut D, token: Token) -> Result<GatherTokens, E> {
        let mut tokens = GatherTokens {
            tokens: vec!(),
        };
        try!(tokens.gather_token(d, token));
        Ok(tokens)
    }
}

//////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use std::collections::TreeMap;
    use serialize::Decoder;

    use super::{Deserializer, Deserializable, Token};
    use super::{
        Null,
        Bool,
        Int,
        I8,
        I16,
        I32,
        I64,
        Uint,
        U8,
        U16,
        U32,
        U64,
        F32,
        F64,
        Char,
        Str,
        String,
        Option,
        TupleStart,
        StructStart,
        EnumStart,
        SeqStart,
        MapStart,
        End,
    };

    macro_rules! treemap {
        ($($k:expr => $v:expr),*) => ({
            let mut _m = ::std::collections::TreeMap::new();
            $(_m.insert($k, $v);)*
            _m
        })
    }

    //////////////////////////////////////////////////////////////////////////////

    #[deriving(Clone, PartialEq, Show, Decodable)]
    struct Inner {
        a: (),
        b: uint,
        c: TreeMap<String, Option<char>>,
    }

    impl Deserializable for Inner {
        #[inline]
        fn deserialize_token<
            D: Deserializer<E>, E
        >(d: &mut D, token: Token) -> Result<Inner, E> {
            try!(d.expect_struct_start(token, "Inner"));
            let a = try!(d.expect_struct_field("a"));
            let b = try!(d.expect_struct_field("b"));
            let c = try!(d.expect_struct_field("c"));
            try!(d.expect_struct_end());
            Ok(Inner { a: a, b: b, c: c })
        }
    }

    //////////////////////////////////////////////////////////////////////////////

    #[deriving(Clone, PartialEq, Show, Decodable)]
    struct Outer {
        inner: Vec<Inner>,
    }

    impl Deserializable for Outer {
        #[inline]
        fn deserialize_token<
            D: Deserializer<E>, E
        >(d: &mut D, token: Token) -> Result<Outer, E> {
            try!(d.expect_struct_start(token, "Outer"));
            let inner = try!(d.expect_struct_field("inner"));
            try!(d.expect_struct_end());
            Ok(Outer { inner: inner })
        }
    }

    //////////////////////////////////////////////////////////////////////////////

    #[deriving(Clone, PartialEq, Show, Decodable)]
    enum Animal {
        Dog,
        Frog(String, int)
    }

    impl Deserializable for Animal {
        #[inline]
        fn deserialize_token<
            D: Deserializer<E>, E
        >(d: &mut D, token: Token) -> Result<Animal, E> {
            match try!(d.expect_enum_start(token, "Animal", ["Dog", "Frog"])) {
                0 => {
                    try!(d.expect_enum_end());
                    Ok(Dog)
                }
                1 => {
                    let x0 = try!(Deserializable::deserialize(d));
                    let x1 = try!(Deserializable::deserialize(d));
                    try!(d.expect_enum_end());
                    Ok(Frog(x0, x1))
                }
                _ => unreachable!(),
            }
        }
    }

    //////////////////////////////////////////////////////////////////////////////

    #[deriving(Show)]
    enum Error {
        EndOfStream,
        SyntaxError,
        IncompleteValue,
    }

    //////////////////////////////////////////////////////////////////////////////

    struct TokenDeserializer<Iter> {
        tokens: Iter,
    }

    impl<Iter: Iterator<Token>> TokenDeserializer<Iter> {
        #[inline]
        fn new(tokens: Iter) -> TokenDeserializer<Iter> {
            TokenDeserializer {
                tokens: tokens,
            }
        }
    }

    impl<Iter: Iterator<Token>> Iterator<Result<Token, Error>> for TokenDeserializer<Iter> {
        #[inline]
        fn next(&mut self) -> Option<Result<Token, Error>> {
            match self.tokens.next() {
                None => None,
                Some(token) => Some(Ok(token)),
            }
        }
    }

    impl<Iter: Iterator<Token>> Deserializer<Error> for TokenDeserializer<Iter> {
        fn end_of_stream_error<T>(&self) -> Result<T, Error> {
            Err(EndOfStream)
        }

        fn syntax_error<T>(&self, _token: Token) -> Result<T, Error> {
            Err(SyntaxError)
        }

        fn missing_field_error<T>(&self, _field: &'static str) -> Result<T, Error> {
            Err(IncompleteValue)
        }
    }

    //////////////////////////////////////////////////////////////////////////////

    macro_rules! test_value {
        ($name:ident, [$($tokens:expr => $value:expr: $ty:ty),*]) => {
            #[test]
            fn $name() {
                $(
                    let mut deserializer = TokenDeserializer::new($tokens.move_iter());
                    let value: $ty = Deserializable::deserialize(&mut deserializer).unwrap();

                    assert_eq!(value, $value);
                )+
            }
        }
    }

    test_value!(test_primitives, [
        vec!(Null) => (): (),
        vec!(Bool(true)) => true: bool,
        vec!(Bool(false)) => false: bool,
        vec!(Int(5)) => 5: int,
        vec!(I8(5)) => 5: i8,
        vec!(I16(5)) => 5: i16,
        vec!(I32(5)) => 5: i32,
        vec!(I64(5)) => 5: i64,
        vec!(Uint(5)) => 5: uint,
        vec!(U8(5)) => 5: u8,
        vec!(U16(5)) => 5: u16,
        vec!(U32(5)) => 5: u32,
        vec!(U64(5)) => 5: u64,
        vec!(F32(5.0)) => 5.0: f32,
        vec!(F64(5.0)) => 5.0: f64,
        vec!(Char('c')) => 'c': char,
        vec!(Str("abc")) => "abc": &str,
        vec!(String("abc".to_string())) => "abc".to_string(): String
    ])

    test_value!(test_tuples, [
        vec!(
            TupleStart(0),
            End,
        ) => (): (),

        vec!(
            TupleStart(2),
                Int(5),

                Str("a"),
            End,
        ) => (5, "a"): (int, &'static str),

        vec!(
            TupleStart(3),
                Null,

                TupleStart(0),
                End,

                TupleStart(2),
                    Int(5),

                    Str("a"),
                End,
            End,
        ) => ((), (), (5, "a")): ((), (), (int, &'static str))
    ])

    test_value!(test_options, [
        vec!(Option(false)) => None: Option<int>,

        vec!(
            Option(true),
            Int(5),
        ) => Some(5): Option<int>
    ])

    test_value!(test_structs, [
        vec!(
            StructStart("Outer", 1),
                Str("inner"),
                SeqStart(0),
                End,
            End,
        ) => Outer { inner: vec!() }: Outer,

        vec!(
            StructStart("Outer", 1),
                Str("inner"),
                SeqStart(1),
                    StructStart("Inner", 3),
                        Str("a"),
                        Null,

                        Str("b"),
                        Uint(5),

                        Str("c"),
                        MapStart(1),
                            String("abc".to_string()),

                            Option(true),
                            Char('c'),
                        End,
                    End,
                End,
            End,
        ) => Outer {
            inner: vec!(
                Inner {
                    a: (),
                    b: 5,
                    c: treemap!("abc".to_string() => Some('c')),
                },
            ),
        }: Outer
    ])

    test_value!(test_enums, [
        vec!(
            EnumStart("Animal", "Dog", 0),
            End,
        ) => Dog: Animal,

        vec!(
            EnumStart("Animal", "Frog", 2),
                String("Henry".to_string()),
                Int(349),
            End,
        ) => Frog("Henry".to_string(), 349): Animal
    ])

    test_value!(test_vecs, [
        vec!(
            SeqStart(0),
            End,
        ) => vec!(): Vec<int>,

        vec!(
            SeqStart(3),
                Int(5),

                Int(6),

                Int(7),
            End,
        ) => vec!(5, 6, 7): Vec<int>,


        vec!(
            SeqStart(3),
                SeqStart(1),
                    Int(1),
                End,

                SeqStart(2),
                    Int(2),

                    Int(3),
                End,

                SeqStart(3),
                    Int(4),

                    Int(5),

                    Int(6),
                End,
            End,
        ) => vec!(vec!(1), vec!(2, 3), vec!(4, 5, 6)): Vec<Vec<int>>
    ])

    test_value!(test_treemaps, [
        vec!(
            MapStart(0),
            End,
        ) => treemap!(): TreeMap<int, String>,

        vec!(
            MapStart(2),
                Int(5),
                String("a".to_string()),

                Int(6),
                String("b".to_string()),
            End,
        ) => treemap!(5i => "a".to_string(), 6i => "b".to_string()): TreeMap<int, String>
    ])
}
