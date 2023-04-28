## [0.3.3](https://github.com/christiandaley/signals2/releases/tag/v0.3.3) - 2023-04-27
- Update documentation to say "inspired by" boost::signals2 rather than "based on"

## [0.3.2](https://github.com/christiandaley/signals2/releases/tag/v0.3.2) - 2022-01-18
- Added `Signal::default` function.

## [0.3.1](https://github.com/christiandaley/signals2/releases/tag/v0.3.1) - 2021-07-31
- Switched to using `BTreeSet` instead of `BTreeMap` to store the slots in the signal core

## [0.3.0](https://github.com/christiandaley/signals2/releases/tag/v0.3.0) - 2021-07-31
- Added `ConnectHandle` and `EmitHandle`
- Added changelog

## [0.2.1](https://github.com/christiandaley/signals2/releases/tag/v0.2.1) - 2021-07-31
- Removed `UntypedSignalCore` trait and reimplemented `Connection` and `SharedConnectionBlock`
- Replaced `Mutex` with `RwLock` for the signal core

## [0.2.0](https://github.com/christiandaley/signals2/releases/tag/v0.2.0) - 2021-07-30
- Used const generics to clean up the implementations of `Connection` and `ScopedConnection`
- Removed public `ConnectionInterface` trait
- Added `Send + Sync` constraints to the `Combiner` and `UntypedSignalCore` traits to cleanup the code

## [0.1.0](https://github.com/christiandaley/signals2/releases/tag/v0.1.0) - 2021-07-25
- Initial release