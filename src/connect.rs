// Copyright Christian Daley 2021
// Copyright Frank Mori Hess 2007-2008.
// Distributed under the Boost Software License, Version 1.0. 
// See http://www.boost.org/LICENSE_1_0.txt

use std::sync::{Arc, Weak, Mutex};

use crate::Signal;
use crate::signal_core::UntypedSignalCore;
use crate::combiner::Combiner;

/// Represents a position to connect a slot to in a group of slots.
pub enum Position {
    /// A position at the front of a group. A slot connected at `Position::Front` be executed 
    /// *before* all other slots in the group that were present before it was conncted.
    Front,
    /// A position at the back of a group. A slot connected at `Position::Back` be executed 
    /// *after* all other slots in the group that were present before it was conncted.
    Back
}

/// Represents a group to connect a slot to in a signal.
#[derive(Ord, PartialOrd, Eq, PartialEq)]
pub enum Group<G>
where
    G: Ord + Send + Sync
{
    /// The unnamed "front" group. Slots in the "front" grouped are executed before
    /// slots from named groups and the unnamed "back" group.
    Front,
    /// A named group. Slots in named groups are executed after all slots in the
    /// unnamed "front" group. Individual named groups are executed according the ordering defined by the
    /// [Ord] trait.
    Named(G),
    /// The unnamed "back" group. Slots in the "back" grouped are executed after
    /// slots from the unnamed "front" group and slots from named groups.
    Back
}

