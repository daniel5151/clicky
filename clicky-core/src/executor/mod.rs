//! Types and Traits related to task execution.

use std::fmt::Debug;

use futures::task::Spawn;
pub use futures::task::SpawnExt;

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        pub type Executor = local::LocalExecutor;
        pub type Spawner = local::LocalSpawner;
    } else {
        pub type Executor = thread::ThreadExecutor;
        pub type Spawner = thread::ThreadSpawner;
    }
}

/// A cloneable spawn handle.
pub trait ClickySpawn: Spawn + Debug + Clone {}

/// Abstraction over single/multi threaded executors.
pub trait ClickyExecutor: Debug + Sized {
    type Spawner: ClickySpawn;

    /// Construct a new executor.
    fn new() -> std::io::Result<Self>;

    /// Runs all tasks in the pool and returns if no more progress can be made
    /// on any task.
    ///
    /// On multi-threaded executors, this method is a noop.
    fn run_until_stalled(&mut self);

    /// Return a cloneable spawn handle.
    fn spawner(&self) -> Self::Spawner;
}

mod local {
    use super::*;

    use futures::future::FutureObj;
    use futures::task::SpawnError;

    #[derive(Debug, Clone)]
    pub struct LocalSpawner(futures_executor::LocalSpawner);

    impl ClickySpawn for LocalSpawner {}
    impl Spawn for LocalSpawner {
        fn spawn_obj(&self, future: FutureObj<'static, ()>) -> Result<(), SpawnError> {
            self.0.spawn_obj(future)
        }

        #[inline]
        fn status(&self) -> Result<(), SpawnError> {
            self.0.status()
        }
    }

    #[derive(Debug)]
    pub struct LocalExecutor(futures_executor::LocalPool);

    impl ClickyExecutor for LocalExecutor {
        type Spawner = LocalSpawner;

        fn new() -> std::io::Result<Self> {
            Ok(LocalExecutor(futures_executor::LocalPool::new()))
        }

        fn run_until_stalled(&mut self) {
            futures_executor::LocalPool::run_until_stalled(&mut self.0)
        }

        fn spawner(&self) -> Self::Spawner {
            LocalSpawner(self.0.spawner())
        }
    }
}

mod thread {
    use super::*;

    use futures::future::FutureObj;
    use futures::task::SpawnError;

    #[derive(Debug, Clone)]
    pub struct ThreadSpawner(futures_executor::ThreadPool);

    impl ClickySpawn for ThreadSpawner {}
    impl Spawn for ThreadSpawner {
        fn spawn_obj(&self, future: FutureObj<'static, ()>) -> Result<(), SpawnError> {
            self.0.spawn_obj(future)
        }

        #[inline]
        fn status(&self) -> Result<(), SpawnError> {
            self.0.status()
        }
    }

    #[derive(Debug)]
    pub struct ThreadExecutor(futures_executor::ThreadPool);

    impl ClickyExecutor for ThreadExecutor {
        type Spawner = ThreadSpawner;

        fn new() -> std::io::Result<Self> {
            Ok(ThreadExecutor(futures_executor::ThreadPool::new()?))
        }

        fn run_until_stalled(&mut self) {}

        fn spawner(&self) -> Self::Spawner {
            ThreadSpawner(self.0.clone())
        }
    }
}
