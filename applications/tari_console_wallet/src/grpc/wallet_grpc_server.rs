use tari_app_grpc::tari_rpc::{wallet_server, GetCoinbaseRequest, GetCoinbaseResponse};
use tonic::{Response, Status, Request};
use tari_wallet::Wallet;
use std::sync::{Arc};
use tokio::sync::RwLock;
use tari_wallet::storage::sqlite_db::WalletSqliteDatabase;
use tari_wallet::transaction_service::storage::sqlite_db::TransactionServiceSqliteDatabase;
use tari_wallet::output_manager_service::storage::sqlite_db::OutputManagerSqliteDatabase;
use tari_wallet::contacts_service::storage::sqlite_db::ContactsServiceSqliteDatabase;
use tari_core::transactions::transaction::Transaction;

pub struct WalletGrpcServer {
    wallet: Arc<RwLock< Wallet<
        WalletSqliteDatabase,
        TransactionServiceSqliteDatabase,
        OutputManagerSqliteDatabase,
        ContactsServiceSqliteDatabase,
    >>>
}

impl WalletGrpcServer{
    pub fn new(wallet: Arc<RwLock< Wallet<
        WalletSqliteDatabase,
        TransactionServiceSqliteDatabase,
        OutputManagerSqliteDatabase,
        ContactsServiceSqliteDatabase,
    >>>) -> Self{
        Self {
            wallet
        }
    }
}
#[tonic::async_trait]
impl wallet_server::Wallet for WalletGrpcServer {
    async fn get_coinbase(&self, request: Request<GetCoinbaseRequest>) -> Result<Response<GetCoinbaseResponse>, Status> {

        let request = request.into_inner();
        let response = self.wallet.write().await.transaction_service.generate_coinbase_transaction(request.reward.into(), request.fee.into(), request.height).await;
        match response {
            Ok(resp) => Ok(Response::new(GetCoinbaseResponse {
                transaction: Some(resp.into())
            })),
            Err(err) => {
                unimplemented!()
            }
        }
    }
}