macro_rules! impl_connect {
    ($name:ident; $($args:ident)*; $($params:ident)*) => {

        /// Connect trait for signals with slots that accept the corresponding number of arguments. 
        pub trait $name<R, C, G, $($args),*>
        where 
            ($($args,)*): Clone + 'static,
            R: 'static,
            C: Combiner<R> + 'static,
            G: Ord + Send + Sync
        {
            /// Connects the slot function `f` to the given [Group] at the given [Position]
            fn connect_group_position<F>(&self, f: F, group: Group<G>, pos: Position) -> Connection
            where 
                F: Fn($($args,)*) -> R + Send + Sync + 'static;

            /// Connects the extended slot function `f` to the given [Group] at the given [Position]
            fn connect_group_position_extended<F>(&self, f: F, group: Group<G>, pos: Position) -> Connection
            where 
                F: Fn(Connection, $($args,)*) -> R + Send + Sync + 'static;

            /// Connects the slot function `f` to the given [Group] at [Position::Back]. Equivalent to calling
            /// `connect_group_position(f, group, Position::Back)`.
            fn connect_group<F>(&self, f: F, group: Group<G>) -> Connection
            where 
                F: Fn($($args,)*) -> R + Send + Sync + 'static
            {
                self.connect_group_position(f, group, Position::Back)
            }

            /// Connects the slot function `f` to [Group::Back] at the given position. Equivalent to calling
            /// `connect_group_position(f, Group::Back, pos)`.
            fn connect_position<F>(&self, f: F, pos: Position) -> Connection
            where 
                F: Fn($($args,)*) -> R + Send + Sync + 'static
            {
                self.connect_group_position(f, Group::Back, pos)
            }

            /// Connects the slot function `f` to [Group::Back] at [Position::Back]. Equivalent to calling
            /// `connect_group_position(f, Group::Back, Position::Back)`.
            fn connect<F>(&self, f: F) -> Connection
            where 
                F: Fn($($args,)*) -> R + Send + Sync + 'static
            {
                self.connect_group_position(f, Group::Back, Position::Back)
            }

            /// Connects the extended slot function `f` to the given [Group] at [Position::Back]. Equivalent to calling
            /// `connect_group_position_extended(f, group, Position::Back)`.
            fn connect_group_extended<F>(&self, f: F, group: Group<G>) -> Connection
            where 
                F: Fn(Connection, $($args,)*) -> R + Send + Sync + 'static
            {
                self.connect_group_position_extended(f, group, Position::Back)
            }

            /// Connects the extended slot function `f` to [Group::Back] at the given position. Equivalent to calling
            /// `connect_group_position_extended(f, Group::Back, pos)`.
            fn connect_position_extended<F>(&self, f: F, pos: Position) -> Connection
            where 
                F: Fn(Connection, $($args,)*) -> R + Send + Sync + 'static
            {
                self.connect_group_position_extended(f, Group::Back, pos)
            }

             /// Connects the extended slot function `f` to [Group::Back] at [Position::Back]. Equivalent to calling
            /// `connect_group_position_extended(f, Group::Back, Position::Back)`.
            fn connect_extended<F>(&self, f: F) -> Connection
            where 
                F: Fn(Connection, $($args,)*) -> R + Send + Sync + 'static
            {
                self.connect_group_position_extended(f, Group::Back, Position::Back)
            }
        }

        impl<R, C, G, $($args,)*> $name<R, C, G, $($args,)*> for Signal<($($args,)*), R, C, G> 
        where
            ($($args,)*): Clone + 'static,
            R: 'static,
            C: Combiner<R> + 'static,
            G: Ord + Send + Sync + 'static,
        {
            fn connect_group_position<F>(&self, f: F, group: Group<G>, pos: Position) -> Connection
            where
                F: Fn($($args,)*) -> R + Send + Sync + 'static
            {
                let untyped_core: Arc<dyn UntypedSignalCore> = self.core.clone();
                let make_conn = |id| Connection::new(Arc::downgrade(&untyped_core), id);

                let mut lock = self.core.lock().unwrap();
                let mut core_clone = (**lock).clone();

                let wrapped_f = move |($($params,)*)| f($($params,)*);
                let conn = core_clone.connect(wrapped_f, group, pos, make_conn);

                *lock = Arc::new(core_clone);
                conn
            }

            fn connect_group_position_extended<F>(&self, f: F, group: Group<G>, pos: Position) -> Connection
            where
                F: Fn(Connection, $($args,)*) -> R + Send + Sync + 'static
            {
                let untyped_core: Arc<dyn UntypedSignalCore> = self.core.clone();
                let make_conn = |id| Connection::new(Arc::downgrade(&untyped_core), id);

                let mut lock = self.core.lock().unwrap();
                let mut core_clone = (**lock).clone();

                let wrapped_f = move |conn, ($($params,)*)| f(conn, $($params,)*);
                let conn = core_clone.connect_extended(wrapped_f, group, pos, make_conn);

                *lock = Arc::new(core_clone);
                conn
            }
        }
    };
}

impl_connect!(Connect0;;);
impl_connect!(Connect1; T0; a);
impl_connect!(Connect2; T0 T1; a b);
impl_connect!(Connect3; T0 T1 T2; a b c);
impl_connect!(Connect4; T0 T1 T2 T3; a b c d);
impl_connect!(Connect5; T0 T1 T2 T3 T4; a b c d e);
impl_connect!(Connect6; T0 T1 T2 T3 T4 T5; a b c d e f);
impl_connect!(Connect7; T0 T1 T2 T3 T4 T5 T6; a b c d e f g);
impl_connect!(Connect8; T0 T1 T2 T3 T4 T5 T6 T7; a b c d e f g h);
impl_connect!(Connect9; T0 T1 T2 T3 T4 T5 T6 T7 T8; a b c d e f g h i);
impl_connect!(Connect10; T0 T1 T2 T3 T4 T5 T6 T7 T8 T9; a b c d e f g h i j);
impl_connect!(Connect11; T0 T1 T2 T3 T4 T5 T6 T7 T8 T9 T10; a b c d e f g h i j k);
impl_connect!(Connect12; T0 T1 T2 T3 T4 T5 T6 T7 T8 T9 T10 T11; a b c d e f g h i j k l);

