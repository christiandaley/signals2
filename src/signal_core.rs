// Copyright Christian Daley 2021
// Copyright Frank Mori Hess 2007-2008.
// Distributed under the Boost Software License, Version 1.0. 
// See http://www.boost.org/LICENSE_1_0.txt

use std::sync::{Arc, Weak, atomic::{AtomicUsize, AtomicIsize, AtomicBool, Ordering}};
use std::collections::BTreeSet;
use std::cmp;

use crate::combiner::Combiner;
use crate::connect::{Position, Group, Connection};

fn next_position(pos: &Position) -> isize {
    static POSITION_COUNTER: AtomicIsize = AtomicIsize::new(0);
    let sign = match pos {
        Position::Front => -1isize,
        Position::Back => 1isize
    };

    POSITION_COUNTER.fetch_add(1, Ordering::Relaxed) * sign
}

// A key used to indentify a slot. This tuple will implement Ord
// because both Group<G> and isize implement Ord.
type SlotKey<G> = (Group<G>, isize);

enum SlotFunc<Args, R> {
    Basic(Box<dyn Fn(Args) -> R + Send + Sync + 'static>),
    Extended((Box<dyn Fn(Connection, Args) -> R + Send + Sync + 'static>, Connection))
}

struct Slot<Args, R, G> 
where
    Args: 'static,
    R: 'static,
    G: Ord + Send + Sync + 'static
{
    func: SlotFunc<Args, R>,
    connected: Arc<AtomicBool>,
    blocker_count: Arc<AtomicUsize>,
    key: SlotKey<G>
}

impl<Args, R, G> PartialEq for Slot<Args, R, G> 
where 
    Args: 'static,
    R: 'static,
    G: Ord + Send + Sync + 'static
{
    fn eq(&self, other: &Self) -> bool {
        self.key.eq(&other.key)
    }
}

impl<Args, R, G> Eq for Slot<Args, R, G> 
where 
    Args: 'static,
    R: 'static,
    G: Ord + Send + Sync + 'static
{}

impl<Args, R, G> PartialOrd for Slot<Args, R, G> 
where 
    Args: 'static,
    R: 'static,
    G: Ord + Send + Sync + 'static
{
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(&other))
    }
}

impl<Args, R, G> Ord for Slot<Args, R, G> 
where 
    Args: 'static,
    R: 'static,
    G: Ord + Send + Sync + 'static
{
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.key.cmp(&other.key)
    }
}

impl<Args, R, G> Slot<Args, R, G> 
where 
    Args: 'static,
    R: 'static,
    G: Ord + Send + Sync + 'static
{
    fn emit(&self, args: Args) -> R {
        match &self.func {
            SlotFunc::Basic(f) => f(args),
            SlotFunc::Extended((f, conn)) => f(conn.clone(), args)
        }
    }

    fn connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    fn blocked(&self) -> bool {
        self.blocker_count.load(Ordering::SeqCst) != 0usize
    }

    fn disconnect(&self) {
        self.connected.store(false,  Ordering::SeqCst);
    }
}

pub struct SignalCore<Args, R, C, G> 
where 
    Args: Clone + 'static,
    R: 'static,
    C: Combiner<R> + 'static,
    G: Ord + Send + Sync + 'static
{
    slots: BTreeSet<Arc<Slot<Args, R, G>>>,
    combiner: Arc<C>
}

impl<Args, R, C, G> Clone for SignalCore<Args, R, C, G> 
where 
    Args: Clone + 'static,
    R: 'static,
    C: Combiner<R> + 'static,
    G: Ord + Send + Sync
{
    fn clone(&self) -> Self {
        Self {
            slots: self.slots.clone(),
            combiner: self.combiner.clone()
        }
    }
}

impl<Args, R, C, G> SignalCore<Args, R, C, G>
where 
    Args: Clone + 'static,
    R: 'static,
    C: Combiner<R> + 'static,
    G: Ord + Send + Sync + 'static
{
    pub fn new(combiner: C) -> Self {
        SignalCore {
            slots: BTreeSet::new(),
            combiner: Arc::new(combiner)
        }
    }

    pub fn emit(&self, args: &Args) -> C::Output {
        let iter = self.slots.iter().filter_map(
            |slot| {
                if slot.connected() && !slot.blocked() {
                    Some(slot.emit(args.clone()))
                } else {
                    None
                }
            }
        );

        self.combiner.combine(iter)
    }

    fn connect_impl(&mut self, slot_func: SlotFunc<Args, R>, group: Group<G>, pos: Position, connected: Arc<AtomicBool>, blocker_count: Arc<AtomicUsize>)
    {
        let new_slot: Slot<Args, R, G> = Slot {
            func: slot_func,
            connected: connected,
            blocker_count: blocker_count,
            key: (group, next_position(&pos))
        };

        self.slots.insert(Arc::new(new_slot));
    }

    pub fn connect<F>(&mut self, f: F, group: Group<G>, pos: Position, make_conn: impl FnOnce(Weak<AtomicBool>, Weak<AtomicUsize>) -> Connection) -> Connection
    where
        F: Fn(Args) -> R + Send + Sync + 'static
    {
        let connected = Arc::new(AtomicBool::new(true));
        let blocker_count = Arc::new(AtomicUsize::new(0usize));
        let conn =         make_conn(Arc::downgrade(&connected), Arc::downgrade(&blocker_count));

        self.connect_impl(SlotFunc::Basic(Box::new(f)), group, pos, connected, blocker_count);
        conn
    }

    pub fn connect_extended<F>(&mut self, f: F, group: Group<G>, pos: Position, make_conn: impl FnOnce(Weak<AtomicBool>, Weak<AtomicUsize>) -> Connection) -> Connection
    where
        F: Fn(Connection, Args) -> R + Send + Sync + 'static
    {
        let connected = Arc::new(AtomicBool::new(true));
        let blocker_count = Arc::new(AtomicUsize::new(0usize));
        let conn =         make_conn(Arc::downgrade(&connected), Arc::downgrade(&blocker_count));

        self.connect_impl(SlotFunc::Extended((Box::new(f), conn.clone())), group, pos, connected, blocker_count);
        conn
    }

    pub fn set_combiner(&mut self, combiner: C) {
        self.combiner = Arc::new(combiner);
    }

    pub fn disconnect_all(&self) {
        for slot in self.slots.iter() {
            slot.disconnect();
        }
    }

    pub fn clear(&mut self) {
        self.slots.clear();
    }

    pub fn cleanup(&mut self) {
        self.slots.retain(|slot| slot.connected());
    }

    pub fn count(&self) -> usize {
        self.slots.iter().filter(|slot| slot.connected()).count()
    }
}