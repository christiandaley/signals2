// Copyright Christian Daley 2021
// Copyright Frank Mori Hess 2007-2008.
// Distributed under the Boost Software License, Version 1.0. 
// See http://www.boost.org/LICENSE_1_0.txt

use signals2::*;
use combiner::{Combiner, VecCombiner, SumCombiner};
use std::thread;
use std::mem;
use std::time::Duration;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

#[test]
fn basic_signal_test() {
    let sig: Signal<(i32,), i32> = Signal::new();
    assert_eq!(sig.count(), 0);
    assert_eq!(sig.emit(0), None);

    sig.connect(|x| x + 1);
    assert_eq!(sig.count(), 1);
    assert_eq!(sig.emit(0), Some(1));

    sig.connect(|x| x + 2);
    assert_eq!(sig.count(), 2);
    assert_eq!(sig.emit(0), Some(2));

    sig.clear();
    assert_eq!(sig.count(), 0);
    assert_eq!(sig.emit(0), None);
}

#[test]
fn disconnect_test() {
    let counter = Arc::new (AtomicUsize::new(0usize));
    let counter_clone = counter.clone();

    let get_counter = || counter.load(Ordering::Relaxed);
    let inc_counter = move || { counter_clone.fetch_add(1, Ordering::Relaxed); };
    let reset_counter = || counter.store(0usize, Ordering::Relaxed);

    let sig: Signal<()> = Signal::new();
    {
        let conn1 = sig.connect(inc_counter.clone());
        let conn2 = sig.connect(inc_counter.clone());
        let conn2_copy = conn2.clone();
        let conn3 = sig.connect(inc_counter.clone()).scoped();

        assert!(conn1.connected());
        assert!(conn2.connected());
        assert!(conn2_copy.connected());
        assert!(conn3.connected());
        assert_eq!(sig.count(), 3usize);

        sig.emit();
        assert_eq!(get_counter(), 3usize);
        reset_counter();

        conn1.disconnect();
        assert!(!conn1.connected());
        assert!(conn2.connected());
        assert!(conn2_copy.connected());
        assert!(conn3.connected());
        assert_eq!(sig.count(), 2usize);

        sig.emit();
        assert_eq!(get_counter(), 2usize);
        reset_counter();

        conn2.disconnect();
        assert!(!conn1.connected());
        assert!(!conn2.connected());
        assert!(!conn2_copy.connected());
        assert!(conn3.connected());
        assert_eq!(sig.count(), 1usize);

        sig.emit();
        assert_eq!(get_counter(), 1usize);
        reset_counter();
    }

    assert_eq!(sig.count(), 0usize);

    sig.emit();
    assert_eq!(get_counter(), 0usize);
    reset_counter();
}

#[test]
fn signal_emitting_cloned_block_test() {
    // This test caught a bug with the clone implementation for SignalCore
    let sig: Signal<(), i32> = Signal::new();
    let sig_clone = sig.clone();

    sig.connect(|| {
        thread::sleep(Duration::from_millis(750));
        0
    });

    let conn = sig.connect(|| 1);

    let thread = thread::spawn(move || {
        sig_clone.emit()
    });

    thread::sleep(Duration::from_millis(250));
    sig.connect(|| 2);
    let _block = conn.shared_block(true);
    let res = thread.join().unwrap();
    assert_eq!(res, Some(0));
}