/// The implementation used by both [Connection] and [ScopedConnection].
/// Takes a const bool parameter indicating whether it is a scoped connection or not.
#[derive(Clone)]
pub struct ConnectionImpl<const SCOPED: bool>
{
    weak_core: Weak<dyn UntypedSignalCore>,
    slot_id: usize
}

impl<const SCOPED: bool> ConnectionImpl<SCOPED> {
    fn new(weak_core: Weak<dyn UntypedSignalCore>, slot_id: usize) -> Self {
        Self {
            weak_core,
            slot_id
        }
    }

    /// Returns true if the underlying slot is still connected, false otherwise. Will return false 
    /// if the underlying signal no longer exists.
    pub fn connected(&self) -> bool {
        match self.weak_core.upgrade() {
            Some(core) => {
                core.connected(self.slot_id)
            }
            None => false
        }
    }

    /// Disconnects the underlying slot. Further, repeated calls to `disconnect` will do nothing.
    /// When a connection is disconnected its underlying slot is permanently removed from the the signal's slot list.
    /// Once disconnected, there is no way to re-connect a slot.
    pub fn disconnect(&self) {
        if let Some(core) = self.weak_core.upgrade() {
            core.disconnect(self.slot_id);
        }
    }

    /// Returns true if the underlying slot is blocked, false otherwise. Will return true if either the
    /// underyling slot or underlying signal no longer exists.
    pub fn blocked(&self) -> bool {        
        match self.weak_core.upgrade() {
            Some(core) => {
                core.blocked(self.slot_id)
            }
            None => true
        }
    }

    /// Returns the number of [SharedConnectionBlocks](SharedConnectionBlock) currently blocking the slot. 
    /// Will return `usize::Max` if either the underyling slot or underlying signal no longer exists.
    pub fn blocker_count(&self) -> usize {
        match self.weak_core.upgrade() {
            Some(core) => {
                core.blocker_count(self.slot_id)
            }
            None => usize::MAX
        }
    }

    #[must_use="shared connection blocks are automatically unblocked when dropped"]
    /// Gets a [SharedConnectionBlock] that can be used to temporarily block the underlying slot.
    pub fn shared_block(&self, initially_blocking: bool) -> SharedConnectionBlock {
        SharedConnectionBlock::new(self.weak_core.clone(), self.slot_id, initially_blocking)
    }
}

impl<const SCOPED: bool> Drop for ConnectionImpl<SCOPED> {
    /// Disconnects the connection if and only if the connection is scoped.
    fn drop(&mut self) {
        if SCOPED {
            self.disconnect();
        }
    }
}

impl ConnectionImpl<false> {
    /// Consumes the connection and returns a [ScopedConnection].
    #[must_use="ScopedConnection automatically disconnects when dropped"]
    pub fn scoped(self) -> ScopedConnection {
        ScopedConnection::new(self.weak_core.clone(), self.slot_id)
    }
}

/// A connection manages one slot for one particular signal. Connections carry no type information about their
/// underlying signal. Connections are created when new slots are connected to a signal.
/// 
/// Note that when a connection is dropped it *will not* automatically disconnect its underlying slot.
/// See [ScopedConnection] for a connection that automatically disconnects when dropped.
///
/// See [ConnectionImpl] for details on the various functions implemented by connections.
/// # Examples 
/// ```
/// use signals2::*;
/// 
/// let sig: Signal<(), i32> = Signal::new();
/// let conn = sig.connect(|| 4);
/// assert_eq!(sig.emit(), Some(4));
/// conn.disconnect(); // disconnect the slot
/// assert_eq!(sig.emit(), None);
/// ```
pub type Connection = ConnectionImpl<false>;


/// Scoped connections are identical to regular connections, except that they will automcatically
/// disconnect themselves when dropped.
///
/// See [ConnectionImpl] for details on the various functions implemented by scoped connections.
/// ```
/// use signals2::*;
/// 
/// let sig: Signal<(), i32> = Signal::new();
/// {
///    let _conn = sig.connect(|| 4).scoped(); // create a scoped connection
///    assert_eq!(sig.emit(), Some(4));
/// }
/// 
/// assert_eq!(sig.emit(), None);
/// ```
pub type ScopedConnection = ConnectionImpl<true>;

