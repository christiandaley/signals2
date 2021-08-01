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