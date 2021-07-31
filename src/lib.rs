// Copyright Christian Daley 2021
// Copyright Frank Mori Hess 2007-2008.
// Distributed under the Boost Software License, Version 1.0. 
// See http://www.boost.org/LICENSE_1_0.txt

//! # signals2
//!
//! `signals2` is a thread-safe signal/slot library based on the [boost::signals2](https://www.boost.org/doc/libs/1_76_0/doc/html/signals2.html)
//! C++ library. [Signals](Signal) are objects that contain a list of callback functions ("slots") to be executed when the signal is
//! "emitted". Signals and their corresponding slots can be managed through the use of [connections](Connection)
//! and [shared connection blocks](SharedConnectionBlock).
//!
//! `signals2` contains no unsafe code and compiles on stable Rust 1.53. 
//! 
//! `signals2` is distributed under the [Boost Software License, Version 1.0](https://www.boost.org/LICENSE_1_0.txt).
//!
//! ### Links
//! * [Github](https://github.com/christiandaley/signals2/)

#![deny(missing_docs)]

use std::sync::{Arc, Weak, RwLock};

mod signal_core;
use signal_core::{SignalCore};

/// Defines the combiner trait and several simple combiners that can be used.
pub mod combiner;
use combiner::{Combiner, DefaultCombiner};

/// Defines different `emit` traits for signals.
pub mod emit;
#[doc(inline)]
pub use emit::{Emit0, Emit1, Emit2, Emit3, Emit4, Emit5, Emit6, Emit7, Emit8, Emit9, Emit10, Emit11, Emit12};

/// Defines different `connect` traits for signals.
pub mod connect;
#[doc(inline)]
pub use connect::{SharedConnectionBlock, Connection, ScopedConnection, Position, Group, 
    Connect0, Connect1, Connect2, Connect3, Connect4, Connect5, Connect6, Connect7, Connect8,
    Connect9, Connect10, Connect11, Connect12};

/// A handle to a signal with a slot function signature of `Args -> R`. `C` defines the combiner used
/// to generate a return value when `emit` is envoked. `G` defines the ordering of groups of slots. **Arguments given
/// to the signal must implement `Clone`. If you need to emit a signal with an argument that doesn't implement clone, that
/// argument should be wrapped in an `Arc<T>` (as an example) to make it cloneable.**
/// # Examples
/// ```
/// use signals2::*;
/// 
/// let sig: Signal<()> = Signal::new();
/// sig.connect(|| println!("Hello, world!"));
/// sig.emit(); // prints "Hello, world!"
/// ```
/// The only required template parameter for a `Signal` is the type of the parameters that the slot functions will
/// accept, represented as a tuple. If we want our signal to have slot functions that accept two `i32`s as parameters, the
/// template parameter will be `(i32, i32)`. Slot functions may accept 0-12 parameters.
/// ```
/// use signals2::*;
///
/// let sig: Signal<(i32, i32)> = Signal::new();
/// sig.connect(|x, y| println!("x + y = {}", x + y));
/// sig.emit(2, 3); // prints "x + y = 5"
/// ```
/// Special care must be taken when creating `Signals` with slots that accept only one parameter. The single parameter type
/// must still be represented as a tuple in the type signature of the `Signal`. The Rust compiler does not recognize `(T)` as a tuple-type
/// of arity one, rather it recognizes it as simply the type `T`. A comma must be used to force the Rust compiler to recognize it as a tuple, e.x. `(T,)`
/// ```
/// use signals2::*;
///
/// let sig: Signal<(i32,)> = Signal::new(); // Note that using Signal<(i32)> or Signal<i32> will not compile!
/// sig.connect(|x| println!("x = {}", x));
/// sig.emit(7); // prints "x = 7"
/// ```
/// Slot functions can have return values, and the return value of the entire `Signal` is determined by the [Combiner] type.
/// The default combiner simply returns an `Option<R>` with a value of `Some(x)` where `x` is the value returned by
/// the last slot executed, or `None` in the case that no slots were executed.
/// ```
/// use signals2::*;
///
/// let sig: Signal<(i32, i32), i32> = Signal::new();
/// assert_eq!(sig.emit(2, 3), None); // no slots have been added yet
/// sig.connect(|x, y| x + y);
/// assert_eq!(sig.emit(2, 3), Some(5));
/// ```
pub struct Signal<Args, R = (), C = DefaultCombiner, G = i32>
where 
    Args: Clone + 'static,
    R: 'static,
    C: Combiner<R> + 'static,
    G: Ord + Send + Sync + 'static
{
    core: Arc<RwLock<Arc<SignalCore<Args, R, C, G>>>>
}

