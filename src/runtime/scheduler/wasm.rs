use futures::channel::mpsc::{channel, Sender};
use futures::future::Future;
use slab::Slab;
use wasm_rs_async_executor::single_threaded;

use crate::runtime::config;
use crate::runtime::run_block;
use crate::runtime::scheduler::Scheduler;
use crate::runtime::AsyncMessage;
use crate::runtime::Topology;

#[derive(Clone, Debug)]
pub struct WasmScheduler;

impl WasmScheduler {
    pub fn new() -> WasmScheduler {
        WasmScheduler
    }
}

impl Scheduler for WasmScheduler {
    fn run_topology(
        &self,
        topology: &mut Topology,
        main_channel: &Sender<AsyncMessage>,
    ) -> Slab<Option<Sender<AsyncMessage>>> {
        let mut inboxes = Slab::new();
        let max = topology.blocks.iter().map(|(i, _)| i).max().unwrap_or(0);
        for _ in 0..=max {
            inboxes.insert(None);
        }
        let queue_size = config::config().queue_size;

        // spawn block executors
        for (id, block_o) in topology.blocks.iter_mut() {
            let block = block_o.take().unwrap();

            let (sender, receiver) = channel::<AsyncMessage>(queue_size);
            inboxes[id] = Some(sender);

            if block.is_blocking() {
                self.spawn_blocking(run_block(block, id, main_channel.clone(), receiver));
            } else {
                self.spawn(run_block(block, id, main_channel.clone(), receiver));
            }
        }

        inboxes
    }

    fn spawn<T: Send + 'static>(
        &self,
        future: impl Future<Output = T> + Send + 'static,
    ) -> single_threaded::TaskHandle<T> {
        single_threaded::spawn(future)
    }

    fn spawn_blocking<T: Send + 'static>(
        &self,
        future: impl Future<Output = T> + Send + 'static,
    ) -> single_threaded::TaskHandle<T> {
        info!("no spawn blocking for wasm, using spawn");
        self.spawn(future)
    }
}

impl Default for WasmScheduler {
    fn default() -> Self {
        Self::new()
    }
}
