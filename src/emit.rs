// Copyright Christian Daley 2021
// Copyright Frank Mori Hess 2007-2008.
// Distributed under the Boost Software License, Version 1.0. 
// See http://www.boost.org/LICENSE_1_0.txt

use std::mem;

use crate::{Signal, EmitHandle};
use crate::combiner::Combiner;

macro_rules! impl_emit {
    ($name:ident; $($args:ident)*; $($params:ident)*) => {

        /// Emit trait for signals with slots that accept the corresponding number of arguments. 
        pub trait $name<R, C, $($args,)*> 
        where 
            ($($args,)*): Clone,
            C: Combiner<R> + 'static
        {
            /// The return value of `emit` will be `C::Output` for [Signals](Signal) and `Option<C::Output>` for [EmitHandles](EmitHandle)
            type Output;
            /// Executes the signal's underlying slots, passing clones of the given arguments to the slot
            /// functions. 
            fn emit(&self, $($params: $args,)*) -> Self::Output;
        }

        impl<R, C, G, $($args,)*> $name<R, C, $($args,)*> for Signal<($($args,)*), R, C, G> 
        where 
            ($($args,)*): Clone,
            C: Combiner<R> + 'static,
            G: Ord + Send + Sync
        {
            type Output = C::Output;

            fn emit(&self, $($params: $args,)*) -> C::Output {
                let lock = self.core.read().unwrap();
                let handle = lock.clone();
                mem::drop(lock);
                handle.emit(&($($params,)*))
            }
        }

        impl<R, C, G, $($args,)*> $name<R, C, $($args,)*> for EmitHandle<($($args,)*), R, C, G> 
        where 
            ($($args,)*): Clone,
            C: Combiner<R> + 'static,
            G: Ord + Send + Sync
        {
            type Output = Option<C::Output>;

            fn emit(&self, $($params: $args,)*) -> Option<C::Output> {
                self.weak_sig
                    .upgrade()
                    .map(|sig| sig.emit($($params,)*))
            }
        }
    };
}

impl_emit!(Emit0;;);
impl_emit!(Emit1; T0; a);
impl_emit!(Emit2; T0 T1; a b);
impl_emit!(Emit3; T0 T1 T2; a b c);
impl_emit!(Emit4; T0 T1 T2 T3; a b c d);
impl_emit!(Emit5; T0 T1 T2 T3 T4; a b c d e);
impl_emit!(Emit6; T0 T1 T2 T3 T4 T5; a b c d e f);
impl_emit!(Emit7; T0 T1 T2 T3 T4 T5 T6; a b c d e f g);
impl_emit!(Emit8; T0 T1 T2 T3 T4 T5 T6 T7; a b c d e f g h);
impl_emit!(Emit9; T0 T1 T2 T3 T4 T5 T6 T7 T8; a b c d e f g h i);
impl_emit!(Emit10; T0 T1 T2 T3 T4 T5 T6 T7 T8 T9; a b c d e f g h i j);
impl_emit!(Emit11; T0 T1 T2 T3 T4 T5 T6 T7 T8 T9 T10; a b c d e f g h i j k);
impl_emit!(Emit12; T0 T1 T2 T3 T4 T5 T6 T7 T8 T9 T10 T11; a b c d e f g h i j k l);