impl<Args, R, C, G> Clone for Signal<Args, R, C, G>
where 
    Args: Clone + 'static,
    R: 'static,
    C: Combiner<R> + 'static,
    G: Ord + Send + Sync + 'static
{
    /// Clones the corresponding signal. Note that a `Signal` is really just a handle to
    /// an underlying collection of slots. Cloning a signal will result in two handles that
    /// "point" to the same slots.
    ///
    /// # Example
    /// ```
    /// use signals2::*;
    ///
    /// let sig1: Signal<()> = Signal::new();
    /// let sig2 = sig1.clone();
    /// sig1.connect(|| println!("Hello, world!")); // connect to the first signal
    /// sig2.emit(); // prints "Hello, world!" because sig1 and sig2 share the same set of slots.
    /// ```
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone()
        }
    }
}

impl<Args, R, C, G> Signal<Args, R, C, G>
where 
    Args: Clone + 'static,
    R: 'static,
    C: Combiner<R> + 'static,
    G: Ord + Send + Sync + 'static
{
    /// Creates a new signal with a corresponding [Combiner].
    pub fn new_with_combiner(combiner: C) -> Self {
        let core: SignalCore<Args, R, C, G> = SignalCore::new(combiner);
        Signal {
            core: Arc::new(RwLock::new(Arc::new(core)))
        }
    }

    /// Creates a [WeakSignal] that holds a weak reference to its underling slots.
    pub fn weak(&self) -> WeakSignal<Args, R, C, G> {
        WeakSignal {
            weak_core: Arc::downgrade(&self.core)
        }
    }

    /// Sets a new [Combiner] for the signal.
    pub fn set_combiner(&self, combiner: C) {
        let mut lock = self.core.write().unwrap();
        let mut new_core = (**lock).clone();
        new_core.set_combiner(combiner);
        *lock = Arc::new(new_core);
    }

    /// Disconnects all slots from the signal. Will cause any existing [Connections](Connection) to enter a
    /// "disconnected" state.
    pub fn clear(&self) {
        self.core.read().unwrap().disconnect_all();
        let mut lock = self.core.write().unwrap();
        let mut new_core = (**lock).clone();
        new_core.clear();
        *lock = Arc::new(new_core);
    }

    /// Returns the number of connected slots for the signal.
    pub fn count(&self) -> usize {
        self.core.read().unwrap().count()
    }
}

impl<Args, R, C, G> Signal<Args, R, C, G>
where 
    Args: Clone + 'static,
    R: 'static,
    C: Combiner<R> + Default + 'static,
    G: Ord + Send + Sync + 'static
{
    /// Creates a new signal with a [Combiner] created by calling `C::default()`.
    pub fn new() -> Self {
        Self::new_with_combiner(C::default())
    }
}

/// A weak reference to a signal's slots. Useful for allowing slots to maintain a persistant reference to their 
/// owning signal without causing a memory leak.
/// # Example
/// ```
/// use signals2::*;
///
/// let sig: Signal<()> = Signal::new();
/// let weak_sig = sig.weak();
/// sig.connect(move || {
///     // if we had captured a cloned sig here it would cause a memory leak.
///     // Signals maintain strong references to their slot functions, so a slot function
///     // should not maintain a strong reference to its own signal or else a memory leak
///     // will occur.
///     weak_sig.upgrade().unwrap().connect(|| println!("Hello, world!"));
/// });
/// 
/// sig.emit(); // prints nothing
/// sig.emit(); // prints "Hello, world!" once
/// sig.emit(); // prints "Hello, world!" twice
/// // etc...
/// ```
pub struct WeakSignal<Args, R = (), C = DefaultCombiner, G = i32>
where 
    Args: Clone + 'static,
    R: 'static,
    C: Combiner<R> + 'static,
    G: Ord + Send + Sync + 'static
{
    weak_core: Weak<RwLock<Arc<SignalCore<Args, R, C, G>>>>
}

impl<Args, R, C, G> Clone for WeakSignal<Args, R, C, G>
where 
    Args: Clone + 'static,
    R: 'static,
    C: Combiner<R> + 'static,
    G: Ord + Send + Sync + 'static
{
    fn clone(&self) -> Self {
        Self {
            weak_core: self.weak_core.clone()
        }
    }
}

impl<Args, R, C, G> WeakSignal<Args, R, C, G>
where 
    Args: Clone + 'static,
    R: 'static,
    C: Combiner<R> + 'static,
    G: Ord + Send + Sync + 'static
{
    /// Returns `Some(sig)` where `sig` is the singal that the weak signal was
    /// created from. If the original signal (and all other clones of it) have been
    /// dropped, returns `None`. 
    pub fn upgrade(&self) -> Option<Signal<Args, R, C, G>> {
        self.weak_core.upgrade().map(|core| Signal {core})
    }
}