#[test]
fn connection_block_test() {
    let sig: Signal<()> = Signal::new();
    let conn = sig.connect(|| ());

    assert!(sig.emit().is_some());

    {
        let block1 = conn.shared_block(true);
        assert!(conn.blocked());
        assert_eq!(conn.blocker_count(), 1usize);
        assert!(block1.blocking());
        assert!(sig.emit().is_none());

        let block2 = conn.shared_block(false);
        assert!(conn.blocked());
        assert_eq!(conn.blocker_count(), 1usize);
        assert!(!block2.blocking());
        assert!(sig.emit().is_none());

        let block3 = block1.clone();
        assert!(conn.blocked());
        assert_eq!(conn.blocker_count(), 2usize);
        assert!(block3.blocking());
        assert!(sig.emit().is_none());

        let block4 = block2.clone();
        assert!(conn.blocked());
        assert_eq!(conn.blocker_count(), 2usize);
        assert!(!block4.blocking());
        assert!(sig.emit().is_none());

        block1.block();
        block2.block();
        block3.block();
        block4.block();

        assert!(conn.blocked());
        assert_eq!(conn.blocker_count(), 4usize);
        assert!(block1.blocking());
        assert!(block2.blocking());
        assert!(block3.blocking());
        assert!(block4.blocking());
        assert!(sig.emit().is_none());

        block1.unblock();
        block3.unblock();
        assert!(conn.blocked());
        assert_eq!(conn.blocker_count(), 2usize);
        assert!(!block1.blocking());
        assert!(block2.blocking());
        assert!(!block3.blocking());
        assert!(block4.blocking());
        assert!(sig.emit().is_none());

        block2.unblock();
        block4.unblock();
        assert!(!conn.blocked());
        assert_eq!(conn.blocker_count(), 0usize);
        assert!(!block1.blocking());
        assert!(!block2.blocking());
        assert!(!block3.blocking());
        assert!(!block4.blocking());
        assert!(sig.emit().is_some());

        block1.block();
        block2.block();
        block3.block();
        block4.block();
        assert!(conn.blocked());
        assert_eq!(conn.blocker_count(), 4usize);
        assert!(block1.blocking());
        assert!(block2.blocking());
        assert!(block3.blocking());
        assert!(block4.blocking());
        assert!(sig.emit().is_none());
    }

    let block = conn.shared_block(false);
    assert!(!conn.blocked());
    assert_eq!(conn.blocker_count(), 0usize);
    assert!(!block.blocking());
    assert!(sig.emit().is_some());

    mem::drop(sig);
    assert!(conn.blocked());
    assert_eq!(conn.blocker_count(), usize::MAX);
    assert!(!block.blocking());
}

#[test]
fn connect_while_emitting() {
    let sig: Signal<(), i32, SumCombiner> = Signal::new();
    let weak_sig = sig.weak();

    sig.connect(move|| {
        weak_sig.upgrade().unwrap().connect(|| 1);
        1
    });

    assert_eq!(sig.emit(), 1);
    assert_eq!(sig.emit(), 2);
    assert_eq!(sig.emit(), 3);
    assert_eq!(sig.emit(), 4);
}

#[test]
fn disconnect_while_emitting() {
    let sig: Signal<(i32,), i32> = Signal::new();
    let weak_sig = sig.weak();

    sig.connect_extended(move |conn, count| {
        if count == 100 {
            assert_eq!(weak_sig.upgrade().unwrap().count(), 1);
            conn.disconnect();
            assert_eq!(weak_sig.upgrade().unwrap().count(), 0);
        }

        1 + weak_sig.upgrade().unwrap().emit(count + 1).unwrap_or(0)
    });

    assert_eq!(sig.count(), 1);
    assert_eq!(sig.emit(0), Some(101));
    assert_eq!(sig.count(), 0);
}

#[test]
fn block_while_emitting() {
    let sig: Signal<(i32,), usize> = Signal::new();
    let weak_sig = sig.weak();

    let extended_slot = move |conn: Connection, count| {
        let blocker = conn.shared_block(false);

        if count == 100 {
            blocker.block();
        }

        1 + weak_sig.upgrade().unwrap().emit(count + 1).unwrap_or(0)
    };

    sig.connect_extended(extended_slot);

    assert_eq!(sig.emit(0), Some(101));
}

