use crate::{prelude::*, sumeragi::Message, MessageSender};
use futures::{executor::ThreadPool, lock::Mutex};
use iroha_derive::log;
use iroha_network::prelude::*;
use std::{convert::TryFrom, sync::Arc};

pub struct Torii {
    url: String,
    pool_ref: ThreadPool,
    world_state_view: Arc<Mutex<WorldStateView>>,
    transaction_sender: Arc<Mutex<TransactionSender>>,
    message_sender: Arc<Mutex<MessageSender>>,
}

impl Torii {
    pub fn new(
        url: &str,
        pool_ref: ThreadPool,
        world_state_view: Arc<Mutex<WorldStateView>>,
        transaction_sender: TransactionSender,
        message_sender: MessageSender,
    ) -> Self {
        Torii {
            url: url.to_string(),
            world_state_view,
            pool_ref,
            transaction_sender: Arc::new(Mutex::new(transaction_sender)),
            message_sender: Arc::new(Mutex::new(message_sender)),
        }
    }

    pub async fn start(&mut self) {
        let url = &self.url.clone();
        let world_state_view = Arc::clone(&self.world_state_view);
        let transaction_sender = Arc::clone(&self.transaction_sender);
        let message_sender = Arc::clone(&self.message_sender);
        let state = ToriiState {
            pool: self.pool_ref.clone(),
            world_state_view,
            transaction_sender,
            message_sender,
        };
        Network::listen(Arc::new(Mutex::new(state)), url, handle_connection)
            .await
            .expect("Failed to start listening Torii.");
    }
}

struct ToriiState {
    pool: ThreadPool,
    world_state_view: Arc<Mutex<WorldStateView>>,
    transaction_sender: Arc<Mutex<TransactionSender>>,
    message_sender: Arc<Mutex<MessageSender>>,
}

async fn handle_connection(
    state: State<ToriiState>,
    stream: Box<dyn AsyncStream>,
) -> Result<(), String> {
    //TODO: Why network can't spawn new task?
    let state22 = Arc::clone(&state);
    state.lock().await.pool.spawn_ok(async move {
        Network::handle_message_async(state22, stream, handle_request)
            .await
            .expect("Failed to handle message.")
    });
    Ok(())
}

#[log]
async fn handle_request(state: State<ToriiState>, request: Request) -> Result<Response, String> {
    match request.url() {
        uri::INSTRUCTIONS_URI => match Transaction::try_from(request.payload().to_vec()) {
            Ok(transaction) => {
                state
                    .lock()
                    .await
                    .transaction_sender
                    .lock()
                    .await
                    .start_send(transaction.accept().expect("Failed to accept transaction."))
                    .map_err(|e| format!("{}", e))?;
                Ok(Response::empty_ok())
            }
            Err(e) => {
                eprintln!("Failed to decode transaction: {}", e);
                Ok(Response::InternalError)
            }
        },
        uri::QUERY_URI => match QueryRequest::try_from(request.payload().to_vec()) {
            Ok(request) => match request
                .query
                .execute(&*state.lock().await.world_state_view.lock().await)
            {
                Ok(result) => {
                    let result = &result;
                    Ok(Response::Ok(result.into()))
                }
                Err(e) => {
                    eprintln!("{}", e);
                    Ok(Response::InternalError)
                }
            },
            Err(e) => {
                eprintln!("Failed to decode transaction: {}", e);
                Ok(Response::InternalError)
            }
        },
        uri::BLOCKS_URI => match Message::try_from(request.payload().to_vec()) {
            Ok(message) => {
                state
                    .lock()
                    .await
                    .message_sender
                    .lock()
                    .await
                    .start_send(message)
                    .map_err(|e| format!("{}", e))?;
                Ok(Response::empty_ok())
            }
            Err(e) => {
                eprintln!("Failed to decode peer message: {}", e);
                Ok(Response::InternalError)
            }
        },
        non_supported_uri => panic!("URI not supported: {}.", &non_supported_uri),
    }
}

pub mod uri {
    pub const QUERY_URI: &str = "/query";
    pub const INSTRUCTIONS_URI: &str = "/instruction";
    pub const BLOCKS_URI: &str = "/block";
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Configuration;
    use async_std::task;
    use futures::channel::mpsc;
    use std::time::Duration;

    const CONFIGURATION_PATH: &str = "tests/test_config.json";

    #[async_std::test]
    async fn create_and_start_torii() {
        let config =
            Configuration::from_path(CONFIGURATION_PATH).expect("Failed to load configuration.");
        let torii_url = config.torii_url.to_string();
        let (tx_tx, _) = mpsc::unbounded();
        let (ms_tx, _) = mpsc::unbounded();
        let mut torii = Torii::new(
            &torii_url,
            ThreadPool::new().expect("Failed to build Thread Pool."),
            Arc::new(Mutex::new(WorldStateView::new(Peer::new(
                torii_url.clone(),
                &Vec::new(),
            )))),
            tx_tx,
            ms_tx,
        );
        task::spawn(async move {
            torii.start().await;
        });
        std::thread::sleep(Duration::from_millis(50));
    }
}
