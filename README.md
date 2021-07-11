# About

`signals2` is a thread-safe signal/slot library based on the [boost::signals2](https://www.boost.org/doc/libs/1_76_0/doc/html/signals2.html) C++ library. Signals are objects that contain a list of callback functions ("slots") to be executed when the signal is "emitted". Signals and their corresponding slots can be managed through the use of connections and shared connection blocks.

`signals2` contains no unsafe code and compiles on stable Rust 1.53.

`signals2` is distributed under the [Boost Software License, Version 1.0](LICENSE.txt).

* [Basic usage](#basic-usage)
* [Advanced usage](#advanced-usage)
* [Concurrency](#concurrency)

# Basic usage
These basic use patterns should cover most usage scenarios. 

## Creating signals and connecting slots
The follwing example creates a signal with slot functions that take no arguments and do not return anything. The `()` argument given to the `Signal` template indicates the type of parameters that the slot functions will accept. In this case the slots do not accept any parameters.

    use signals2::*; 

    let sig: Signal<()> = Signal::new();
    sig.connect(|| println!("Hello, world!"));
    sig.emit(); // prints "Hello, world!"

As a more complex example we create a signal with slots that accept two arguments.

    let sig: Signal<(i32, i32)> = Signal::new();
    sig.connect(|x, y| println!("x + y = {}", x + y));
    sig.emit(2, 3); // prints "x + y = 5"

Special care must be taken when creating signals with slots that accept only one parameter. The single parameter type
must still be represented as a tuple in the type signature of the signal. The Rust compiler does not recognize `(T)` as a tuple-type
of arity one, rather it recognizes it as simply the type `T`. A comma must be used to force the Rust compiler to recognize it as a tuple, e.x. `(T,)`

    let sig: Signal<(i32,)> = Signal::new(); // Note that using Signal<(i32)> or Signal<i32> will not compile!
    sig.connect(|x| println!("x = {}", x));
    sig.emit(7); // prints "x = 7"

Slots can return values, and the return value of calling `emit` on a slot is determined by the signal's combiner. By default `emit` will simply return an `Option<R>` representing the value returned by the last slot that was executed. If no slots were executed `None` will be returned.

    let sig: Signal<(i32, i32), i32> = Signal::new();
    assert_eq!(sig.emit(2, 3), None); // no slots have been added yet
    sig.connect(|x, y| x + y);
    assert_eq!(sig.emit(2, 3), Some(5));

Cloning a signal will result in a new signal that "points" to the same underlying set of slots as the original signal. Any modifications made to the cloned signal will affect the original signal, and vice versa.

    let sig1: Signal<()> = Signal::new();
    let sig2 = sig1.clone();
    sig1.connect(|| println!("Hello, world!")); // connect to the first signal
    sig2.emit(); // prints "Hello, world!" because sig1 and sig2 share the same set of slots.

## Using connections to manage slots
Connections are used to manage the lifetime of a slot. When a new slot is connected to a signal a corresponding connection is created. A connection manages the lifetime of exactly one slot for some particular signal. Connections can be used to disconnect slots from their signals.

    let sig: Signal<()> = Signal::new();
    let conn = sig.connect(|| println!("Hello, world!"));
    conn.disconnect(); // disconnect the slot
    sig.emit(); // prints nothing, the slot has been disconnected

A single slot may have an arbitrary number of connections associated with it. More connections for a slot may be created by calling `Clone` on existing connections. Note that cloning a connection creates a new connection that "points" to the same slot. Disconnecting one connection to a slot will disconnect all other connections to that slot. Once a slot has been disconnected it is permanently removed from the signal and cannot be reconnected.

    let sig: Signal<()> = Signal::new();
    let conn1 = sig.connect(|| println!("Hello, world!"));
    let conn2 = conn1.clone();

    conn2.disconnect();
    assert!(conn1.connected() == false); // disconnecting conn2 has also disconnected conn1
    assert!(conn2.connected() == false);

    sig.emit(); // prints nothing, the slot has been disconnected.

Connections do not automatically disconnect when they are dropped. If you wish to have a connection that disconnects itself automatically when dropped, use a scoped connection. Regular connections can be converted into scoped connections by calling `scoped()` on them.

    let sig: Signal<()> = Signal::new();
    {
        let _conn = sig.connect(|| println!("Hello, world!")).scoped(); // create a scoped connection
        sig.emit(); // prints "Hello, world!"
    } // _conn is dropped and disconnects

    sig.emit(); // prints nothing, the slot has been disconnected.

## Weak signals
A slot function may need to have access to its own signal, for example in the case where a slot wishes to recursively emit its own signal or connect a new slot to the signal. To accomplish this, an initial instinct may be to clone the signal and then move the cloned signal into a closure that is then connected to the original signal as in the following example. **This will cause a memory leak.**

    let sig: Signal<()> = Signal::new();
    let sig_clone = sig.clone();
    sig.connect(move || { // memory leak!
        sig_clone.connect(|| println!("Hello, world!"));
    });

    sig.emit(); // prints nothing (see the "Concurrency" section for why nothing is printed here)
    sig.emit(); // prints "Hello, world!" once
    sig.emit(); // prints "Hello, world!" twice
    // etc...

Signals maintain ownership over their slots, so therefore a slot cannot also have ownership over its own signal. A weak signal can be used to break this circular dependency.

    let sig: Signal<()> = Signal::new();
    let weak_sig = sig.weak();
    sig.connect(move || { // no memory leak!
        weak_sig.upgrade().unwrap().connect(|| println!("Hello, world!"));
    });

    sig.emit(); // prints nothing
    sig.emit(); // prints "Hello, world!" once
    sig.emit(); // prints "Hello, world!" twice
    // etc...

# Advanced usage
Less common usage patterns.

## Shared connection blocks
A user may wish to temporarily block a slot from executing without permanently disconnecting the slot. Shared connection blocks can be used to accomplish this. There can be an arbitrary number of shared connection blocks for any particular slot. If any of the shared connection blocks are blocking the slot, that slot will not be executed when the signal is emitted.

    let sig: Signal<(), i32> = Signal::new();
    let conn = sig.connect(|| 4);
    assert_eq!(sig.emit(), Some(4));

    let blocker1 = conn.shared_block(true); // blocking
    let blocker2 = blocker1.clone(); // also blocking, since blocker1 is blocking

    assert_eq!(conn.blocker_count(), 2);
    assert_eq!(sig.emit(), None); // slot is blocked and will not execute 

    blocker1.unblock();
    assert_eq!(conn.blocker_count(), 1);
    assert_eq!(sig.emit(), None); // slot is still blocked

    blocker2.unblock();
    assert_eq!(conn.blocker_count(), 0);
    assert_eq!(sig.emit(), Some(4)); // no more active blockers

Shared connection blocks automatically unblock themselved when dropped.

    let sig: Signal<(), i32> = Signal::new();
    let conn = sig.connect(|| 4);
    assert_eq!(sig.emit(), Some(4));
    {
        let _blocker = conn.shared_block(true);
        assert_eq!(conn.blocker_count(), 1);
        assert_eq!(sig.emit(), None);
    }

    assert_eq!(conn.blocker_count(), 0);
    assert_eq!(sig.emit(), Some(4)); // blocker was dropped

## Custom combiners
The return value of calling `emit` on a signal is determined by the combiner type of the signal. The default combiner simply returns an `Option` that represents the return value of the last slot to be executed (`None` in the case where no slot was executed). A custom combiner can be created by implementing the `Combiner<R>` trait on some type. Note that consuming the iterator passed to the `combine()` function of the `Combiner` trait is what causes slots to execute. A slot is executed each time the `next` function is called on the iterator. 

    #[derive(Default)]
    struct ProductCombiner {}

    impl Combiner<i32> for ProductCombiner {
        type Output = i32;

        /// ProductCombiner computes the product of all values returned by the slots
        fn combine(&self, iter: impl Iterator<Item=i32>) -> Self::Output {
            iter.product()
        }
    }

    let sig: Signal<(), i32, ProductCombiner> = Signal::new();
    
    sig.connect(|| 2);
    sig.connect(|| 3);
    sig.connect(|| 4);

    assert_eq!(sig.emit(), 24);

## Controlling the order in which slots execute using groups and positions
Internally, a signal stores its slots in groups. Groups of slots are ordered, and groups with higher precedence are executed first. By default there exist two "unnamed" groups of slots. These groups are referred to as the "front" group and the "back group". The "front" group of slots will always be executed *before* all other groups of slots are executed. The "back" group of slots will always be executed *after* all other groups of slots are executed. Any number of "named" groups may be created, and they are executed according to their ordering. Named groups will always be executed after the "front" group and before the "back" group.

Additionally, when a slot is connected to a group it can be connected at either the `Front` or `Back` of the group. When a slot is connected to the front of a group it is guaranteed to execute *before* all other slots that were present in the group when it was connected. When a slot is connected to the back of a group it is guaranteed to execute *after* all other slots that were present in the group when it was connected.

The following example shows how to use the `Group` and `Position` enums to control the order in which slots are executed.

    let sig: Signal<()> = Signal::new();

    sig.connect_group_position(|| println!("World!"), Group::Front, Position::Front);
    sig.connect_group_position(|| print!("Hello,"), Group::Front, Position::Front);

    sig.emit(); // prints "Hello, world!"

A more complex example using named groups.

    let sig: Signal<()> = Signal::new();

    sig.connect_group_position(|| print!("lazy "), Group::Back, Position::Back);
    sig.connect_group_position(|| print!("brown "), Group::Named(5), Position::Back);
    sig.connect_group_position(|| print!("over "), Group::Named(7), Position::Back);
    sig.connect_group_position(|| print!("the "), Group::Front, Position::Back);
    sig.connect_group_position(|| print!("jumps "), Group::Named(7), Position::Front);
    sig.connect_group_position(|| println!("dog"), Group::Back, Position::Back);
    sig.connect_group_position(|| print!("the "), Group::Named(7), Position::Back);
    sig.connect_group_position(|| print!("quick "), Group::Named(-8), Position::Back);
    sig.connect_group_position(|| print!("fox "), Group::Named(5), Position::Back);

    sig.emit(); // prints "the quick brown fox jumps over the lazy dog"

Named groups are ordered according to the `Ord` trait implemented for their underlying type. By default their underlying type is `i32`. This can be changed to any other type that implements the `Ord` trait by simply specifying the type in the signal's type. For example, named groups may be identified by Strings and sorted lexographically.

    use signals2::*;
    use combiner::DefaultCombiner;

    let sig: Signal<(), (), DefaultCombiner, String> = Signal::new();

Examples in the basic usage section just use the `connect` function rather than `connect_position_group`. Using `connect(f)` is identical to using `connect_position_group(f, Group::Back, Position::Back)`. There is also a `connect_group(f, group)` function that allows only a group to be specified. It is equivalent to `connect_position_group(f, group, Position::Back)`. Likewise there is a `connect_position(f, position)` function that allows only a position to be specified. It is equivalent to `connect_position_group(f, Group::Back, position)`.

## Extended slots
In the basic usage section we saw how a slot can use a weak signal to maintain a persistant reference to its own signal. This is useful in the case where a slot function may need to recursivey emit its own signal or connect new slots to the signal. However, what if a slot needs to be able to disconnect/block itself? A connection is required to disconnect a signal. How can a slot access its own connection? An attempt such as the one below will fail.

    let sig: Signal<(i32,)> = Signal::new();
    let weak_sig = sig.weak();

    let conn = sig.connect(move |x| {
        if x == 5 {
            println!("Slot recursively called itself 5 times! Disconnecting now.");
            conn.disconnect(); // compiler error
        }

        weak_sig.upgrade().unwrap().emit(x + 1);
    });

    sig.emit(0);

The issue is that a slot function cannot caputure its own connection because its connection is not created until it is actually connected to the signal. This problem can be solved by using "extended slots". Extended slots are similar to regular slots except that their functions take one more parameter: a conncetion.

    let sig: Signal<(i32,)> = Signal::new();
    let weak_sig = sig.weak();

    sig.connect_extended(move |conn, x| { // the connection is passed in as the first parameter to the slot
        if x == 5 {
            println!("Slot recursively called itself 5 times! Disconnecting now.");
            conn.disconnect();
        }

        weak_sig.upgrade().unwrap().emit(x + 1);
    });

    sig.emit(0);

There are corresponding `connect_position_extended`, `connect_group_extended`, and `connect_position_group_extended` functions as well.

# Concurrency

Signals are thread safe and may be shared between threads (provided the slot functions, combiner, and group types are threadsafe). A signal may be emitted concurrently (i.e. two or more threads may emit the signal simultaneously). A signal may have a new slot connected to it while it is currently emitting. Neither of these scenarios will result in a dead-lock. The internal mutex of a signal will never deadlock regardless of how many different threads are using the signal or how many times it is recursively emittied. There is, however, some subtley when it comes to modifying a signal while it is emitting.

It is possible (and safe) to modify a signal while it is in the process of emitting. The question is: will modifications made to a signal while it is emitting be "visible" to the currently emitting slots? The answer is: it depends. Blocking/disconnecting a slot while a signal is emitting will be visible. The newly blocked/disconnected slot will not be executed (provided that the slot has not yet started executing). However, connecting a new slot to a signal or changing its combiner while it is emitting are changes that will not be visible to currently emitting slots. Consider the following example.

    let sig: Signal<()> = Signal::new();
    let weak_sig = sig.weak();
    sig.connect(move || {
        weak_sig.upgrade().unwrap().connect(|| println!("Hello, world!"));
    });

    sig.emit(); // prints nothing
    sig.emit(); // prints "Hello, world!" once
    sig.emit(); // prints "Hello, world!" twice
    // etc...

In this example, the first call to `emit` will not print anything even though a new slot is connected while the signal is emitting. The newly connected slot will not be executed until the next time the signal is emitted.