#[test]
fn position_group_order_test() {
    let sig: Signal<(), i32, VecCombiner> = Signal::new();

    let check_vec = |vec| {
        assert_eq!(sig.emit(), vec);
    };

    sig.connect(|| 0);
    check_vec(vec!(0));

    sig.connect_group(|| 1, Group::Front);
    check_vec(vec!(1, 0));

    sig.connect_group(|| 2, Group::Front);
    check_vec(vec!(1, 2, 0));

    sig.connect_group_position(|| 3, Group::Front, Position::Front);
    check_vec(vec!(3, 1, 2, 0));

    sig.connect_group_position(|| 4, Group::Front, Position::Front);
    check_vec(vec!(4, 3, 1, 2, 0));

    sig.connect_group_position(|| 5, Group::Front, Position::Back);
    check_vec(vec!(4, 3, 1, 2, 5, 0));

    sig.connect_group(|| 6, Group::Back);
    check_vec(vec!(4, 3, 1, 2, 5, 0, 6));

    sig.connect_group_position(|| 7, Group::Back, Position::Back);
    check_vec(vec!(4, 3, 1, 2, 5, 0, 6, 7));

    sig.connect_group_position(|| 8, Group::Back, Position::Front);
    check_vec(vec!(4, 3, 1, 2, 5, 8, 0, 6, 7));

    sig.connect_group(|| 9, Group::Named(0));
    check_vec(vec!(4, 3, 1, 2, 5, 9, 8, 0, 6, 7));

    sig.connect_group(|| 10, Group::Named(0));
    check_vec(vec!(4, 3, 1, 2, 5, 9, 10, 8, 0, 6, 7));

    sig.connect_group_position(|| 11, Group::Named(0), Position::Front);
    check_vec(vec!(4, 3, 1, 2, 5, 11, 9, 10, 8, 0, 6, 7));

    sig.connect_group(|| 12, Group::Named(-15));
    check_vec(vec!(4, 3, 1, 2, 5, 12, 11, 9, 10, 8, 0, 6, 7));

    sig.connect_group_position(|| 13, Group::Named(-15), Position::Front);
    check_vec(vec!(4, 3, 1, 2, 5, 13, 12, 11, 9, 10, 8, 0, 6, 7));

    sig.connect_group_position(|| 14, Group::Named(-15), Position::Back);
    check_vec(vec!(4, 3, 1, 2, 5, 13, 12, 14, 11, 9, 10, 8, 0, 6, 7));

    sig.connect_group(|| 15, Group::Named(-2));
    check_vec(vec!(4, 3, 1, 2, 5, 13, 12, 14, 15, 11, 9, 10, 8, 0, 6, 7));

    sig.connect_group(|| 16, Group::Named(3));
    check_vec(vec!(4, 3, 1, 2, 5, 13, 12, 14, 15, 11, 9, 10, 16, 8, 0, 6, 7));

    sig.connect_group_position(|| 17, Group::Named(3), Position::Front);
    check_vec(vec!(4, 3, 1, 2, 5, 13, 12, 14, 15, 11, 9, 10, 17, 16, 8, 0, 6, 7));

    sig.connect_position(|| 18, Position::Back);
    check_vec(vec!(4, 3, 1, 2, 5, 13, 12, 14, 15, 11, 9, 10, 17, 16, 8, 0, 6, 7, 18));

    sig.connect_position(|| 19, Position::Front);
    check_vec(vec!(4, 3, 1, 2, 5, 13, 12, 14, 15, 11, 9, 10, 17, 16, 19, 8, 0, 6, 7, 18));
}

#[test]
fn weak_signal_test() {
    let sig: Signal<()> = Signal::new();
    let weak_sig = sig.weak();

    assert!(weak_sig.upgrade ().is_some());
    mem::drop(sig);
    assert!(weak_sig.upgrade ().is_none());
}

