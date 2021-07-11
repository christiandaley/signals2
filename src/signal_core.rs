// Copyright Christian Daley 2021
// Copyright Frank Mori Hess 2007-2008.
// Distributed under the Boost Software License, Version 1.0. 
// See http://www.boost.org/LICENSE_1_0.txt

use std::sync::{Mutex, Arc, atomic::{AtomicUsize, AtomicIsize, AtomicBool, Ordering}};
use std::collections::{BTreeMap, HashMap};

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

fn next_id() -> usize {
    static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);
    ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

// A key used to indentify a slot. This tuple will implement Ord
// because both Group<G> and isize implement Ord.
type SlotKey<G> = (Group<G>, isize);

enum SlotFunc<Args, R> {
    Basic(Box<dyn Fn(Args) -> R + Send + Sync + 'static>),
    Extended((Box<dyn Fn(Connection, Args) -> R + Send + Sync + 'static>, Connection))
}

struct Slot<Args, R> 
where
    Args: 'static,
    R: 'static,
{
    func: SlotFunc<Args, R>,
    connected: AtomicBool,
    blocker_count: AtomicUsize
}

impl<Args, R> Slot<Args, R> 
where 
    Args: 'static,
    R: 'static,
{
    fn emit(&self, args: Args) -> R {
        match &self.func {
            SlotFunc::Basic(f) => f(args),
            SlotFunc::Extended((f, conn)) => f(conn.clone(), args)
        }
    }

    fn connected(&self) -> bool {
        self.connected.load(Ordering::Acquire)
    }

    fn disconnect(&self) {
        self.connected.store(false,  Ordering::Release);
    }

    fn block(&self, block: bool) {
        let val = if block {1usize} else {usize::MAX};
        self.blocker_count.fetch_add(val, Ordering::AcqRel);
    }

    fn blocked(&self) -> bool {
        self.blocker_count() != 0usize
    }

    fn blocker_count(&self) -> usize {
        self.blocker_count.load(Ordering::Acquire)
    }
}

pub struct SignalCore<Args, R, C, G> 
where 
    Args: Clone + 'static,
    R: 'static,
    C: Combiner<R> + Send + Sync + 'static,
    G: Ord + Send + Sync
{
    slots: BTreeMap<Arc<SlotKey<G>>, Arc<Slot<Args, R>>>,
    ids: HashMap<usize, Arc<SlotKey<G>>>,
    combiner: Arc<C>
}

impl<Args, R, C, G> Clone for SignalCore<Args, R, C, G> 
where 
    Args: Clone + 'static,
    R: 'static,
    C: Combiner<R> + Send + Sync + 'static,
    G: Ord + Send + Sync
{
    fn clone(&self) -> Self {
        Self {
            slots: self.slots.clone(),
            ids: self.ids.clone(),
            combiner: self.combiner.clone()
        }
    }
}

impl<Args, R, C, G> SignalCore<Args, R, C, G>
where 
    Args: Clone + 'static,
    R: 'static,
    C: Combiner<R> + Send + Sync + 'static,
    G: Ord + Send + Sync
{
    fn slot_from_id(&self, slot_id: usize) -> Option<&Slot<Args, R>> {
        let key = self.ids.get(&slot_id)?;
        let slot = self.slots.get(key)?;
        Some(slot)
    }

    pub fn new(combiner: C) -> Self {
        SignalCore {
            slots: BTreeMap::new(),
            ids: HashMap::new(),
            combiner: Arc::new(combiner)
        }
    }

    pub fn emit(&self, args: &Args) -> C::Output {
        let iter = self.slots.iter().filter_map(
            |(_, slot)| {
                if slot.connected() && !slot.blocked() {
                    Some(slot.emit(args.clone()))
                } else {
                    None
                }
            }
        );

        self.combiner.combine(iter)
    }

    fn connect_impl(&mut self, slot_func: SlotFunc<Args, R>, group: Group<G>, pos: Position, id: usize)
    {
        let key = Arc::new((group, next_position(&pos)));
        let new_slot: Slot<Args, R> = Slot {
            func: slot_func,
            connected: AtomicBool::new(true),
            blocker_count: AtomicUsize::new(0usize),
        };
        self.slots.insert(key.clone(), Arc::new(new_slot));

        self.ids.insert(id, key);
    }

    pub fn connect<F>(&mut self, f: F, group: Group<G>, pos: Position, make_conn: impl FnOnce(usize) -> Connection) -> Connection
    where
        F: Fn(Args) -> R + Send + Sync + 'static
    {
        let id = next_id();
        self.connect_impl(SlotFunc::Basic(Box::new(f)), group, pos, id);
        make_conn(id)
    }

    pub fn connect_extended<F>(&mut self, f: F, group: Group<G>, pos: Position, make_conn: impl FnOnce(usize) -> Connection) -> Connection
    where
        F: Fn(Connection, Args) -> R + Send + Sync + 'static
    {
        let id = next_id();
        let conn = make_conn(id);
        self.connect_impl(SlotFunc::Extended((Box::new(f), conn.clone())), group, pos, id);
        conn
    }

    pub fn set_combiner(&mut self, combiner: C) {
        self.combiner = Arc::new(combiner);
    }

    pub fn disconnect_all(&self) {
        for (_, slot) in self.slots.iter() {
            slot.disconnect();
        }
    }

    pub fn clear(&mut self) {
        self.slots.clear();
        self.ids.clear();
    }

    pub fn count(&self) -> usize {
        self.slots.len()
    }

    fn remove_slot(&mut self, slot_id: usize) {
        if let Some(key) = self.ids.remove(&slot_id) {
            self.slots.remove(&key);
        }
    }
}

pub trait UntypedSignalCore {
    fn connected(&self, slot_id: usize) -> bool;

    fn disconnect(&self, slot_id: usize);

    fn block(&self, slot_id: usize, block: bool);

    fn blocked(&self, slot_id: usize) -> bool;

    fn blocker_count(&self, slot_id: usize) -> usize;
}

impl<Args, R, C, G> UntypedSignalCore for Mutex<Arc<SignalCore<Args, R, C, G>>>
where 
    Args: Clone + 'static,
    R: 'static,
    C: Combiner<R> + Send + Sync + 'static,
    G: Ord + Send + Sync
{
    fn connected(&self, slot_id: usize) -> bool {
        self.lock().unwrap().slot_from_id(slot_id)
            .map(|slot| slot.connected())
            .unwrap_or(false)
    }

    fn disconnect(&self, slot_id: usize) {
        let mut lock = self.lock().unwrap();
        if let Some(slot) = lock.slot_from_id(slot_id) {
            slot.disconnect();
            let mut new_core = (**lock).clone();
            new_core.remove_slot(slot_id);
            *lock = Arc::new(new_core);
        }
    }

    fn block(&self, slot_id: usize, block: bool) {
        if let Some(slot) = self.lock().unwrap().slot_from_id(slot_id) {
            slot.block(block);
        }
    }

    fn blocked(&self, slot_id: usize) -> bool {
        self.lock().unwrap().slot_from_id(slot_id)
            .map(|slot| slot.blocked())
            .unwrap_or(true)
    }

    fn blocker_count(&self, slot_id: usize) -> usize {
        self.lock().unwrap().slot_from_id(slot_id)
            .map(|slot| slot.blocker_count())
            .unwrap_or(usize::MAX)
    }
}