/// A shared connection block can be used to temporarily block a slot from executing. There can be an
/// arbitrary number of shared connection blocks for any particular slot. If any of the shared connection blocks
/// are blocking the slot, that slot will not be executed when the signal is emitted.
/// # Examples 
/// ```
/// use signals2::*;
/// 
/// let sig: Signal<(), i32> = Signal::new();
/// let conn = sig.connect(|| 4);
/// assert_eq!(sig.emit(), Some(4));
///
/// let blocker1 = conn.shared_block(true); // blocking
/// let blocker2 = blocker1.clone(); // also blocking, since blocker1 is blocking
///
/// assert_eq!(conn.blocker_count(), 2);
/// assert_eq!(sig.emit(), None); // slot is blocked and will not execute
/// 
/// blocker1.unblock();
/// assert_eq!(conn.blocker_count(), 1);
/// assert_eq!(sig.emit(), None); // slot is still blocked
///
/// blocker2.unblock();
/// assert_eq!(conn.blocker_count(), 0);
/// assert_eq!(sig.emit(), Some(4)); // no more active blockers
/// ```
///  Shared connection blocks automatically unblock themselved when dropped.
/// ```
/// use signals2::*;
/// 
/// let sig: Signal<(), i32> = Signal::new();
/// let conn = sig.connect(|| 4);
/// assert_eq!(sig.emit(), Some(4));
/// {
///    let _blocker = conn.shared_block(true);
///    assert_eq!(conn.blocker_count(), 1);
///    assert_eq!(sig.emit(), None);
/// }
///
/// assert_eq!(conn.blocker_count(), 0);
/// assert_eq!(sig.emit(), Some(4)); // blocker was dropped
/// ```
pub struct SharedConnectionBlock {
    weak_core: Weak<dyn UntypedSignalCore>,
    slot_id: usize,
    blocking: Mutex<bool>
}

impl SharedConnectionBlock {
    fn new(weak_core: Weak<dyn UntypedSignalCore>, slot_id: usize, initially_blocking: bool) -> Self {
        let shared_block = Self {
            weak_core,
            slot_id,
            blocking: Mutex::new(false)
        };

        if initially_blocking {
            shared_block.block_impl(true);
        }

        shared_block
    }

    /// Causes the `SharedConnectionBlock` to begin blocking, if it isn't already.
    pub fn block(&self) {
        if !self.blocking() {
            self.block_impl(true);
        }
    }

    /// Causes the `SharedConnectionBlock` to stop blocking, if it isn't already.
    pub fn unblock(&self) {
        if self.blocking() {
            self.block_impl(false);
        }
    }

    /// Returns true if the `SharedConnectionBlock` is currently blocking, false otherwise.
    /// If this function returns `true` it is guaranteed that the given slot will not be executed when
    /// the signal is emitted. However, if this function returns `false` it is not guaranteed that the given
    /// slot will be executed when the signal is emitted because there could be other existing blockers for
    /// the slot.
    pub fn blocking(&self) -> bool {
        *self.blocking.lock().unwrap()
    }

    fn block_impl(&self, block: bool) {
        if let Some(core) = self.weak_core.upgrade() {
            core.block(self.slot_id, block);
        }

        let mut lock = self.blocking.lock().unwrap();
        *lock = block;
    }
}

impl Clone for SharedConnectionBlock {
    /// Creates a copy of the given `SharedConnectionBlock` with the same blocking state.
    fn clone(&self) -> Self {
        SharedConnectionBlock::new(self.weak_core.clone(), self.slot_id, self.blocking())
    }
}

impl Drop for SharedConnectionBlock {
    /// Unblocks the underlying signal, if it sitll exists.
    fn drop(&mut self) {
        self.unblock();
    }
}