#[test]
fn lazy_slots_test() {
    struct FirstNSlots {
        n: usize
    }

    impl Combiner<i32> for FirstNSlots {
        type Output = Vec<i32>;

        fn combine(&self, mut iter: impl Iterator<Item=i32>) -> Self::Output {
            let mut i = 0;
            let mut values: Vec<i32> = Vec::new();
            while i < self.n {
                match iter.next() {
                    Some(val) => values.push(val),
                    None => break
                }

                i += 1;
            }

            values
        }
    }

    let counter = Arc::new(AtomicUsize::new(0));
    let reset_counter = || counter.store(0, Ordering::Relaxed);

    let counter_clone1 = counter.clone();
    let counter_clone2 = counter.clone();
    let counter_clone3 = counter.clone();
    let counter_clone4 = counter.clone();
    let counter_clone5 = counter.clone();

    let sig: Signal<(), i32, FirstNSlots> = Signal::new_with_combiner(FirstNSlots {n: 0});
    sig.connect(move || { counter_clone1.fetch_add(1, Ordering::Relaxed); 0 });
    sig.connect(move || { counter_clone2.fetch_add(1, Ordering::Relaxed); 1 });
    sig.connect(move || { counter_clone3.fetch_add(1, Ordering::Relaxed); 2 });
    sig.connect(move || { counter_clone4.fetch_add(1, Ordering::Relaxed); 3 });
    sig.connect(move || { counter_clone5.fetch_add(1, Ordering::Relaxed); 4 });


    assert_eq!(sig.emit(), vec!());
    assert_eq!(counter.load(Ordering::Relaxed), 0);

    sig.set_combiner(FirstNSlots {n: 1});
    assert_eq!(sig.emit(), vec!(0));
    assert_eq!(counter.load(Ordering::Relaxed), 1);
    reset_counter();

    sig.set_combiner(FirstNSlots {n: 2});
    assert_eq!(sig.emit(), vec!(0, 1));
    assert_eq!(counter.load(Ordering::Relaxed), 2);
    reset_counter();

    sig.set_combiner(FirstNSlots {n: 3});
    assert_eq!(sig.emit(), vec!(0, 1, 2));
    assert_eq!(counter.load(Ordering::Relaxed), 3);
    reset_counter();

    sig.set_combiner(FirstNSlots {n: 4});
    assert_eq!(sig.emit(), vec!(0, 1, 2, 3));
    assert_eq!(counter.load(Ordering::Relaxed), 4);
    reset_counter();

    sig.set_combiner(FirstNSlots {n: 5});
    assert_eq!(sig.emit(), vec!(0, 1, 2, 3, 4));
    assert_eq!(counter.load(Ordering::Relaxed), 5);
    reset_counter();

    sig.set_combiner(FirstNSlots {n: 6});
    assert_eq!(sig.emit(), vec!(0, 1, 2, 3, 4));
    assert_eq!(counter.load(Ordering::Relaxed), 5);
}

#[test]
fn async_emit_test() {
    let sig: Signal<(), usize> = Signal::new();
    let sig_clone = sig.clone();
    let sig_clone2 = sig.clone();
    let sig_clone3 = sig.clone();
    
    let counter = AtomicUsize::new(0);

    sig.connect(move || counter.fetch_add(1, Ordering::AcqRel));

    let thread1 = thread::spawn(move || {
        for _ in 0..10000 {
            sig_clone.emit();
        }
    });

    let thread2 = thread::spawn(move || {
        for _ in 0..10000 {
            sig_clone2.emit();
        }
    });

    let thread3 = thread::spawn(move || {
        for _ in 0..10000 {
            sig_clone3.emit();
        }
    });

    for _ in 0..10000 {
        sig.emit();
    }

    thread1.join().unwrap();
    thread2.join().unwrap();
    thread3.join().unwrap();

    assert_eq!(sig.emit(), Some(40000));
}

#[test]
fn async_connect_test() {
    let sig: Signal<(), i32, SumCombiner> = Signal::new();
    let sig_clone1 = sig.clone();
    let sig_clone2 = sig.clone();

    sig.connect(|| {
        thread::sleep(Duration::from_millis(1000));
        1
    });

    let thread1 = thread::spawn(move || {
        thread::sleep(Duration::from_millis(100));
        for _ in 0..10 {
            sig_clone2.connect(|| 1);
        }
    });

    let thread2 = thread::spawn(move || {
        assert_eq!(sig_clone1.emit(), 1);
        assert_eq!(sig_clone1.emit(), 11);
    });

    thread2.join().unwrap();
    thread1.join().unwrap();

    assert_eq!(sig.emit(), 11);
}

#[test]
fn async_disconnect_test() {
    let sig: Signal<()> = Signal::new();
    let sig_clone = sig.clone();

    sig.connect(|| thread::sleep(Duration::from_millis(500)));

    let thread = thread::spawn(move || {
        thread::sleep(Duration::from_millis(100));
        sig_clone.clear();
    });

    assert!(sig.emit().is_some());
    thread.join().unwrap();
    assert!(sig.emit().is_none());
}

#[test]
fn recursive_connect_test() {
    let sig: Signal<(i32,), i32, SumCombiner> = Signal::new();

    let weak_sig = sig.weak();

    sig.connect_extended(move |conn, n| {
        let s = weak_sig.upgrade().unwrap();
        if n > 0 {
            s.connect(|_| 1);
        } else {
            conn.disconnect();
        }

        s.emit(n - 1)
    });

    assert_eq!(sig.emit(100), 5150);

    // Original conn is disconnected after the first computation, will always give number of connections now
    assert_eq!(sig.emit(0), 100);
    assert_eq!(sig.emit(-10), 100);
    assert_eq!(sig.emit(12000), 100);
    assert_eq!(sig.count(), 100);
}

#[test]
fn mutually_recursive_connect_test() {
    // https://oeis.org/A049778
    let sig1: Signal<(i32, i32), i32, SumCombiner> = Signal::new();
    let sig2: Signal<(i32, i32), i32, SumCombiner> = Signal::new();

    let weak_sig1 = sig1.weak();
    let weak_sig2 = sig2.weak();

    sig1.connect_extended(move |conn, count, max| {
        if count == max {
            conn.disconnect();
        }

        let sig = weak_sig2.upgrade().unwrap();
        sig.connect(|count, _| count);
        sig.emit(count + 1, max)
    });

    sig2.connect_extended(move |conn, count, max| {
        if count == max {
            conn.disconnect();
        }

        let sig = weak_sig1.upgrade().unwrap();
        sig.connect(|count, _| count);
        sig.emit(count + 1, max)
    });

    assert_eq!(sig1.emit(0, 39), 12131);
}

#[test]
fn connect_handle_test() {
    let sig: Signal<(), i32> = Signal::new();
    let connect_handle = sig.get_connect_handle();
    let conn = connect_handle.connect(|| 1);
    assert!(conn.connected());
    assert_eq!(sig.count(), 1);
    assert_eq!(sig.emit(), Some(1));

    mem::drop(sig);
    let conn = connect_handle.connect(|| 2);
    assert!(!conn.connected());
}

#[test]
fn emit_handle_test() {
    let sig: Signal<(f64, f64), f64> = Signal::new();
    let emit_handle = sig.get_emit_handle();
    assert_eq!(emit_handle.emit(1.0, 2.5), Some(None));

    sig.connect(|x, y| x - y);
    
    assert_eq!(emit_handle.emit(1.0, 2.5), Some(Some(-1.5)));

    mem::drop(sig);
    assert_eq!(emit_handle.emit(1.0, 2.5), None);
}

#[test]
fn default_test() {
    let sig: Signal<(), i32> = Signal::default();
    assert_eq!(sig.emit(), None);
    sig.connect(|| 5);
    assert_eq!(sig.emit(), Some(5));
}